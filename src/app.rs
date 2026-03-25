use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};
use anyhow::Result;
use crossterm::event::KeyCode;
use ::image::ImageBuffer;    // 绝对路径引用外部 crate
use ::image::Rgba;           // 绝对路径引用外部 crate
use serde::{Deserialize, Serialize};

use crate::audio;
use crate::config::Config;
use crate::dialogue::{DialogueDB, DialogueLine};
use crate::image;

// ---------- 默认剧情（嵌入代码）----------
const DEFAULT_DIALOGUE: &str = r#"{
  "title": "原神 VS 鸣朝",
  "footer": "按回车继续 | q 返回主菜单 | H 历史 | A 自动播放",
  "scenes": {
    "start": {
      "dialogue": [
        { "music": "music.mp3" },
        "NAS油条:本项目由Rust语言开发，按回车键继续。:nas_intro.mp3",
        "NAS油条:哪个游戏牛逼?:gamenb.mp3"
      ],
      "options": [
        { "text": "原神牛逼👍", "next_scene": "ysnb" },
        { "text": "鸣朝牛逼👍", "next_scene": "mcnb" }
      ]
    },
    "ysnb": {
      "dialogue": [
        "鸣朝:鸣朝才牛逼😡:mcnb.mp3",
        "鸣朝:原神不牛逼🤓:ys_no_nb.mp3"
      ],
      "options": [
        { "text": "鸣朝牛逼", "next_scene": "hnb" }
      ]
    },
    "mcnb": {
      "dialogue": [
        "原神:原神才牛逼🤓👍:ysnb.mp3",
        "原神:鸣朝不牛逼😡:mc_no_nb.mp3"
      ],
      "options": [
        { "text": "原神牛逼", "next_scene": "hnb" }
      ]
    },
    "hnb": {
      "dialogue": [
        "我:😋他们产的片才牛逼😋:ysmcnb.mp3",
        "NAS油条:游戏结束"
      ],
      "options": []
    }
  },
  "initial_scene": "start"
}"#;

const HISTORY_MAX: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
pub enum AppState {
    Menu,
    Settings,
    About,
    History,
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

pub enum ChoiceAction {
    Select,
    Exit,
    Save,
    Load,
}

pub enum SettingsAction {
    BgmUp,
    BgmDown,
    VoiceUp,
    VoiceDown,
    AutoPlayToggle,
    AutoPlaySpeedUp,
    AutoPlaySpeedDown,
    Save,
}

pub struct App {
    pub state: AppState,
    pub menu_options: Vec<String>,
    pub selected: usize,
    pub status_message: Option<String>,
    pub db: DialogueDB,
    pub config: Config,
    pub portraits: HashMap<String, ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub logo: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub should_quit: bool,
    pub bgm_process: Option<Child>,
    pub voice_process: Option<Child>,
    pub history: VecDeque<(Option<String>, String)>,
    pub auto_play_timer: Option<Instant>,
}

impl App {
    pub fn new() -> Result<Self> {
        Self::ensure_directories()?;
        let db_content = Self::ensure_dialogue_file()?;
        let db: DialogueDB = serde_json::from_str(&db_content)?;
        let config = Config::load()?;

        // 加载角色图片
        let mut portraits = HashMap::new();
        let portraits_dir = Path::new("assets/portraits");
        if portraits_dir.exists() {
            for entry in fs::read_dir(portraits_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(img) = image::load_image(&path) {
                            portraits.insert(name.to_string(), img);
                        }
                    }
                }
            }
        }

        // 加载 logo
        let logo_path = Path::new("assets/portraits/title.png");
        let logo = if logo_path.exists() {
            image::load_image(logo_path).ok()
        } else {
            None
        };

        // 启动默认背景音乐
        let bgm_path = Path::new("assets/music/bgm.mp3");
        let bgm_process = if bgm_path.exists() {
            audio::play_audio(&bgm_path, true, config.bgm_volume).ok()
        } else {
            None
        };

        Ok(Self {
            state: AppState::Menu,
            menu_options: vec![
                "开始游戏".to_string(),
                "加载游戏".to_string(),
                "关于我们".to_string(),
                "游戏设置".to_string(),
                "退出游戏".to_string(),
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
            history: VecDeque::with_capacity(HISTORY_MAX),
            auto_play_timer: None,
        })
    }

    fn ensure_directories() -> io::Result<()> {
        for dir in &["assets", "assets/portraits", "assets/music", "assets/voices", "save"] {
            if !Path::new(dir).exists() {
                fs::create_dir(dir)?;
            }
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

    pub fn play_bgm(&mut self, filename: &str) {
        self.stop_bgm();
        let music_path = Path::new("assets/music").join(filename);
        if music_path.exists() {
            if let Ok(child) = audio::play_audio(&music_path, true, self.config.bgm_volume) {
                self.bgm_process = Some(child);
            }
        }
    }

    pub fn stop_bgm(&mut self) {
        if let Some(mut child) = self.bgm_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn play_voice_by_file(&mut self, speaker: &str, voice_filename: Option<&str>) {
        self.stop_voice();
        let filename = if let Some(name) = voice_filename {
            name.to_string()
        } else {
            format!("{}.mp3", speaker)
        };
        let voice_path = Path::new("assets/voices").join(&filename);
        if voice_path.exists() {
            if let Ok(child) = audio::play_audio(&voice_path, false, self.config.voice_volume) {
                self.voice_process = Some(child);
            }
        }
    }

    pub fn stop_voice(&mut self) {
        if let Some(mut child) = self.voice_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn play_voice_for_line(&mut self, line: &DialogueLine) {
        if let (Some(speaker), Some(_text)) = (&line.speaker, &line.text) {
            self.play_voice_by_file(speaker, line.voice.as_deref());
        }
    }

    pub fn add_to_history(&mut self, speaker: Option<&str>, text: &str) {
        let speaker_clone = speaker.map(|s| s.to_string());
        self.history.push_back((speaker_clone, text.to_string()));
        while self.history.len() > HISTORY_MAX {
            self.history.pop_front();
        }
    }

    pub fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => self.load_game(),
            2 => self.state = AppState::About,
            3 => self.state = AppState::Settings,
            4 => self.should_quit = true,
            _ => {}
        }
    }

    pub fn start_game(&mut self) {
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
            if let (Some(speaker), Some(text)) = (&first_line.speaker, &first_line.text) {
                self.add_to_history(Some(speaker), text);
            } else if let Some(text) = &first_line.text {
                self.add_to_history(None, text);
            }
            self.play_voice_for_line(&first_line);
        }

        self.state = AppState::InDialogue {
            scene_id: initial_scene,
            line_index: music_files.len(),
        };
        self.status_message = None;
        if self.config.auto_play {
            self.auto_play_timer = Some(Instant::now());
        } else {
            self.auto_play_timer = None;
        }
    }

    pub fn save_game(&mut self) {
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

    pub fn load_game(&mut self) {
        let save_path = Path::new("save/save.json");
        match fs::read_to_string(save_path) {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(data) => {
                    self.state = data.state;
                    self.selected = data.menu_selected;
                    self.status_message = Some("读档成功".to_string());
                    let current_line = self.current_dialogue_line().cloned();
                    if let Some(line) = current_line {
                        if let (Some(speaker), Some(text)) = (&line.speaker, &line.text) {
                            self.add_to_history(Some(speaker), text);
                        } else if let Some(text) = &line.text {
                            self.add_to_history(None, text);
                        }
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

    pub fn current_dialogue_line(&self) -> Option<&DialogueLine> {
        match &self.state {
            AppState::InDialogue { scene_id, line_index } => {
                self.db.scenes.get(scene_id)?.dialogue.get(*line_index)
            }
            _ => None,
        }
    }

    pub fn current_speaker(&self) -> Option<String> {
        self.current_dialogue_line()?.speaker.clone()
    }

    pub fn current_text(&self) -> Option<String> {
        self.current_dialogue_line()?.text.clone()
    }

    pub fn advance_dialogue(&mut self) {
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
            if let (Some(speaker), Some(text)) = (&next_line.speaker, &next_line.text) {
                self.add_to_history(Some(speaker), text);
            } else if let Some(text) = &next_line.text {
                self.add_to_history(None, text);
            }
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                line_index: next_index,
            };
            self.play_voice_for_line(&next_line);
            if self.config.auto_play {
                self.auto_play_timer = Some(Instant::now());
            }
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
            self.auto_play_timer = None;
        }
    }

    pub fn select_option(&mut self) {
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
            if let (Some(speaker), Some(text)) = (&first_line.speaker, &first_line.text) {
                self.add_to_history(Some(speaker), text);
            } else if let Some(text) = &first_line.text {
                self.add_to_history(None, text);
            }
            self.play_voice_for_line(&first_line);
        }

        self.state = AppState::InDialogue {
            scene_id: next_scene_id,
            line_index: music_files.len(),
        };
        if self.config.auto_play {
            self.auto_play_timer = Some(Instant::now());
        }
    }

    pub fn handle_settings(&mut self, action: SettingsAction) -> bool {
        match action {
            SettingsAction::BgmUp => {
                if self.config.bgm_volume <= 90 {
                    self.config.bgm_volume += 10;
                    if self.bgm_process.is_some() {
                        self.stop_bgm();
                        let bgm_path = Path::new("assets/music/bgm.mp3");
                        if bgm_path.exists() {
                            let _ = audio::play_audio(&bgm_path, true, self.config.bgm_volume)
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
                        let bgm_path = Path::new("assets/music/bgm.mp3");
                        if bgm_path.exists() {
                            let _ = audio::play_audio(&bgm_path, true, self.config.bgm_volume)
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
            SettingsAction::AutoPlayToggle => {
                self.config.auto_play = !self.config.auto_play;
                if self.config.auto_play {
                    self.auto_play_timer = Some(Instant::now());
                    self.status_message = Some("自动播放开启".to_string());
                } else {
                    self.auto_play_timer = None;
                    self.status_message = Some("自动播放关闭".to_string());
                }
            }
            SettingsAction::AutoPlaySpeedUp => {
                let new_speed = (self.config.auto_play_speed + 0.5).min(5.0);
                self.config.auto_play_speed = new_speed;
                self.status_message = Some(format!("自动播放速度: {:.1}秒", new_speed));
            }
            SettingsAction::AutoPlaySpeedDown => {
                let new_speed = (self.config.auto_play_speed - 0.5).max(0.5);
                self.config.auto_play_speed = new_speed;
                self.status_message = Some(format!("自动播放速度: {:.1}秒", new_speed));
            }
            SettingsAction::Save => {
                if let Err(e) = self.config.save() {
                    self.status_message = Some(format!("保存配置失败: {}", e));
                } else {
                    self.status_message = Some("配置已保存".to_string());
                }
            }
        }
        false
    }

    pub fn handle_event(&mut self, key: KeyCode) {
        self.status_message = None;

        // 处理弹窗
        match self.state {
            AppState::History => {
                match key {
                    KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('H') => self.state = AppState::Menu,
                    _ => {}
                }
                return;
            }
            AppState::About => {
                match key {
                    KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Menu,
                    _ => {}
                }
                return;
            }
            _ => {}
        }

        // 正常游戏状态
        match &mut self.state {
            AppState::Menu => {
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 { self.selected -= 1; }
                    }
                    KeyCode::Down => {
                        if self.selected < self.menu_options.len() - 1 { self.selected += 1; }
                    }
                    KeyCode::Enter => self.execute_menu(),
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('h') | KeyCode::Char('H') => self.state = AppState::History,
                    _ => {}
                }
                return;
            }
            AppState::Settings => {
                match key {
                    KeyCode::Char('+') | KeyCode::Char('=') => { self.handle_settings(SettingsAction::BgmUp); }
                    KeyCode::Char('-') | KeyCode::Char('_') => { self.handle_settings(SettingsAction::BgmDown); }
                    KeyCode::Char('[') => { self.handle_settings(SettingsAction::VoiceDown); }
                    KeyCode::Char(']') => { self.handle_settings(SettingsAction::VoiceUp); }
                    KeyCode::Char('a') | KeyCode::Char('A') => { self.handle_settings(SettingsAction::AutoPlayToggle); }
                    KeyCode::Char('1') => { self.handle_settings(SettingsAction::AutoPlaySpeedDown); }
                    KeyCode::Char('2') => { self.handle_settings(SettingsAction::AutoPlaySpeedUp); }
                    KeyCode::Char('s') | KeyCode::Char('S') => { self.handle_settings(SettingsAction::Save); }
                    KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Menu,
                    _ => {}
                }
                return;
            }
            AppState::InDialogue { .. } => {
                match key {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        self.advance_dialogue();
                        if self.config.auto_play {
                            self.auto_play_timer = Some(Instant::now());
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.stop_voice();
                        self.state = AppState::Menu;
                        self.auto_play_timer = None;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => self.save_game(),
                    KeyCode::Char('l') | KeyCode::Char('L') => self.load_game(),
                    KeyCode::Char('h') | KeyCode::Char('H') => self.state = AppState::History,
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.config.auto_play = !self.config.auto_play;
                        if self.config.auto_play {
                            self.auto_play_timer = Some(Instant::now());
                            self.status_message = Some("自动播放开启".to_string());
                        } else {
                            self.auto_play_timer = None;
                            self.status_message = Some("自动播放关闭".to_string());
                        }
                    }
                    _ => {}
                }
                return;
            }
            AppState::InChoice { .. } => {
                match key {
                    KeyCode::Char('h') | KeyCode::Char('H') => self.state = AppState::History,
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.stop_voice();
                        self.state = AppState::Menu;
                        self.auto_play_timer = None;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // 处理选项界面的上下选择和确认
        if let AppState::InChoice { options, selected, .. } = &mut self.state {
            let options_count = options.len();
            match key {
                KeyCode::Up => {
                    if *selected > 0 { *selected -= 1; }
                }
                KeyCode::Down => {
                    if *selected < options_count - 1 { *selected += 1; }
                }
                KeyCode::Enter => self.select_option(),
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.stop_voice();
                    self.state = AppState::Menu;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => self.save_game(),
                KeyCode::Char('l') | KeyCode::Char('L') => self.load_game(),
                _ => {}
            }
        }
    }

    pub fn update_auto_play(&mut self) {
        if self.config.auto_play {
            if let Some(timer) = self.auto_play_timer {
                if timer.elapsed() >= Duration::from_secs_f64(self.config.auto_play_speed) {
                    match self.state {
                        AppState::InDialogue { .. } => {
                            self.advance_dialogue();
                            self.auto_play_timer = Some(Instant::now());
                        }
                        _ => self.auto_play_timer = None,
                    }
                }
            }
        }
    }
}