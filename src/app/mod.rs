// src/app/mod.rs
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;

use crate::audio;
use crate::config::Config;
use crate::parser::{self, ImageParams};
use crate::variables::Variables;

pub mod state;
pub mod sounds;
pub mod save_load;
pub mod script;
pub mod text;
pub mod settings;
pub mod event;
pub mod rewind;

pub use state::{AppState, SettingsAction, Snapshot};

const HISTORY_MAX: usize = 50;

pub struct App {
    pub state: AppState,
    pub menu_options: Vec<String>,
    pub selected: usize,
    pub status_message: Option<String>,
    pub scenes: HashMap<String, parser::SceneData>,
    pub config: Config,
    pub logo: Option<crate::image::RgbaImage>,
    pub menu_image: Option<crate::image::RgbaImage>,
    pub should_quit: bool,
    pub history: VecDeque<(Option<String>, String)>,
    pub sleep_until: Option<Instant>,
    pub auto_play_timer: Option<Instant>,
    pub prev_state: Option<Box<AppState>>,
    pub title: String,
    pub footer: String,
    pub menu_layout: u8,
    pub variables: Variables,
    pub input_buffer: String,
    pub current_background: Option<String>,
    pub current_image_params: Option<ImageParams>,
    pub target_text: String,
    pub display_text: String,
    pub last_char_time: Instant,
    pub current_file: Option<String>,
    pub current_bgm: Option<String>,
    pub file_scene_order: HashMap<String, Vec<String>>,
    pub snapshot_stack: Vec<Snapshot>,
    pub history_selected: usize,
}

impl App {
    pub fn new() -> Result<Self> {
        Self::ensure_directories()?;
        audio::init()?;

        let game_config = parser::load_game_config()?;
        let dialogue_content = parser::load_dialogue()?;
        let (scenes, order) = parser::parse_dialogue_file_with_order(&dialogue_content)?;

        let config = Config::load()?;

        let logo = if let Some(logo_file) = &game_config.logo {
            let path = format!("assets/portraits/{}", logo_file);
            crate::image::load_image_once(&path).ok()
        } else {
            None
        };

        let menu_image = if let Some(ref path) = game_config.menu_image {
            let img_path = format!("assets/portraits/{}", path);
            crate::image::load_image_once(&img_path).ok()
        } else {
            None
        };

        // 主菜单背景音乐
        if let Some(bgm_file) = &game_config.bgm {
            let bgm_path = format!("assets/music/{}", bgm_file);
            if crate::assets::exists(&bgm_path) {
                let _ = audio::play_bgm(&bgm_path, config.bgm_volume);
            }
        } else {
            let default_path = "assets/music/title.mp3";
            if crate::assets::exists(default_path) {
                let _ = audio::play_bgm(default_path, config.bgm_volume);
            }
        }

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
            logo,
            menu_image,
            should_quit: false,
            history: VecDeque::with_capacity(HISTORY_MAX),
            sleep_until: None,
            auto_play_timer: None,
            prev_state: None,
            title: game_config.title,
            footer: game_config.footer,
            menu_layout: game_config.menu_layout.clamp(1, 5),
            variables: Variables::new(),
            input_buffer: String::new(),
            current_background: None,
            current_image_params: None,
            target_text: String::new(),
            display_text: String::new(),
            last_char_time: Instant::now(),
            current_file: None,
            current_bgm: None,
            file_scene_order,
            snapshot_stack: Vec::new(),
            history_selected: 0,
        })
    }

    fn ensure_directories() -> io::Result<()> {
        // save/ 永远需要
        if !PathBuf::from("save").exists() {
            let _ = fs::create_dir_all("save");
        }

        // 文件系统模式才需要 assets 目录结构
        if !crate::assets::is_memory() {
            for dir in &[
                "assets",
                "assets/dialog",
                "assets/portraits",
                "assets/music",
                "assets/voices",
            ] {
                if !PathBuf::from(dir).exists() {
                    let _ = fs::create_dir_all(dir);
                }
            }
        }

        Ok(())
    }

    // ==================== 历史记录 ====================

    pub fn add_to_history(&mut self, speaker: Option<&str>, text: &str) {
        let speaker_clone = speaker.map(|s| s.to_string());
        self.history.push_back((speaker_clone, text.to_string()));
        while self.history.len() > HISTORY_MAX {
            self.history.pop_front();
        }
    }

    // ==================== 菜单执行 ====================

    pub fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => self.open_load_slot(),
            2 => self.state = AppState::About,
            3 => self.state = AppState::Settings,
            4 => self.should_quit = true,
            _ => {}
        }
    }
}