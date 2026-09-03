use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};
use anyhow::Result;
use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::audio;
use crate::config::Config;
use crate::parser::{self, DialogueCommand, ImageParams};
use crate::variables::Variables;
use crate::save::SaveData;
use crate::image;

const HISTORY_MAX: usize = 50;

#[derive(Serialize, Deserialize, Clone)]
pub enum AppState {
    Menu,
    Settings,
    About,
    History,
    GameMenu,
    SaveSlot,
    LoadSlot,
    Input { prompt: String, var_name: String },
    InDialogue {
        scene_id: String,
        cmd_index: usize,
    },
    InChoice {
        scene_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
    EndOfFile,
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
    TextAnimationToggle,
    TextSpeedUp,
    TextSpeedDown,
    BgColorNext,
    Save,
}

pub struct App {
    pub state: AppState,
    pub menu_options: Vec<String>,
    pub selected: usize,
    pub status_message: Option<String>,
    pub scenes: HashMap<String, parser::SceneData>,
    pub config: Config,
    pub portraits: HashMap<String, crate::image::RgbaImage>,
    pub logo: Option<crate::image::RgbaImage>,
    pub should_quit: bool,
    pub bgm_process: Option<Child>,
    pub voice_process: Option<Child>,
    pub history: VecDeque<(Option<String>, String)>,
    pub auto_play_timer: Option<Instant>,
    pub prev_state: Option<Box<AppState>>,
    pub title: String,
    pub footer: String,
    pub variables: Variables,
    pub input_buffer: String,
    pub current_background: Option<String>,
    pub current_image_params: Option<ImageParams>,
    pub image_cache: HashMap<String, crate::image::RgbaImage>,
    pub menu_image: Option<crate::image::RgbaImage>,
    pub target_text: String,
    pub display_text: String,
    pub last_char_time: Instant,
    pub current_file: Option<String>,
    pub current_bgm: Option<String>,
    pub file_scene_order: HashMap<String, Vec<String>>,
}

impl App {
    pub fn new() -> Result<Self> {
        Self::ensure_directories()?;

        let game_config = parser::load_game_config()?;
        let dialogue_content = parser::load_dialogue()?;
        let (scenes, order) = parser::parse_dialogue_file_with_order(&dialogue_content)?;

        let config = Config::load()?;

        let image_cache = HashMap::new();
        let portraits = HashMap::new();

        let logo = if let Some(logo_file) = &game_config.logo {
            let logo_path = Path::new("assets/portraits").join(logo_file);
            image::load_image_rgba(&logo_path).ok()
        } else {
            None
        };

        let menu_image = if let Some(ref path) = game_config.menu_image {
            let img_path = Path::new("assets/portraits").join(path);
            image::load_image_rgba(&img_path).ok()
        } else {
            None
        };

        let title_bgm_path = if let Some(bgm_file) = &game_config.bgm {
            Path::new("assets/music").join(bgm_file)
        } else {
            Path::new("assets/music/title.mp3").to_path_buf()
        };
        let bgm_process = if title_bgm_path.exists() {
            audio::play_audio(&title_bgm_path, true, config.bgm_volume).ok()
        } else {
            None
        };

        let mut file_scene_order = HashMap::new();
        file_scene_order.insert("dialogue.ng".to_string(), order);

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
            prev_state: None,
            title: game_config.title,
            footer: game_config.footer,
            variables: Variables::new(),
            input_buffer: String::new(),
            current_background: None,
            current_image_params: None,
            image_cache,
            target_text: String::new(),
            display_text: String::new(),
            last_char_time: Instant::now(),
            menu_image,
            current_file: None,
            current_bgm: None,
            file_scene_order,
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

    pub fn play_title_bgm(&mut self) {
        let bgm_path = Path::new("assets/music/title.mp3");
        if bgm_path.exists() {
            self.stop_bgm();
            if let Ok(child) = audio::play_audio(&bgm_path, true, self.config.bgm_volume) {
                self.bgm_process = Some(child);
                self.current_bgm = Some("title.mp3".to_string());
            }
        }
    }

    pub fn play_bgm(&mut self, filename: &str) {
        self.stop_bgm();
        let music_path = Path::new("assets/music").join(filename);
        if music_path.exists() {
            if let Ok(child) = audio::play_audio(&music_path, true, self.config.bgm_volume) {
                self.bgm_process = Some(child);
                self.current_bgm = Some(filename.to_string());
            }
        }
    }

    pub fn stop_bgm(&mut self) {
        if let Some(mut child) = self.bgm_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.current_bgm = None;
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
        let mut result = self.variables.interpolate(text);
        let re = regex::Regex::new(r"\$\(([^)]+)\)").unwrap();
        while let Some(caps) = re.captures(&result) {
            let full_match = caps.get(0).unwrap().as_str();
            let cmd = caps.get(1).unwrap().as_str().trim();
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output();
            let replacement = match output {
                Ok(out) => {
                    if out.status.success() {
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    } else {
                        format!("[命令执行失败: {}]", cmd)
                    }
                }
                Err(e) => format!("[命令错误: {}]", e),
            };
            result = result.replace(full_match, &replacement);
        }
        result
    }

    pub fn update_animation(&mut self) {
        if !self.config.text_animation {
            if self.display_text != self.target_text {
                self.display_text = self.target_text.clone();
            }
            return;
        }

        if self.target_text.is_empty() {
            return;
        }

        if self.display_text.len() < self.target_text.len() {
            let elapsed = self.last_char_time.elapsed();
            let speed = Duration::from_millis(self.config.text_speed);
            if elapsed >= speed {
                let add_count = (elapsed.as_millis() / speed.as_millis()).max(1) as usize;
                for _ in 0..add_count {
                    let current_len = self.display_text.len();
                    if current_len < self.target_text.len() {
                        let next_char = self.target_text[current_len..].chars().next();
                        if let Some(c) = next_char {
                            let char_len = c.len_utf8();
                            self.display_text
                                .push_str(&self.target_text[current_len..current_len + char_len]);
                        } else {
                            break;
                        }
                    }
                }
                self.last_char_time = Instant::now();
            }
        }
    }

    pub fn execute_command(&mut self, cmd: DialogueCommand) {
        match cmd {
            DialogueCommand::Text {
                speaker,
                text,
                voice,
            } => {
                let interpolated = self.interpolate_text(&text);
                self.target_text = interpolated.clone();
                self.display_text = String::new();
                self.last_char_time = Instant::now();
                if !self.target_text.is_empty() {
                    let first_char = self.target_text.chars().next().unwrap();
                    let char_len = first_char.len_utf8();
                    self.display_text.push_str(&self.target_text[0..char_len]);
                }

                let interpolated_speaker = speaker.as_ref().map(|s| self.variables.interpolate(s));
                let interpolated_text = self.variables.interpolate(&text);
                let final_speaker = interpolated_speaker.as_deref();

                if let Some(s) = final_speaker {
                    self.add_to_history(Some(s), &interpolated_text);
                } else {
                    self.add_to_history(None, &interpolated_text);
                }

                if let Some(v) = voice {
                    self.play_voice_by_file(final_speaker.unwrap_or(""), Some(&v));
                } else if let Some(s) = final_speaker {
                    self.play_voice_by_file(s, None);
                }
            }
            DialogueCommand::Image(params) => {
                self.current_image_params = Some(params);
            }
            DialogueCommand::Music { filename } => {
                self.current_bgm = Some(filename.clone());
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
            DialogueCommand::Load { file, target } => {
                // 处理外部文件加载
                if let Some(file_name) = file {
                    // 过滤无效文件名
                    if file_name == "null" || file_name.is_empty() {
                        // 不改变 current_file
                    } else {
                        self.current_file = Some(file_name.clone());
                        if !self.file_scene_order.contains_key(&file_name) {
                            let _ = self.load_external_file(&file_name);
                        }
                        if !self.scenes.contains_key(&target) {
                            let _ = self.load_external_file(&file_name);
                        }
                    }
                } else {
                    // 同文件跳转：保留 current_file（不做任何修改）
                    // 如果当前 current_file 为 None，则保持 None
                }
            
                self.state = AppState::InDialogue {
                    scene_id: target,
                    cmd_index: 0,
                };
                // 执行第一个命令
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
                self.current_image_params = None;
                self.current_background = None;
                self.current_file = None;
                self.stop_bgm();
                self.play_title_bgm();
            }
            DialogueCommand::Input { prompt, var_name } => {
                self.prev_state = Some(Box::new(self.state.clone()));
                self.state = AppState::Input { prompt, var_name };
            }
            DialogueCommand::SetVar { name, value } => {
                if let Some(computed) = self.variables.eval_expr(&value) {
                    self.variables.set(&name, &computed);
                } else {
                    let interpolated = self.variables.interpolate(&value);
                    self.variables.set(&name, &interpolated);
                }
                self.advance_dialogue();
            }
            DialogueCommand::Background { filename } => {
                self.current_background = filename;
            }
            DialogueCommand::If { condition, target } => {
                if self.variables.eval_condition(&condition) {
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
            }
        }
    }

    pub fn skip_non_interactive_commands(&mut self) {
        loop {
            match &self.state {
                AppState::InDialogue {
                    scene_id,
                    cmd_index,
                } => {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(cmd) = scene.commands.get(*cmd_index) {
                            match cmd {
                                DialogueCommand::Image { .. }
                                | DialogueCommand::Background { .. }
                                | DialogueCommand::Music { .. }
                                | DialogueCommand::MusicStop
                                | DialogueCommand::SetVar { .. }
                                | DialogueCommand::If { .. } => {
                                    let next_index = cmd_index + 1;
                                    if let Some(next_cmd) = scene.commands.get(next_index) {
                                        self.state = AppState::InDialogue {
                                            scene_id: scene_id.clone(),
                                            cmd_index: next_index,
                                        };
                                        self.execute_command(next_cmd.clone());
                                        continue;
                                    } else {
                                        break;
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
        self.current_file = Some("dialogue.ng".to_string());
        self.current_bgm = None;
        self.current_image_params = None;
        self.current_background = None;
        self.target_text = String::new();
        self.display_text = String::new();
        self.input_buffer = String::new();
        self.prev_state = None;
        self.stop_bgm();

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

    pub fn load_external_file(&mut self, file_name: &str) -> Result<()> {
        let path = Path::new("assets/dialog").join(file_name);
        let content = fs::read_to_string(&path)?;
        let (new_scenes, scene_order) = parser::parse_dialogue_file_with_order(&content)?;
        for (k, v) in new_scenes {
            self.scenes.insert(k, v);
        }
        self.file_scene_order.insert(file_name.to_string(), scene_order);
        Ok(())
    }

    pub fn save_game_slot(&mut self, slot: usize) {
        let save_state = if let Some(prev) = &self.prev_state {
            prev.as_ref().clone()
        } else {
            self.state.clone()
        };
        let image_params = self.current_image_params.clone();
        let background = self.current_background.clone();
        let bgm = self.current_bgm.clone();
        let current_file = self.current_file.clone().and_then(|f| {
            if f == "null" || f.is_empty() { None } else { Some(f) }
        });
    
        if let Err(e) = SaveData::save(
            slot,
            &save_state,
            self.selected,
            &self.variables,
            current_file,
            background,
            bgm,
            image_params,
        ) {
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
                self.current_image_params = data.image_params;
                self.current_background = data.background;
    
                
                if let Some(file) = data.current_file {
                    if file == "null" || file.is_empty() {
                        self.current_file = None;
                    } else {
                        self.current_file = Some(file.clone());
                        
                        if !self.file_scene_order.contains_key(&file) {
                            let _ = self.load_external_file(&file);
                        }
                        if let AppState::InDialogue { scene_id, .. } = &self.state {
                            if !self.scenes.contains_key(scene_id) {
                                let _ = self.load_external_file(&file);
                            }
                        }
                    }
                } else {
                    self.current_file = None;
                }
    
                
                if self.current_file.is_none() {
                    if let AppState::InDialogue { scene_id, .. } = &self.state {
                        if self.scenes.contains_key(scene_id) {
                            self.current_file = Some("dialogue.ng".to_string());
                        }
                    }
                }
    
                
                if let Some(bgm) = &data.bgm {
                    self.current_bgm = Some(bgm.clone());
                    self.play_bgm(bgm);
                } else {
                    self.current_bgm = None;
                    self.stop_bgm();
                }
    
                self.status_message = Some(format!("从槽位 {} 读档成功", slot));
                self.prev_state = None;
    
                
                if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(cmd) = scene.commands.get(*cmd_index) {
                            self.execute_command(cmd.clone());
                        }
                    }
                }
            }
            Err(e) => {
                self.status_message = Some(format!("读档失败: {}", e));
                if let Some(prev) = self.prev_state.take() {
                    self.state = *prev;
                } else {
                    self.state = AppState::Menu;
                    self.play_title_bgm();
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
            AppState::InDialogue {
                scene_id,
                cmd_index,
            } => (scene_id.clone(), *cmd_index),
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
            self.target_text = String::new();
            self.display_text = String::new();
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                cmd_index: next_cmd_index,
            };
            self.execute_command(next_cmd.clone());
            self.skip_non_interactive_commands();
        } else {
            
            let next_scene = if let Some(file) = &self.current_file {
                if let Some(order) = self.file_scene_order.get(file) {
                    if let Some(pos) = order.iter().position(|s| s == &current_scene_id) {
                        if pos + 1 < order.len() {
                            Some(order[pos + 1].clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(next_scene_id) = next_scene {
                let next_scene_id_clone = next_scene_id.clone();
                self.target_text = String::new();
                self.display_text = String::new();
                self.state = AppState::InDialogue {
                    scene_id: next_scene_id_clone,
                    cmd_index: 0,
                };
                if let Some(scene) = self.scenes.get(&next_scene_id) {
                    if let Some(first_cmd) = scene.commands.first() {
                        self.execute_command(first_cmd.clone());
                    }
                }
                self.skip_non_interactive_commands();
            } else {
                
                self.state = AppState::EndOfFile;
                self.current_image_params = None;
                self.current_background = None;
                self.status_message = Some("剧情结束，按任意键返回主菜单".to_string());
            }
        }
    }

    pub fn select_option(&mut self) {
        let (options, selected, _current_scene_id) = match &self.state {
            AppState::InChoice {
                options,
                selected,
                scene_id,
            } => (options.clone(), *selected, scene_id.clone()),
            _ => return,
        };

        if let Some((_, next_scene)) = options.get(selected) {
            self.target_text = String::new();
            self.display_text = String::new();
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
            SettingsAction::TextAnimationToggle => {
                self.config.text_animation = !self.config.text_animation;
                if !self.config.text_animation && self.display_text != self.target_text {
                    self.display_text = self.target_text.clone();
                }
                self.status_message = Some(
                    if self.config.text_animation {
                        "文字动画开启"
                    } else {
                        "文字动画关闭"
                    }
                    .to_string(),
                );
            }
            SettingsAction::TextSpeedUp => {
                if self.config.text_speed <= 90 {
                    self.config.text_speed += 10;
                    self.status_message = Some(format!("文字速度: {}ms", self.config.text_speed));
                }
            }
            SettingsAction::TextSpeedDown => {
                if self.config.text_speed >= 20 {
                    self.config.text_speed -= 10;
                    self.status_message = Some(format!("文字速度: {}ms", self.config.text_speed));
                }
            }
            SettingsAction::Save => {
                if let Err(e) = self.config.save() {
                    self.status_message = Some(format!("保存配置失败: {}", e));
                } else {
                    self.status_message = Some("配置已保存".to_string());
                }
            }
            SettingsAction::BgColorNext => {
                let colors = vec![
                    "default".to_string(),
                    "#2A2A3E".to_string(),
                    "#1E1E2E".to_string(),
                    "#222436".to_string(),
                    "#2C2C3C".to_string(),
                    "#3A2C3C".to_string(),
                ];
                let current = colors
                    .iter()
                    .position(|c| c == &self.config.background_color)
                    .unwrap_or(0);
                let next = (current + 1) % colors.len();
                self.config.background_color = colors[next].clone();
                let color_name = match self.config.background_color.as_str() {
                    "default" => "终端默认",
                    "#2A2A3E" => "深灰紫",
                    "#1E1E2E" => "猫鼬暗色",
                    "#222436" => "深藏青",
                    "#2C2C3C" => "暖灰",
                    "#3A2C3C" => "紫罗兰灰",
                    _ => &self.config.background_color,
                };
                self.status_message = Some(format!("背景颜色: {}", color_name));
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
            AppState::InDialogue {
                scene_id,
                cmd_index,
            } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    if let Some(DialogueCommand::Text { speaker, .. }) =
                        scene.commands.get(*cmd_index)
                    {
                        if let Some(s) = speaker {
                            return Some(self.variables.interpolate(s));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn current_text(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue {
                scene_id,
                cmd_index,
            } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    if let Some(DialogueCommand::Text { text, .. }) = scene.commands.get(*cmd_index)
                    {
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

    fn handle_game_menu(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected < 3 {
                    self.selected += 1;
                }
            }
            KeyCode::Char('1') => {
                if let Some(prev) = self.prev_state.take() {
                    self.state = *prev;
                } else {
                    self.state = AppState::Menu;
                    self.play_title_bgm(); 
                }
            }
            KeyCode::Char('2') => {
                self.state = AppState::SaveSlot;
            }
            KeyCode::Char('3') => {
                self.state = AppState::LoadSlot;
            }
            KeyCode::Char('q') => {
                self.state = AppState::Menu;
                self.prev_state = None;
                self.play_title_bgm(); 
            }
            KeyCode::Enter => match self.selected {
                0 => {
                    if let Some(prev) = self.prev_state.take() {
                        self.state = *prev;
                    } else {
                        self.state = AppState::Menu;
                        self.play_title_bgm(); 
                    }
                }
                1 => self.state = AppState::SaveSlot,
                2 => self.state = AppState::LoadSlot,
                3 => {
                    self.state = AppState::Menu;
                    self.prev_state = None;
                    self.play_title_bgm(); 
                }
                _ => {}
            },
            _ => {}
        }
    }
    pub fn handle_event(&mut self, key: KeyCode) {
        self.status_message = None;

        match self.state {
            AppState::History => {
                
                let items_len = self.history.len();
                
                match key {
                    KeyCode::Up => {
                        
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        
                        if self.selected < items_len.saturating_sub(1) {
                            self.selected += 1;
                        }
                    }
                    _ => {
                        
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
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
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected < 9 {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.save_game_slot(self.selected + 1);
                    }
                    KeyCode::Esc => {
                        self.state = AppState::GameMenu;
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
                let valid_slots: Vec<usize> = (1..=10).filter(|&i| SaveData::exists(i)).collect();
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected < valid_slots.len().saturating_sub(1) {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(&slot) = valid_slots.get(self.selected) {
                            self.load_game_slot(slot);
                            return;
                        }
                    }
                    KeyCode::Esc => {
                        self.state = AppState::GameMenu;
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let slot = c.to_digit(10).unwrap() as usize;
                        if slot >= 1 && slot <= 10 && SaveData::exists(slot) {
                            self.load_game_slot(slot);
                            return;
                        }
                    }
                    _ => {}
                }
                return;
            }
            AppState::GameMenu => {
                self.handle_game_menu(key);
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
            AppState::EndOfFile => {
                match key {
                    _ => {
                        self.state = AppState::Menu;
                        self.play_title_bgm();
                    }
                }
                return;
            }
            _ => {}
        }

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
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.handle_settings(SettingsAction::BgmUp)
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        self.handle_settings(SettingsAction::BgmDown)
                    }
                    KeyCode::Char('[') => self.handle_settings(SettingsAction::VoiceDown),
                    KeyCode::Char(']') => self.handle_settings(SettingsAction::VoiceUp),
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.handle_settings(SettingsAction::AutoPlayToggle)
                    }
                    KeyCode::Char('1') => self.handle_settings(SettingsAction::AutoPlaySpeedDown),
                    KeyCode::Char('2') => self.handle_settings(SettingsAction::AutoPlaySpeedUp),
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        self.handle_settings(SettingsAction::TextAnimationToggle)
                    }
                    KeyCode::Char('3') => self.handle_settings(SettingsAction::TextSpeedDown),
                    KeyCode::Char('4') => self.handle_settings(SettingsAction::TextSpeedUp),
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        self.handle_settings(SettingsAction::BgColorNext)
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.handle_settings(SettingsAction::Save)
                    }
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
                        self.prev_state = Some(Box::new(self.state.clone()));
                        self.selected = 0;
                        self.state = AppState::GameMenu;
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
            AppState::InChoice { .. } => match key {
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
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.stop_voice();
                    self.prev_state = Some(Box::new(self.state.clone()));
                    self.selected = 0;
                    self.state = AppState::GameMenu;
                    self.auto_play_timer = None;
                    return;
                }
                _ => {}
            },
            _ => {}
        }

        if let AppState::InChoice {
            options, selected, ..
        } = &mut self.state
        {
            let options_count = options.len();
            match key {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if *selected < options_count - 1 {
                        *selected += 1;
                    }
                }
                KeyCode::Enter => self.select_option(),
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.stop_voice();
                    self.prev_state = Some(Box::new(self.state.clone()));
                    self.selected = 0;
                    self.state = AppState::GameMenu;
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