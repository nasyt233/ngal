// src/app/state.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::parser::ImageParams;

#[derive(Serialize, Deserialize, Clone)]
pub enum AppState {
    Menu,
    Settings,
    About,
    History,
    GameMenu,
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

/// 剧情回退快照
#[derive(Clone)]
pub struct Snapshot {
    pub state: AppState,
    pub current_file: Option<String>,
    pub variables: HashMap<String, String>,
    pub current_background: Option<String>,
    pub current_image_params: Option<ImageParams>,
    pub current_bgm: Option<String>,
    pub target_text: String,
}