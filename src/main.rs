use std::collections::HashMap;
use std::fs;
use std::io::{self, stdout};
use std::panic;
use std::path::Path;
use std::process::{Child, Command};
use anyhow::{anyhow, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use image::{ImageBuffer, ImageReader, Rgba};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};

// ---------- 默认对话与配置 ----------
const DEFAULT_DIALOGUE: &str = r#"{
  "title": "原神 VS 鸣朝",
  "footer": "按回车继续 | q 退出",
  "scenes": {
    "start": {
      "dialogue": [
        { "music": "music.mp3" },
        { "speaker": "NAS油条", "text": "本项目由Rust语言开发，按回车键继续。", "voice": "nas_intro.mp3" },
        { "speaker": "NAS油条", "text": "哪个游戏牛逼?", "voice": "gamenb.mp3" }
      ],
      "options": [
        { "text": "原神牛逼👍", "next_scene": "ysnb" },
        { "text": "鸣朝牛逼👍", "next_scene": "mcnb" }
      ]
    },
    "ysnb": {
      "dialogue": [
        { "speaker": "鸣朝", "text": "鸣朝牛逼😡", "voice": "mcnb.mp3" },
        { "speaker": "鸣朝", "text": "原神不牛逼🤓", "voice": "ys_no_nb.mp3" }
      ],
      "options": [
        { "text": "鸣朝牛逼", "next_scene": "hnb" }
      ]
    },
    "mcnb": {
      "dialogue": [
        { "speaker": "原神", "text": "原神牛逼🤓👍", "voice": "ysnb.mp3"},
        { "speaker": "原神", "text": "鸣朝不牛逼😡", "voice": "mc_no_nb.mp3" }
      ],
      "options": [
        { "text": "原神牛逼", "next_scene": "hnb" }
      ]
    },
    "hnb": {
      "dialogue": [
        { "speaker": "我", "text": "😋他们产的片才牛逼😋", "voice": "ysmcnb.mp3" },
        { "speaker": "NAS油条", "text": "游戏结束" }
      ],
      "options": []
    }
  },
  "initial_scene": "start"
}"#;

const DEFAULT_CONFIG: &str = r#"{
  "bgm_volume": 70,
  "voice_volume": 80,
  "version": "0.3.0"
}"#;

// ---------- 数据模型 ----------

#[derive(Debug, Clone, Deserialize)]
struct DialogueLine {
    speaker: Option<String>,
    text: Option<String>,
    voice: Option<String>,
    music: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SceneData {
    dialogue: Vec<DialogueLine>,
    options: Vec<OptionData>,
}

#[derive(Debug, Clone, Deserialize)]
struct OptionData {
    text: String,
    next_scene: String,
}

#[derive(Debug, Deserialize)]
struct DialogueDB {
    title: String,
    footer: String,
    scenes: HashMap<String, SceneData>,
    initial_scene: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    bgm_volume: u8,
    voice_volume: u8,
    version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bgm_volume: 70,
            voice_volume: 80,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

// ---------- 游戏状态 ----------

#[derive(Serialize, Deserialize, Clone)]
enum AppState {
    Menu,
    Settings,                    // 新增设置界面
    InDialogue {
        scene_id: String,
        line_index: usize,
    },
    InChoice {
        scene_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    state: AppState,
    menu_selected: usize,
}

// ---------- 辅助枚举 ----------
enum ChoiceAction {
    Select,
    Exit,
    Save,
    Load,
}

enum SettingsAction {
    BgmUp,
    BgmDown,
    VoiceUp,
    VoiceDown,
    Save,
}

// ---------- 图片绘制辅助 ----------

fn draw_portrait(
    frame: &mut Frame,
    area: Rect,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) {
    let (img_w, img_h) = img.dimensions();
    let area_w = area.width as usize;
    let area_h = area.height as usize;

    let target_px_w = area_w;
    let target_px_h = area_h * 2;

    let scale_w = target_px_w as f64 / img_w as f64;
    let scale_h = target_px_h as f64 / img_h as f64;
    let scale = scale_w.min(scale_h);
    if scale <= 0.0 {
        return;
    }

    let new_w = (img_w as f64 * scale) as u32;
    let new_h = (img_h as f64 * scale) as u32;
    if new_w == 0 || new_h == 0 {
        return;
    }

    let resized = image::imageops::resize(
        img,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let char_h = (new_h + 1) / 2;
    let offset_y = if char_h as usize > area_h {
        0
    } else {
        (area_h - char_h as usize) / 2
    };
    let offset_x = (area_w as i32 - new_w as i32) / 2;

    let buffer = frame.buffer_mut();
    for row in 0..area_h {
        let row_in_img = row as i32 - offset_y as i32;
        if row_in_img < 0 {
            continue;
        }
        let y_pixel_top = (row_in_img as usize) * 2;
        if y_pixel_top >= new_h as usize {
            continue;
        }
        let y_pixel_bottom = y_pixel_top + 1;
        let screen_row = (area.y + row as u16) as usize;

        for col in 0..area_w {
            let x_pixel = col as i32 - offset_x;
            if x_pixel < 0 || x_pixel as usize >= new_w as usize {
                continue;
            }
            let x_pixel = x_pixel as usize;
            let pixel_top = resized.get_pixel(x_pixel as u32, y_pixel_top as u32);
            let top_color = Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let bottom_color = if y_pixel_bottom < new_h as usize {
                let pixel_bottom = resized.get_pixel(x_pixel as u32, y_pixel_bottom as u32);
                Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2])
            } else {
                Color::Black
            };

            let cell = buffer.get_mut((area.x + col as u16) as u16, screen_row as u16);
            cell.set_char('▀')
                .set_fg(top_color)
                .set_bg(bottom_color);
        }
    }
}

// ---------- 主应用 ----------

struct App {
    state: AppState,
    menu_options: Vec<String>,
    selected: usize,
    status_message: Option<String>,
    db: DialogueDB,
    config: Config,
    portraits: HashMap<String, ImageBuffer<Rgba<u8>, Vec<u8>>>,
    logo: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    should_quit: bool,
    bgm_process: Option<Child>,
    voice_process: Option<Child>,
}

impl App {
    fn new() -> Result<Self> {
        Self::ensure_directories()?;
        let db_content = Self::ensure_dialogue_file()?;
        let db: DialogueDB = serde_json::from_str(&db_content)?;
        let config = Self::load_config()?;

        // 加载角色图片
        let mut portraits = HashMap::new();
        let portraits_dir = Path::new("assets/portraits");
        if portraits_dir.exists() {
            for entry in fs::read_dir(portraits_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(img) = Self::load_image(&path) {
                            portraits.insert(name.to_string(), img);
                        }
                    }
                }
            }
        }

        // 加载 logo
        let logo_path = Path::new("assets/portraits/title.png");
        let logo = if logo_path.exists() {
            Self::load_image(logo_path).ok()
        } else {
            None
        };

        // 启动默认背景音乐（如果有）
        let bgm_path = Path::new("assets/music/bgm.mp3");
        let bgm_process = if bgm_path.exists() {
            Self::play_audio(&bgm_path, true, config.bgm_volume).ok()
        } else {
            None
        };

        Ok(Self {
            state: AppState::Menu,
            menu_options: vec![
                "开始游戏".to_string(),
                "设置".to_string(),
                "存档".to_string(),
                "读档".to_string(),
                "退出".to_string(),
            ],
            selected: 0,
            status_message: None,
            db,
            config,
            portraits,
            logo,
            should_quit: false,
            bgm_process,
            voice_process: None,
        })
    }

    fn load_image(path: &Path) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let img = ImageReader::open(path)
            .map_err(|e| anyhow!("无法打开图片 {}: {}", path.display(), e))?
            .with_guessed_format()
            .map_err(|e| anyhow!("无法识别图片格式: {}", e))?
            .decode()
            .map_err(|e| anyhow!("解码图片失败: {}", e))?;
        Ok(img.to_rgba8())
    }

    fn ensure_directories() -> io::Result<()> {
        if !Path::new("assets").exists() {
            fs::create_dir("assets")?;
        }
        if !Path::new("assets/portraits").exists() {
            fs::create_dir("assets/portraits")?;
        }
        if !Path::new("assets/music").exists() {
            fs::create_dir("assets/music")?;
        }
        if !Path::new("assets/voices").exists() {
            fs::create_dir("assets/voices")?;
        }
        if !Path::new("save").exists() {
            fs::create_dir("save")?;
        }
        Ok(())
    }

    fn ensure_dialogue_file() -> io::Result<String> {
        let path = Path::new("assets/dialogue.json");
        if !path.exists() {
            fs::write(path, DEFAULT_DIALOGUE)?;
            Ok(DEFAULT_DIALOGUE.to_string())
        } else {
            fs::read_to_string(path)
        }
    }

    fn load_config() -> Result<Config> {
        let path = Path::new("assets/config.json");
        if !path.exists() {
            fs::write(path, DEFAULT_CONFIG)?;
            Ok(Config::default())
        } else {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        }
    }

    fn save_config(&self) -> io::Result<()> {
        let path = Path::new("assets/config.json");
        let json = serde_json::to_string_pretty(&self.config)?;
        fs::write(path, json)
    }

    fn play_audio(path: &Path, background: bool, volume: u8) -> Result<Child> {
        if !path.exists() {
            return Err(anyhow!("音频文件不存在: {}", path.display()));
        }
        let volume_str = format!("{}", volume);
        let mut cmd = Command::new("mpv");
        cmd.arg("--no-video")
            .arg("--really-quiet")
            .arg("--vo=null")
            .arg("--no-window-dragging")
            .arg("--no-input-default-bindings")
            .arg("--no-input-cursor")
            .arg(format!("--volume={}", volume_str))
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if background {
            cmd.arg("--loop=inf");
        }
        Ok(cmd.spawn()?)
    }

    fn stop_voice(&mut self) {
        if let Some(mut child) = self.voice_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn stop_bgm(&mut self) {
        if let Some(mut child) = self.bgm_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn play_bgm(&mut self, filename: &str) {
        self.stop_bgm();
        let music_path = Path::new("assets/music").join(filename);
        if music_path.exists() {
            if let Ok(child) = Self::play_audio(&music_path, true, self.config.bgm_volume) {
                self.bgm_process = Some(child);
            }
        }
    }

    fn play_voice_by_file(&mut self, speaker: &str, voice_filename: Option<&str>) {
        self.stop_voice();
        let filename = if let Some(name) = voice_filename {
            name.to_string()
        } else {
            format!("{}.mp3", speaker)
        };
        let voice_path = Path::new("assets/voices").join(&filename);
        if voice_path.exists() {
            if let Ok(child) = Self::play_audio(&voice_path, false, self.config.voice_volume) {
                self.voice_process = Some(child);
            }
        }
    }

    fn play_voice_for_line(&mut self, line: &DialogueLine) {
        if let (Some(speaker), Some(_text)) = (&line.speaker, &line.text) {
            self.play_voice_by_file(speaker, line.voice.as_deref());
        }
    }

    fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => self.open_settings(),
            2 => self.save_game(),
            3 => self.load_game(),
            4 => self.quit_game(),
            _ => {}
        }
    }

    fn open_settings(&mut self) {
        self.state = AppState::Settings;
        self.status_message = None;
    }

    fn quit_game(&mut self) {
        self.should_quit = true;
    }

    fn start_game(&mut self) {
        let initial_scene = self.db.initial_scene.clone();
        let (music_files, first_line) = if let Some(scene) = self.db.scenes.get(&initial_scene) {
            let mut music_files = Vec::new();
            let mut line_index = 0;
            while let Some(line) = scene.dialogue.get(line_index) {
                if let Some(music_file) = &line.music {
                    music_files.push(music_file.clone());
                    line_index += 1;
                } else {
                    break;
                }
            }
            let first_line = scene.dialogue.get(line_index).cloned();
            (music_files, first_line)
        } else {
            (Vec::new(), None)
        };

        for music_file in &music_files {
            self.play_bgm(music_file);
        }

        if let Some(first_line) = first_line {
            self.play_voice_for_line(&first_line);
        }

        self.state = AppState::InDialogue {
            scene_id: initial_scene,
            line_index: music_files.len(),
        };
        self.status_message = None;
    }

    fn save_game(&mut self) {
        let data = SaveData {
            state: self.state.clone(),
            menu_selected: self.selected,
        };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                let save_path = Path::new("save/save.json");
                if let Err(e) = fs::write(save_path, json) {
                    self.status_message = Some(format!("存档失败：{}", e));
                } else {
                    self.status_message = Some("存档成功".to_string());
                }
            }
            Err(e) => {
                self.status_message = Some(format!("序列化失败：{}", e));
            }
        }
    }

    fn load_game(&mut self) {
        let save_path = Path::new("save/save.json");
        match fs::read_to_string(save_path) {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(data) => {
                    self.state = data.state;
                    self.selected = data.menu_selected;
                    self.status_message = Some("读档成功".to_string());
                    let current_line = self.current_dialogue_line().cloned();
                    if let Some(line) = current_line {
                        self.play_voice_for_line(&line);
                    }
                }
                Err(e) => {
                    self.status_message = Some(format!("解析存档失败：{}", e));
                }
            },
            Err(e) => {
                self.status_message = Some(format!("读取存档失败：{}", e));
            }
        }
    }

    fn current_dialogue_line(&self) -> Option<&DialogueLine> {
        match &self.state {
            AppState::InDialogue { scene_id, line_index } => {
                if let Some(scene) = self.db.scenes.get(scene_id) {
                    scene.dialogue.get(*line_index)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn current_speaker(&self) -> Option<String> {
        self.current_dialogue_line()
            .and_then(|line| line.speaker.clone())
    }

    fn current_text(&self) -> Option<String> {
        self.current_dialogue_line()
            .and_then(|line| line.text.clone())
    }

    fn advance_dialogue(&mut self) {
        let (current_scene_id, current_line_index) = match &self.state {
            AppState::InDialogue { scene_id, line_index } => (scene_id.clone(), *line_index),
            _ => return,
        };

        let (next_index, music_files, next_line) = {
            let scene = match self.db.scenes.get(&current_scene_id) {
                Some(s) => s,
                None => {
                    self.state = AppState::Menu;
                    return;
                }
            };

            let mut next_index = current_line_index + 1;
            let mut music_files = Vec::new();

            while let Some(line) = scene.dialogue.get(next_index) {
                if let Some(music_file) = &line.music {
                    music_files.push(music_file.clone());
                    next_index += 1;
                } else {
                    break;
                }
            }

            let next_line = scene.dialogue.get(next_index).cloned();
            (next_index, music_files, next_line)
        };

        for music_file in &music_files {
            self.play_bgm(music_file);
        }

        if let Some(next_line) = next_line {
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                line_index: next_index,
            };
            self.play_voice_for_line(&next_line);
        } else {
            let scene = match self.db.scenes.get(&current_scene_id) {
                Some(s) => s,
                None => {
                    self.state = AppState::Menu;
                    return;
                }
            };
            if !scene.options.is_empty() {
                let options: Vec<(String, String)> = scene
                    .options
                    .iter()
                    .map(|opt| (opt.text.clone(), opt.next_scene.clone()))
                    .collect();
                self.stop_voice();
                self.state = AppState::InChoice {
                    scene_id: current_scene_id,
                    options,
                    selected: 0,
                };
            } else {
                self.stop_voice();
                self.state = AppState::Menu;
            }
        }
    }

    fn select_option(&mut self) {
        let (current_scene_id, selected_idx) = match &self.state {
            AppState::InChoice { scene_id, selected, .. } => (scene_id.clone(), *selected),
            _ => return,
        };

        let next_scene_id = {
            let scene = match self.db.scenes.get(&current_scene_id) {
                Some(s) => s,
                None => return,
            };
            if let Some(opt) = scene.options.get(selected_idx) {
                opt.next_scene.clone()
            } else {
                return;
            }
        };

        let (music_files, first_line) = {
            let next_scene = match self.db.scenes.get(&next_scene_id) {
                Some(s) => s,
                None => return,
            };
            let mut music_files = Vec::new();
            let mut line_index = 0;
            while let Some(line) = next_scene.dialogue.get(line_index) {
                if let Some(music_file) = &line.music {
                    music_files.push(music_file.clone());
                    line_index += 1;
                } else {
                    break;
                }
            }
            let first_line = next_scene.dialogue.get(line_index).cloned();
            (music_files, first_line)
        };

        for music_file in &music_files {
            self.play_bgm(music_file);
        }

        if let Some(first_line) = first_line {
            self.play_voice_for_line(&first_line);
        }

        self.state = AppState::InDialogue {
            scene_id: next_scene_id,
            line_index: music_files.len(),
        };
    }

    fn handle_settings(&mut self, action: SettingsAction) -> bool {
        match action {
            SettingsAction::BgmUp => {
                if self.config.bgm_volume <= 90 {
                    self.config.bgm_volume += 10;
                    // 重启背景音乐以应用新音量
                    if self.bgm_process.is_some() {
                        self.stop_bgm();
                        if let Some(bgm_path) = self.get_current_bgm_path() {
                            let _ = Self::play_audio(&bgm_path, true, self.config.bgm_volume)
                                .map(|child| self.bgm_process = Some(child));
                        }
                    }
                    self.status_message = Some(format!("BGM音量: {}%", self.config.bgm_volume));
                }
            }
            SettingsAction::BgmDown => {
                if self.config.bgm_volume >= 10 {
                    self.config.bgm_volume -= 10;
                    if self.bgm_process.is_some() {
                        self.stop_bgm();
                        if let Some(bgm_path) = self.get_current_bgm_path() {
                            let _ = Self::play_audio(&bgm_path, true, self.config.bgm_volume)
                                .map(|child| self.bgm_process = Some(child));
                        }
                    }
                    self.status_message = Some(format!("BGM音量: {}%", self.config.bgm_volume));
                }
            }
            SettingsAction::VoiceUp => {
                if self.config.voice_volume <= 90 {
                    self.config.voice_volume += 10;
                    self.status_message = Some(format!("语音音量: {}%", self.config.voice_volume));
                }
            }
            SettingsAction::VoiceDown => {
                if self.config.voice_volume >= 10 {
                    self.config.voice_volume -= 10;
                    self.status_message = Some(format!("语音音量: {}%", self.config.voice_volume));
                }
            }
            SettingsAction::Save => {
                if let Err(e) = self.save_config() {
                    self.status_message = Some(format!("保存配置失败: {}", e));
                } else {
                    self.status_message = Some("配置已保存".to_string());
                }
            }
        }
        false
    }

    fn get_current_bgm_path(&self) -> Option<std::path::PathBuf> {
        // 简化：返回默认背景音乐路径
        let bgm_path = Path::new("assets/music/bgm.mp3");
        if bgm_path.exists() {
            Some(bgm_path.to_path_buf())
        } else {
            None
        }
    }

    fn handle_event(&mut self, key: KeyCode) {
        self.status_message = None;

        match &mut self.state {
            AppState::Menu => {
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected < self.menu_options.len() - 1 {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.execute_menu();
                    }
                    _ => {}
                }
                return;
            }
            AppState::Settings => {
                match key {
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.handle_settings(SettingsAction::BgmUp);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        self.handle_settings(SettingsAction::BgmDown);
                    }
                    KeyCode::Char('[') => {
                        self.handle_settings(SettingsAction::VoiceDown);
                    }
                    KeyCode::Char(']') => {
                        self.handle_settings(SettingsAction::VoiceUp);
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.handle_settings(SettingsAction::Save);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.state = AppState::Menu;
                    }
                    _ => {}
                }
                return;
            }
            AppState::InDialogue { .. } => {
                match key {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        self.advance_dialogue();
                    }
                    KeyCode::Esc => {
                        self.stop_voice();
                        self.state = AppState::Menu;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.save_game();
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        self.load_game();
                    }
                    _ => {}
                }
                return;
            }
            AppState::InChoice { .. } => {}
        }

        let action = if let AppState::InChoice { options, selected, .. } = &mut self.state {
            let options_count = options.len();
            match key {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    None
                }
                KeyCode::Down => {
                    if *selected < options_count - 1 {
                        *selected += 1;
                    }
                    None
                }
                KeyCode::Enter => Some(ChoiceAction::Select),
                KeyCode::Esc => Some(ChoiceAction::Exit),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(ChoiceAction::Save),
                KeyCode::Char('l') | KeyCode::Char('L') => Some(ChoiceAction::Load),
                _ => None,
            }
        } else {
            None
        };

        if let Some(action) = action {
            match action {
                ChoiceAction::Select => self.select_option(),
                ChoiceAction::Exit => {
                    self.stop_voice();
                    self.state = AppState::Menu;
                }
                ChoiceAction::Save => self.save_game(),
                ChoiceAction::Load => self.load_game(),
            }
        }
    }
}

// ---------- UI ----------

const BOTTOM_HEIGHT: u16 = 8;

fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(BOTTOM_HEIGHT)),
            Constraint::Length(BOTTOM_HEIGHT),
        ])
        .split(area);

    render_top(frame, chunks[0], app);
    render_bottom(frame, chunks[1], app);
}

fn render_top(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double);
    frame.render_widget(block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        AppState::Menu => {
            let mut y_offset = 0;

            // 绘制 Logo
            if let Some(logo) = &app.logo {
                let logo_area = Rect {
                    x: inner_area.x,
                    y: inner_area.y,
                    width: inner_area.width,
                    height: 6.min(inner_area.height),
                };
                draw_portrait(frame, logo_area, logo);
                y_offset = 6;
            }

            let title = &app.db.title;
            let title_paragraph = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("✨", Style::default().fg(Color::Rgb(255, 255, 0))),
                    Span::raw(" "),
                    Span::styled(title, Style::default().fg(Color::Rgb(212, 112, 212)).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled("✨", Style::default().fg(Color::Rgb(255, 255, 0))),
                ]),
                Line::from(vec![
                    Span::styled("✦", Style::default().fg(Color::Rgb(200, 100, 255))),
                    Span::raw("   Genshin Impact   "),
                    Span::styled("✦", Style::default().fg(Color::Rgb(200, 100, 255))),
                ]),
                Line::from(vec![
                    Span::styled("★", Style::default().fg(Color::Rgb(255, 215, 0))),
                    Span::raw(" Terminal Edition "),
                    Span::styled("★", Style::default().fg(Color::Rgb(255, 215, 0))),
                ]),
            ])
            .alignment(Alignment::Center);

            let title_area = Rect {
                x: inner_area.x,
                y: inner_area.y + y_offset,
                width: inner_area.width,
                height: 4,
            };
            frame.render_widget(title_paragraph, title_area);

            // 菜单列表
            let items: Vec<ListItem> = app
                .menu_options
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    let style = if i == app.selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(200, 200, 200))
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = app.menu_options.len() as u16 * 2;
            let start_y = inner_area.y + y_offset + 4 + (inner_area.height.saturating_sub(y_offset + 4 + list_height)) / 2;
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(30)) / 2,
                y: start_y,
                width: 30.min(inner_area.width),
                height: list_height.min(inner_area.height.saturating_sub(y_offset + 4)),
            };
            frame.render_widget(list, list_area);

            // 右下角版本信息
            let version_text = format!("ngal v{}", app.config.version);
            let version_paragraph = Paragraph::new(version_text)
                .style(Style::default().fg(Color::Rgb(150, 150, 150)))
                .alignment(Alignment::Right);
            let version_area = Rect {
                x: inner_area.x + inner_area.width - 15,
                y: inner_area.y + inner_area.height - 1,
                width: 15,
                height: 1,
            };
            frame.render_widget(version_paragraph, version_area);
        }
        AppState::Settings => {
            // 设置界面
            let settings_text = vec![
                Line::from(vec![
                    Span::styled("⚙️ 音量设置", Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD))
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("BGM 音量: "),
                    Span::styled(format!("{}%", app.config.bgm_volume), Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  (+/- 调节)"),
                ]),
                Line::from(vec![
                    Span::raw("语音音量: "),
                    Span::styled(format!("{}%", app.config.voice_volume), Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  ([ ] 调节)"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("按 S 保存配置 | ESC 返回", Style::default().fg(Color::Rgb(150, 150, 150)))
                ]),
            ];
            let settings_paragraph = Paragraph::new(settings_text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::NONE));
            let settings_area = Rect {
                x: inner_area.x,
                y: inner_area.y + (inner_area.height.saturating_sub(6)) / 2,
                width: inner_area.width,
                height: 6,
            };
            frame.render_widget(settings_paragraph, settings_area);

            // 显示状态消息
            if let Some(msg) = &app.status_message {
                let msg_paragraph = Paragraph::new(msg.as_str())
                    .style(Style::default().fg(Color::Rgb(255, 255, 0)))
                    .alignment(Alignment::Center);
                let msg_area = Rect {
                    x: inner_area.x,
                    y: inner_area.y + inner_area.height - 3,
                    width: inner_area.width,
                    height: 1,
                };
                frame.render_widget(msg_paragraph, msg_area);
            }
        }
        AppState::InDialogue { .. } => {
            if let Some(line) = app.current_dialogue_line() {
                if let (Some(speaker), Some(_text)) = (&line.speaker, &line.text) {
                    if let Some(img) = app.portraits.get(speaker) {
                        draw_portrait(frame, inner_area, img);
                    } else {
                        let art = match speaker.as_str() {
                            "NAS油条" => "   🍳  NAS油条  🍳",
                            "鸣朝"    => "   ⚔️  鸣朝  ⚔️",
                            "原神"    => "   ✨  原神  ✨",
                            _ => "   （暂无立绘）",
                        };
                        let text = format!("{}\n\n{}", art, speaker);
                        let para = Paragraph::new(text)
                            .style(Style::default().fg(Color::Rgb(212, 112, 212)))
                            .alignment(Alignment::Center)
                            .wrap(Wrap { trim: true });
                        let para_area = Rect {
                            x: inner_area.x,
                            y: inner_area.y + (inner_area.height.saturating_sub(5)) / 2,
                            width: inner_area.width,
                            height: 5.min(inner_area.height),
                        };
                        frame.render_widget(para, para_area);
                    }
                }
            }
        }
        AppState::InChoice { options, selected, .. } => {
            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, (text, _))| {
                    let style = if i == *selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(200, 200, 200))
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().title("请选择：").borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = options.len() as u16 * 2;
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(40)) / 2,
                y: inner_area.y + (inner_area.height.saturating_sub(list_height)) / 2,
                width: 40.min(inner_area.width),
                height: list_height.min(inner_area.height),
            };
            frame.render_widget(list, list_area);
        }
    }
}

fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let name_area = chunks[0];
    let text_area = chunks[1];

    let (speaker, content, status) = match &app.state {
        AppState::Menu => (
            "系统".to_string(),
            format!("{} | q 退出", app.db.footer),
            app.status_message.as_deref(),
        ),
        AppState::Settings => (
            "设置".to_string(),
            "按 +/- 调节BGM音量，[ ] 调节语音音量，S 保存，ESC/q 返回".to_string(),
            app.status_message.as_deref(),
        ),
        AppState::InDialogue { .. } => (
            app.current_speaker().unwrap_or_else(|| "?".to_string()),
            app.current_text().unwrap_or_else(|| "".to_string()),
            app.status_message.as_deref(),
        ),
        AppState::InChoice { .. } => (
            app.current_speaker().unwrap_or_else(|| "系统".to_string()),
            "请选择一项：".to_string(),
            app.status_message.as_deref(),
        ),
    };

    let name_style = Style::default()
        .fg(Color::Rgb(255, 255, 255))
        .add_modifier(Modifier::BOLD);
    let name_paragraph = Paragraph::new(Line::from(Span::styled(speaker, name_style)))
        .alignment(Alignment::Left);
    frame.render_widget(name_paragraph, name_area);

    let display_text = if let Some(status_msg) = status {
        status_msg
    } else {
        content.as_str()
    };
    let text_style = if status.is_some() {
        Style::default().fg(Color::Rgb(255, 255, 0))
    } else {
        Style::default().fg(Color::Rgb(255, 255, 255))
    };

    let text_paragraph = Paragraph::new(display_text)
        .style(text_style)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                .border_type(ratatui::widgets::BorderType::Double),
        );
    frame.render_widget(text_paragraph, text_area);
}

// ---------- 主函数 ----------

fn main() -> anyhow::Result<()> {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().expect("初始化失败，请检查 assets/dialogue.json 和 assets/portraits");

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                _ => app.handle_event(key.code),
            }
        }

        if app.should_quit {
            break;
        }
    }

    app.stop_voice();
    app.stop_bgm();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}