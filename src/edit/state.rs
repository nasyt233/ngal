// src/edit/state.rs
use ratatui::style::Color;

pub const MENU_ITEMS: &[&str] = &[
    "插入对话",       // 0
    "插入输入",       // 1
    "插入音乐",       // 2
    "插入背景",       // 3
    "插入图片",       // 4
    "插入分支",       // 5
    "插入场景标记",   // 6
    "插入加载",       // 7
    "插入等待",       // 8
    "插入命令",       // 9
    "插入结束",       // 10
    "────────────────", // 11
    "游戏设置",       // 12  ← 新增
    "检查统计",       // 13
    "运行测试 (F5)",  // 14
    "打包游戏",       // 15
    "保存文件",       // 16
    "命令执行",       // 17
    "撤销 (Ctrl+Z)",  // 18
    "退出",           // 19
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
    GameSettings,   // ← 新增
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
    pub total_lines: usize,
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
    Variable,
}

// ==================== 游戏设置表单 ====================

#[derive(Clone, Copy, PartialEq)]
pub enum FieldKind {
    Text,
    FilePortrait,
    FileMusic,
    Number { min: u8, max: u8 },
}

#[derive(Clone)]
pub struct SettingsField {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
    pub kind: FieldKind,
}

pub struct GameSettingsForm {
    pub fields: Vec<SettingsField>,
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub error: Option<String>,
    pub dirty: bool,
}

impl GameSettingsForm {
    pub fn load() -> Self {
        let json = std::fs::read_to_string("assets/game.json").unwrap_or_default();
        let v: serde_json::Value =
            serde_json::from_str(&json).unwrap_or(serde_json::json!({}));

        let get_str = |key: &str, default: &str| -> String {
            v.get(key)
                .and_then(|x| x.as_str())
                .unwrap_or(default)
                .to_string()
        };
        let get_num = |key: &str, default: u64| -> String {
            v.get(key)
                .and_then(|x| x.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| default.to_string())
        };

        Self {
            fields: vec![
                SettingsField {
                    key: "title",
                    label: "游戏标题",
                    value: get_str("title", "我的游戏"),
                    kind: FieldKind::Text,
                },
                SettingsField {
                    key: "footer",
                    label: "底部提示",
                    value: get_str("footer", "按回车继续 | q 返回主菜单 | H 历史"),
                    kind: FieldKind::Text,
                },
                SettingsField {
                    key: "index",
                    label: "默认剧本",
                    value: get_str("index", "dialog/dialogue.ng"),
                    kind: FieldKind::Text,
                },
                SettingsField {
                    key: "logo",
                    label: "Logo 图片",
                    value: get_str("logo", ""),
                    kind: FieldKind::FilePortrait,
                },
                SettingsField {
                    key: "bgm",
                    label: "菜单 BGM",
                    value: get_str("bgm", ""),
                    kind: FieldKind::FileMusic,
                },
                SettingsField {
                    key: "menu_image",
                    label: "菜单图片",
                    value: get_str("menu_image", ""),
                    kind: FieldKind::FilePortrait,
                },
                SettingsField {
                    key: "menu_layout",
                    label: "菜单布局",
                    value: get_num("menu_layout", 1),
                    kind: FieldKind::Number { min: 1, max: 5 },
                },
            ],
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            error: None,
            dirty: false,
        }
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        use serde_json::{Map, Value};
        let mut map = Map::new();
        for f in &self.fields {
            let value = match f.kind {
                FieldKind::Number { min, max } => {
                    let n: u64 = f.value.parse().unwrap_or(min as u64);
                    let n = n.clamp(min as u64, max as u64);
                    Value::Number(n.into())
                }
                _ => {
                    if f.value.trim().is_empty() {
                        Value::Null
                    } else {
                        Value::String(f.value.clone())
                    }
                }
            };
            map.insert(f.key.to_string(), value);
        }
        let json = serde_json::to_string_pretty(&Value::Object(map))?;
        std::fs::write("assets/game.json", json)?;
        self.dirty = false;
        self.error = None;
        Ok(())
    }

    pub fn next(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.fields.len();
    }

    pub fn prev(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.fields.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn begin_edit(&mut self) {
        if let Some(f) = self.fields.get(self.selected) {
            self.edit_buffer = f.value.clone();
            self.editing = true;
        }
    }

    pub fn commit_edit(&mut self) {
        if let Some(f) = self.fields.get_mut(self.selected) {
            f.value = self.edit_buffer.clone();
            self.dirty = true;
        }
        self.editing = false;
        self.edit_buffer.clear();
        self.error = None;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn set_value(&mut self, idx: usize, value: String) {
        if let Some(f) = self.fields.get_mut(idx) {
            f.value = value;
            self.dirty = true;
        }
    }

    pub fn adjust_number(&mut self, delta: i32) {
        if let Some(f) = self.fields.get_mut(self.selected) {
            if let FieldKind::Number { min, max } = f.kind {
                let cur: i32 = f.value.parse().unwrap_or(min as i32);
                let new_val = (cur + delta).clamp(min as i32, max as i32);
                f.value = new_val.to_string();
                self.dirty = true;
            }
        }
    }
}