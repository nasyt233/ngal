// src/edit/state.rs
use ratatui::style::Color;

pub const MENU_ITEMS: &[&str] = &[
    "插入对话",
    "插入输入",
    "插入音乐",
    "插入背景",
    "插入图片",
    "插入分支",
    "插入场景标记",
    "插入加载",
    "插入等待",
    "插入命令",
    "插入结束",
    "────────────────",
    "检查统计",
    "运行测试 (F5)",
    "打包游戏",
    "保存文件",
    "命令执行",
    "撤销 (Ctrl+Z)",
    "退出",
];

pub const UNDO_MAX: usize = 100;

pub const C_BG: Color = Color::Rgb(24, 24, 34);
pub const C_BG_ALT: Color = Color::Rgb(30, 30, 42);
pub const C_BORDER_FOCUS: Color = Color::Rgb(180, 140, 255);
pub const C_BORDER_DIM: Color = Color::Rgb(70, 70, 90);
pub const C_ACCENT: Color = Color::Rgb(255, 180, 100);
pub const C_GREEN: Color = Color::Rgb(120, 255, 150);
pub const C_BLUE: Color = Color::Rgb(100, 200, 255);
pub const C_PINK: Color = Color::Rgb(255, 130, 200);
pub const C_GRAY: Color = Color::Rgb(130, 130, 150);
pub const C_YELLOW: Color = Color::Rgb(255, 230, 100);
pub const C_RED: Color = Color::Rgb(255, 120, 120);

pub enum EditorMode {
    Normal,
    FileListFocus,
    Input,
    FilePicker,
    ScenePicker,
    CharacterPicker,
    DirectEdit,
    ConfirmDelete,
    FileNameInput {
        action: FileNameAction,
        buffer: String,
    },
    Shell {
        buffer: String,
        output: Vec<String>,
    },
    ShowStats,
}

pub enum FileNameAction {
    Create,
    Rename(String),
}

#[derive(Clone, Default)]
pub struct StatInfo {
    pub project_name: String,
    pub version: String,
    pub total_size: u64,
    pub img_count: usize,
    pub img_size: u64,
    pub music_count: usize,
    pub music_size: u64,
    pub voice_count: usize,
    pub voice_size: u64,
    pub story_count: usize,
    pub story_size: u64,
    pub total_words: usize,
    pub missing_images: Vec<String>,
    pub missing_music: Vec<String>,
    pub missing_voices: Vec<String>,
}

#[derive(Clone)]
pub struct CompletionItem {
    pub label: String,
    pub insert: String,
    pub desc: String,
    pub kind: CompletionKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CompletionKind {
    Insert,
    FilePicker(usize),
    ScenePicker,
    CharacterPicker,
}