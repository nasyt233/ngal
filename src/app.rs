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
use crate::variables::Variables;
use crate::save::SaveData;
use crate::parser::ImageParams;

const HISTORY_MAX: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
pub enum AppState {
    Menu,
    Settings,
    About,
    History,
    SaveSlot,
    LoadSlot,
    Input {
            prompt: String,
            var_name: String,
        },
    InDialogue {
        scene_id: String,
        cmd_index: usize,
    },
    InChoice {
        scene_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
}

#[derive(Serialize, Deserialize, Clone)]
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
    pub scenes: HashMap<String, parser::SceneData>,
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
    pub title: String,
    pub footer: String,
    pub variables: Variables,
    pub input_buffer: String,
    pub current_background: Option<String>,
    pub current_image_params: Option<ImageParams>,
    pub image_cache: HashMap<String, ImageBuffer<Rgba<u8>, Vec<u8>>>,
}

impl App {
    pub fn new() -> Result<Self> {
        Self::ensure_directories()?;

        let game_config = parser::load_game_config()?;
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

        let logo_path = Path::new("assets/portraits/title.png");
        let logo = if logo_path.exists() {
            image::load_image(logo_path).ok()
        } else {
            None
        };

        
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
            variables: Variables::new(),
            input_buffer: String::new(),
            current_background: None,
            current_image_params: None,
            image_cache: HashMap::new(),
        })
    }

    fn ensure_directories() -> io::Result<()> {
        for dir in &[
            "assets",
            "assets/dialog",
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

    
    fn interpolate_text(&self, text: &str) -> String {
        self.variables.interpolate(text)
    }

    
    pub fn execute_command(&mut self, cmd: DialogueCommand) {
        match cmd {
            DialogueCommand::Text { speaker, text, voice } => {
                let interpolated = self.interpolate_text(&text);
                if let Some(s) = &speaker {
                    self.add_to_history(Some(s), &interpolated);
                } else {
                    self.add_to_history(None, &interpolated);
                }
                if let Some(v) = voice {
                    self.play_voice_by_file(speaker.as_deref().unwrap_or(""), Some(&v));
                } else if let Some(s) = speaker {
                    self.play_voice_by_file(&s, None);
                }
            }
            DialogueCommand::Image(params) => {
                self.current_image_params = Some(params);
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
                if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(first_cmd) = scene.commands.get(*cmd_index) {
                            self.execute_command(first_cmd.clone());
                        }
                    }
                }
                self.skip_non_interactive_commands();
            }
            DialogueCommand::End => {
                self.state = AppState::Menu;
                self.current_image = None;
                self.stop_bgm();
            }
            DialogueCommand::Input { prompt, var_name } => {
                
                self.prev_state = Some(Box::new(self.state.clone()));
                self.state = AppState::Input { prompt, var_name };
            }
            DialogueCommand::SetVar { name, value } => {
                let interpolated = self.interpolate_text(&value);
                self.variables.set(&name, &interpolated);
                self.advance_dialogue();
            }
            DialogueCommand::Background { filename } => {
                self.current_background = filename;
            }
        }
    }

    
    pub fn skip_non_interactive_commands(&mut self) {
        loop {
            match &self.state {
                AppState::InDialogue { scene_id, cmd_index } => {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(cmd) = scene.commands.get(*cmd_index) {
                            match cmd {
                                DialogueCommand::Image { .. } |
                                DialogueCommand::Music { .. } |
                                DialogueCommand::MusicStop |
                                DialogueCommand::SetVar { .. } => {
                                    let next_index = cmd_index + 1;
                                    if let Some(next_cmd) = scene.commands.get(next_index) {
                                        self.state = AppState::InDialogue {
                                            scene_id: scene_id.clone(),
                                            cmd_index: next_index,
                                        };
                                        self.execute_command(next_cmd.clone());
                                        continue;
                                    } else {
                                        self.state = AppState::Menu;
                                        return;
                                    }
                                }
                                _ => break,
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

    
    pub fn start_game(&mut self) {
        let initial_scene = "welcome".to_string();
        if self.scenes.contains_key(&initial_scene) {
            let scene_id = initial_scene.clone();
            self.state = AppState::InDialogue {
                scene_id: scene_id.clone(),
                cmd_index: 0,
            };
            if let Some(scene) = self.scenes.get(&scene_id) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            self.skip_non_interactive_commands();
        } else {
            self.state = AppState::Menu;
            self.status_message = Some("未找到起始场景 welcome".to_string());
        }
        self.status_message = None;
    }

    
    pub fn save_game_slot(&mut self, slot: usize) {
        
        let save_state = if let Some(prev) = &self.prev_state {
            prev.as_ref().clone()
        } else {
            
            self.state.clone()
        };
        
        if let Err(e) = SaveData::save(slot, &save_state, self.selected, &self.variables, self.current_image.clone()) {
            self.status_message = Some(format!("存档失败: {}", e));
        } else {
            self.status_message = Some(format!("已存档到槽位 {}", slot));
        }
        
        
        if let Some(prev) = self.prev_state.take() {
            self.state = *prev;
        } else {
            self.state = AppState::Menu;
        }
    }
    pub fn load_game_slot(&mut self, slot: usize) {
        match SaveData::load(slot) {
            Ok(data) => {
                
                self.state = data.state;
                self.selected = data.menu_selected;
                self.variables.deserialize(data.variables);
                self.current_image = data.current_image;
                self.status_message = Some(format!("从槽位 {} 读档成功", slot));
                
                self.prev_state = None;
                
                
                
            }
            Err(e) => {
                self.status_message = Some(format!("读档失败: {}", e));
                
                if let Some(prev) = self.prev_state.take() {
                    self.state = *prev;
                } else {
                    self.state = AppState::Menu;
                }
            }
        }
    }

    pub fn open_save_slot(&mut self) {
        
        self.prev_state = Some(Box::new(self.state.clone()));
        self.selected = 0;
        self.state = AppState::SaveSlot;
    }
    
    pub fn open_load_slot(&mut self) {
        self.prev_state = Some(Box::new(self.state.clone()));
        self.selected = 0;
        self.state = AppState::LoadSlot;
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
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                cmd_index: next_cmd_index,
            };
            self.execute_command(next_cmd.clone());
            self.skip_non_interactive_commands();
        } else {
            self.state = AppState::Menu;
            self.current_image = None;
            self.stop_bgm();
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
            if let Some(scene) = self.scenes.get(next_scene) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            self.skip_non_interactive_commands();
        }
    }

    
    pub fn handle_settings(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::BgmUp => {
                if self.config.bgm_volume <= 90 {
                    self.config.bgm_volume += 10;
                    self.apply_bgm_volume();
                    self.status_message = Some(format!("BGM音量: {}%", self.config.bgm_volume));
                }
            }
            SettingsAction::BgmDown => {
                if self.config.bgm_volume >= 10 {
                    self.config.bgm_volume -= 10;
                    self.apply_bgm_volume();
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
    }

    fn apply_bgm_volume(&mut self) {
        if self.bgm_process.is_some() {
            self.stop_bgm();
            let bgm_path = Path::new("assets/music/title.mp3");
            if bgm_path.exists() {
                let _ = audio::play_audio(&bgm_path, true, self.config.bgm_volume)
                    .map(|child| self.bgm_process = Some(child));
            }
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
                        return Some(self.interpolate_text(text));
                    }
                }
                None
            }
            _ => None,
        }
    }

    
    pub fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => {
                self.open_load_slot();
            }
            2 => self.state = AppState::About,
            3 => self.state = AppState::Settings,
            4 => self.should_quit = true,
            _ => {}
        }
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
            AppState::SaveSlot => {
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 { self.selected -= 1; }
                    }
                    KeyCode::Down => {
                        if self.selected < 9 { self.selected += 1; }
                    }
                    KeyCode::Enter => {
                        self.save_game_slot(self.selected + 1);
                    }
                    KeyCode::Esc => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let slot = c.to_digit(10).unwrap() as usize;
                        if slot >= 1 && slot <= 10 {
                            self.save_game_slot(slot);
                        }
                    }
                    _ => {}
                }
                return;
            }
            AppState::LoadSlot => {
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 { self.selected -= 1; }
                    }
                    KeyCode::Down => {
                        if self.selected < 9 { self.selected += 1; }
                    }
                    KeyCode::Enter => {
                        let slot = self.selected + 1;
                        if SaveData::exists(slot) {
                            self.load_game_slot(slot);
                            
                            return;
                        } else {
                            self.status_message = Some("该槽位无存档".to_string());
                        }
                    }
                    KeyCode::Esc => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let slot = c.to_digit(10).unwrap() as usize;
                        if slot >= 1 && slot <= 10 && SaveData::exists(slot) {
                            self.load_game_slot(slot);
                            return;
                        } else if slot >= 1 && slot <= 10 {
                            self.status_message = Some("该槽位无存档".to_string());
                        }
                    }
                    _ => {}
                }
                return;
            }
            AppState::Input { ref var_name, .. } => {
                match key {
                    KeyCode::Enter => {
                        let value = if self.input_buffer.is_empty() {
                            "玩家".to_string()
                        } else {
                            self.input_buffer.clone()
                        };
                        self.variables.set(var_name, &value);
                        
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                        self.input_buffer.clear();
                        
                        self.advance_dialogue();
                    }
                    KeyCode::Esc => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                        self.input_buffer.clear();
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                    }
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
                    KeyCode::Char('+') | KeyCode::Char('=') => self.handle_settings(SettingsAction::BgmUp),
                    KeyCode::Char('-') | KeyCode::Char('_') => self.handle_settings(SettingsAction::BgmDown),
                    KeyCode::Char('[') => self.handle_settings(SettingsAction::VoiceDown),
                    KeyCode::Char(']') => self.handle_settings(SettingsAction::VoiceUp),
                    KeyCode::Char('a') | KeyCode::Char('A') => self.handle_settings(SettingsAction::AutoPlayToggle),
                    KeyCode::Char('1') => self.handle_settings(SettingsAction::AutoPlaySpeedDown),
                    KeyCode::Char('2') => self.handle_settings(SettingsAction::AutoPlaySpeedUp),
                    KeyCode::Char('s') | KeyCode::Char('S') => self.handle_settings(SettingsAction::Save),
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
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.open_save_slot();
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        self.open_load_slot();
                    }
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
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.open_save_slot();
                        return;
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        self.open_load_slot();
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