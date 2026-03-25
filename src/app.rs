use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};
use anyhow::Result;
use crossterm::event::KeyCode;
use ::image::ImageBuffer;
use ::image::Rgba;
use serde::{Deserialize, Serialize};

use crate::audio;
use crate::config::Config;
use crate::parser::{self, DialogueCommand};
use crate::image;

const HISTORY_MAX: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
pub enum AppState {
    Menu,
    Settings,
    About,
    History,
    InDialogue {
        scene_id: String,
        cmd_index: usize,      // 改为命令索引
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
    pub scenes: HashMap<String, parser::SceneData>,  // 改为 scenes
    pub config: Config,
    pub portraits: HashMap<String, ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub logo: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub should_quit: bool,
    pub bgm_process: Option<Child>,
    pub voice_process: Option<Child>,
    pub history: VecDeque<(Option<String>, String)>,
    pub auto_play_timer: Option<Instant>,
    pub current_image: Option<String>,
    pub prev_state: Option<Box<AppState>>,
    pub title: String,           // 新增
    pub footer: String,          // 新增
}

impl App {
    pub fn new() -> Result<Self> {
        Self::ensure_directories()?;
        // 加载游戏配置（自动使用默认）
        let game_config = parser::load_game_config()?;
        // 加载剧情文件（自动使用默认）
        let dialogue_content = parser::load_dialogue()?;
        let scenes = parser::parse_dialogue_file(&dialogue_content)?;
        let config = Config::load()?;
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
    
        // 启动主菜单背景音乐
        let title_bgm_path = Path::new("assets/music/title.mp3");
        let bgm_process = if title_bgm_path.exists() {
            audio::play_audio(&title_bgm_path, true, config.bgm_volume).ok()
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
            scenes,
            config,
            portraits,
            logo,
            should_quit: false,
            bgm_process,
            voice_process: None,
            history: VecDeque::with_capacity(HISTORY_MAX),
            auto_play_timer: None,
            current_image: None,
            prev_state: None,
            title: game_config.title,
            footer: game_config.footer,
        })
    }

    fn ensure_directories() -> io::Result<()> {
        for dir in &[
            "assets",
            "assets/dialog",      // 新增剧情目录
            "assets/portraits",
            "assets/music",
            "assets/voices",
            "save",
        ] {
            if !Path::new(dir).exists() {
                fs::create_dir_all(dir)?;
            }
        }
        Ok(())
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

    pub fn add_to_history(&mut self, speaker: Option<&str>, text: &str) {
        let speaker_clone = speaker.map(|s| s.to_string());
        self.history.push_back((speaker_clone, text.to_string()));
        while self.history.len() > HISTORY_MAX {
            self.history.pop_front();
        }
    }

    pub fn execute_command(&mut self, cmd: DialogueCommand) {
        match cmd {
            DialogueCommand::Text { speaker, text, voice } => {
                if let Some(s) = &speaker {
                    self.add_to_history(Some(s), &text);
                } else {
                    self.add_to_history(None, &text);
                }
                if let Some(v) = voice {
                    self.play_voice_by_file(speaker.as_deref().unwrap_or(""), Some(&v));
                } else if let Some(s) = speaker {
                    self.play_voice_by_file(&s, None);
                }
            }
            DialogueCommand::Image { filename } => {
                self.current_image = filename;
                // 图片立即显示，无需等待回车
            }
            DialogueCommand::Music { filename } => {
                self.play_bgm(&filename);
            }
            DialogueCommand::MusicStop => {
                self.stop_bgm();
            }
            DialogueCommand::Choose { options } => {
                if let AppState::InDialogue { scene_id, .. } = &self.state {
                    self.state = AppState::InChoice {
                        scene_id: scene_id.clone(),
                        options,
                        selected: 0,
                    };
                }
            }
            DialogueCommand::Load { target } => {
                self.state = AppState::InDialogue {
                    scene_id: target,
                    cmd_index: 0,
                };
                // 执行新场景的第一个命令
                if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(first_cmd) = scene.commands.get(*cmd_index) {
                            self.execute_command(first_cmd.clone());
                        }
                    }
                }
                // 自动跳过非交互命令
                self.skip_non_interactive_commands();
            }
            DialogueCommand::End => {
                self.state = AppState::Menu;
                self.current_image = None;
                self.stop_bgm(); // 结束时停止音乐
            }
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
        let initial_scene = "welcome".to_string();
        if self.scenes.contains_key(&initial_scene) {
            let scene_id = initial_scene.clone();
            self.state = AppState::InDialogue {
                scene_id: scene_id.clone(),
                cmd_index: 0,
            };
            // 执行第一个命令
            if let Some(scene) = self.scenes.get(&scene_id) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            // 自动跳过非交互命令，直接显示第一个需要交互的对话
            self.skip_non_interactive_commands();
        } else {
            self.state = AppState::Menu;
            self.status_message = Some("未找到起始场景 welcome".to_string());
        }
        self.status_message = None;
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
                    // 读档后恢复当前显示的图片
                    if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                        if let Some(scene) = self.scenes.get(scene_id) {
                            if let Some(cmd) = scene.commands.get(*cmd_index) {
                                self.execute_command(cmd.clone());
                            }
                        }
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

    pub fn advance_dialogue(&mut self) {
        let (current_scene_id, current_cmd_index) = match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => (scene_id.clone(), *cmd_index),
            _ => return,
        };
    
        let scene = match self.scenes.get(&current_scene_id) {
            Some(s) => s,
            None => {
                self.state = AppState::Menu;
                return;
            }
        };
    
        let next_cmd_index = current_cmd_index + 1;
        if let Some(next_cmd) = scene.commands.get(next_cmd_index) {
            // 更新状态
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                cmd_index: next_cmd_index,
            };
            // 执行命令
            self.execute_command(next_cmd.clone());
            
            // 如果执行的是图片或音乐命令，继续推进到下一个非图片/音乐命令
            self.skip_non_interactive_commands();
        } else {
            // 没有更多命令，返回菜单
            self.state = AppState::Menu;
            self.current_image = None;
            self.stop_bgm();
        }
    }
    
    /// 跳过非交互命令（图片、音乐），直到遇到需要用户交互的命令
    /// 跳过非交互命令（图片、音乐），直到遇到需要用户交互的命令
    pub fn skip_non_interactive_commands(&mut self) {
        loop {
            match &self.state {
                AppState::InDialogue { scene_id, cmd_index } => {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(cmd) = scene.commands.get(*cmd_index) {
                            match cmd {
                                DialogueCommand::Image { .. } | 
                                DialogueCommand::Music { .. } |
                                DialogueCommand::MusicStop => {
                                    // 非交互命令，继续推进
                                    let next_index = cmd_index + 1;
                                    if let Some(next_cmd) = scene.commands.get(next_index) {
                                        self.state = AppState::InDialogue {
                                            scene_id: scene_id.clone(),
                                            cmd_index: next_index,
                                        };
                                        self.execute_command(next_cmd.clone());
                                        continue;
                                    } else {
                                        // 没有更多命令，返回菜单
                                        self.state = AppState::Menu;
                                        return;
                                    }
                                }
                                _ => break, // 遇到交互命令（Text 或 Choose），停止跳过
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    pub fn select_option(&mut self) {
        let (options, selected, _current_scene_id) = match &self.state {
            AppState::InChoice { options, selected, scene_id } => {
                (options.clone(), *selected, scene_id.clone())
            }
            _ => return,
        };
    
        if let Some((_, next_scene)) = options.get(selected) {
            self.state = AppState::InDialogue {
                scene_id: next_scene.clone(),
                cmd_index: 0,
            };
            // 执行新场景的第一个命令
            if let Some(scene) = self.scenes.get(next_scene) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            // 自动跳过非交互命令
            self.skip_non_interactive_commands();
        }
    }
    
    pub fn current_speaker(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    if let Some(DialogueCommand::Text { speaker, .. }) = scene.commands.get(*cmd_index) {
                        return speaker.clone();
                    }
                }
                None
            }
            _ => None,
        }
    }
    
    pub fn current_text(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    if let Some(DialogueCommand::Text { text, .. }) = scene.commands.get(*cmd_index) {
                        return Some(text.clone());
                    }
                }
                None
            }
            _ => None,
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

        match self.state {
            AppState::History => {
                match key {
                    KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('H') => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
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
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        self.prev_state = Some(Box::new(self.state.clone()));
                        self.state = AppState::History;
                    }
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
                        self.current_image = None;
                        self.auto_play_timer = None;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => self.save_game(),
                    KeyCode::Char('l') | KeyCode::Char('L') => self.load_game(),
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        self.prev_state = Some(Box::new(self.state.clone()));
                        self.state = AppState::History;
                    }
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
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        self.prev_state = Some(Box::new(self.state.clone()));
                        self.state = AppState::History;
                        return;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

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
                    self.current_image = None;
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