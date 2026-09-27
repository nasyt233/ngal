use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

const MENU_ITEMS: &[&str] = &[
    "插入对话",
    "插入音乐",
    "插入背景",
    "插入图片",
    "插入分支",
    "插入场景标记",
    "插入加载",
    "插入命令",
    "插入结束",
    "────────────────",
    "运行测试 (F5)",
    "打包游戏",
    "保存文件",
    "撤销 (Ctrl+Z)",
    "退出",
];

const UNDO_MAX: usize = 100;

enum EditorMode {
    Normal,
    FileListFocus,
    Input,
    FilePicker,
    ScenePicker,
    DirectEdit,
}

pub struct Editor {
    content: Vec<String>,
    sidebar_selected: usize,
    sidebar_state: ListState,
    content_scroll: u16,
    content_hscroll: u16,
    content_cursor: usize,
    content_col: usize,
    mode: EditorMode,
    input_prompt: String,
    input_buffer: String,
    file_path: PathBuf,
    assets_dir: PathBuf,
    status_message: Option<String>,
    should_quit: bool,
    picker_files: Vec<String>,
    picker_selected: usize,
    picker_target: usize,
    picker_title: String,
    scene_list: Vec<(String, String)>,
    scene_selected: usize,
    scene_title: String,
    story_files: Vec<String>,
    story_selected: usize,
    story_state: ListState,
    dialog_dir: PathBuf,
    run_test_requested: bool,
    build_requested: bool,
    undo_stack: Vec<(Vec<String>, usize, usize)>,
    content_area_width: u16,
    clipboard: Vec<String>,
    show_help: bool,
}

// ======================== 语法高亮 ========================

/// 判断是否是场景标记行： [xxx]
fn is_scene_marker(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('[') && s.ends_with(']') && !s.contains(':') && s.len() >= 3
}

/// 高亮代码部分（不含注释）
fn highlight_code(s: &str) -> Vec<Span<'static>> {
    let s = s.to_string();

    // 场景标记
    if is_scene_marker(&s) {
        return vec![Span::styled(
            s,
            Style::default()
                .fg(Color::Rgb(255, 220, 100))
                .add_modifier(Modifier::BOLD),
        )];
    }

    // end 命令
    if s.trim() == "end" {
        return vec![Span::styled(
            s,
            Style::default()
                .fg(Color::Rgb(255, 130, 130))
                .add_modifier(Modifier::BOLD),
        )];
    }

    // 关键字命令
    let keywords: &[(&str, Color)] = &[
        ("music:", Color::Rgb(100, 200, 255)),
        ("bg:", Color::Rgb(100, 200, 255)),
        ("img:", Color::Rgb(100, 200, 255)),
        ("choose:", Color::Rgb(200, 130, 255)),
        ("load:", Color::Rgb(255, 130, 130)),
        ("input:", Color::Rgb(100, 255, 150)),
        ("if ", Color::Rgb(255, 180, 100)),
    ];

    for (kw, color) in keywords {
        if s.starts_with(kw) {
            let rest = s[kw.len()..].to_string();
            let kw_str = s[..kw.len()].to_string();
            return vec![
                Span::styled(
                    kw_str,
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(rest, Style::default().fg(Color::Rgb(230, 230, 230))),
            ];
        }
    }

    // $(command xxx)
    if s.contains("$(") {
        return vec![Span::styled(
            s,
            Style::default().fg(Color::Rgb(180, 130, 255)),
        )];
    }

    // 变量赋值 xxx = yyy 或 xxx = a + b
    if let Some(eq_pos) = s.find('=') {
        let left = s[..eq_pos].to_string();
        let right = s[eq_pos..].to_string();
        // 排除条件判断（>= <= == !=）
        let is_assign = !s.contains(">=") && !s.contains("<=") && !s.contains("==") && !s.contains("!=");
        if is_assign && !left.contains('(') && left.trim().len() < 30 {
            return vec![
                Span::styled(left, Style::default().fg(Color::Rgb(150, 220, 255))),
                Span::styled(right, Style::default().fg(Color::Rgb(230, 230, 230))),
            ];
        }
    }

    // 说话人:文本 (简单判断：冒号在合理位置，不含等号)
    if let Some(colon_pos) = s.find(':') {
        let speaker = &s[..colon_pos];
        let text = &s[colon_pos..];
        if !speaker.contains('=') && !speaker.contains(' ') && speaker.len() < 24 && !speaker.is_empty() {
            return vec![
                Span::styled(
                    speaker.to_string(),
                    Style::default().fg(Color::Rgb(255, 200, 100)),
                ),
                Span::styled(text.to_string(), Style::default().fg(Color::Rgb(230, 230, 230))),
            ];
        }
    }

    // 默认
    vec![Span::styled(s, Style::default().fg(Color::Rgb(230, 230, 230)))]
}

/// 整行高亮（含注释处理）
fn highlight_line(s: &str) -> Vec<Span<'static>> {
    // 检测注释（# 后面）
    if let Some(hash_pos) = s.find('#') {
        let code_part = &s[..hash_pos];
        let comment_part = &s[hash_pos..];
        let mut spans = highlight_code(code_part);
        spans.push(Span::styled(
            comment_part.to_string(),
            Style::default().fg(Color::Rgb(100, 100, 100)),
        ));
        return spans;
    }

    highlight_code(s)
}

/// 带光标的高亮（编辑模式下当前行）
fn highlight_line_with_cursor(s: &str, cursor_col: usize) -> Vec<Span<'static>> {
    // 简化处理：光标位置之前的正常高亮，光标字符用背景色
    let chars: Vec<char> = s.chars().collect();
    let pos = cursor_col.min(chars.len());

    let mut spans = Vec::new();

    // 光标前部分
    if pos > 0 {
        let before: String = chars[..pos].iter().collect();
        spans.extend(highlight_line(&before));
    }

    // 光标字符（反白）
    if pos < chars.len() {
        let c = chars[pos].to_string();
        spans.push(Span::styled(
            c,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(100, 255, 100))
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        // 行尾光标：显示空格
        spans.push(Span::styled(
            " ",
            Style::default().bg(Color::Rgb(100, 255, 100)),
        ));
    }

    // 光标后部分
    if pos + 1 < chars.len() {
        let after: String = chars[pos + 1..].iter().collect();
        spans.extend(highlight_line(&after));
    }

    spans
}

// ======================== Editor 实现 ========================

impl Editor {
    pub fn new(file_path: PathBuf) -> Result<Self> {
        let content = if file_path.exists() {
            fs::read_to_string(&file_path)?
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            vec!["[welcome]".to_string(), String::new()]
        };

        let assets_dir = file_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("assets"));

        let dialog_dir = assets_dir.join("dialog");

        let mut sidebar_state = ListState::default();
        sidebar_state.select(Some(0));

        let mut story_state = ListState::default();
        story_state.select(Some(0));

        let mut editor = Self {
            content,
            sidebar_selected: 0,
            sidebar_state,
            content_scroll: 0,
            content_hscroll: 0,
            content_cursor: 0,
            content_col: 0,
            mode: EditorMode::Normal,
            input_prompt: String::new(),
            input_buffer: String::new(),
            file_path,
            assets_dir,
            status_message: None,
            should_quit: false,
            picker_files: Vec::new(),
            picker_selected: 0,
            picker_target: 0,
            picker_title: String::new(),
            scene_list: Vec::new(),
            scene_selected: 0,
            scene_title: String::new(),
            story_files: Vec::new(),
            story_selected: 0,
            story_state,
            dialog_dir,
            run_test_requested: false,
            build_requested: false,
            undo_stack: Vec::new(),
            content_area_width: 80,
            clipboard: Vec::new(),
            show_help: false,
        };

        editor.load_story_files();
        let current_name = editor
            .file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if let Some(idx) = editor.story_files.iter().position(|f| f == &current_name) {
            editor.story_selected = idx;
            editor.story_state.select(Some(idx));
        }

        Ok(editor)
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= UNDO_MAX {
            self.undo_stack.remove(0);
        }
        self.undo_stack
            .push((self.content.clone(), self.content_cursor, self.content_col));
    }

    fn undo(&mut self) {
        if let Some((content, cursor, col)) = self.undo_stack.pop() {
            self.content = content;
            self.content_cursor = cursor.min(self.content.len().saturating_sub(1));
            self.content_col = col;
            self.clamp_col();
            self.adjust_hscroll();
            self.status_message = Some(format!("已撤销 (剩余 {} 步)", self.undo_stack.len()));
        } else {
            self.status_message = Some("没有可撤销的操作".to_string());
        }
    }

    fn load_story_files(&mut self) {
        let mut files = Vec::new();
        if self.dialog_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.dialog_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if ext == "ng" || ext == "txt" {
                                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                    files.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        files.sort();
        self.story_files = files;

        if self.story_files.is_empty() {
            self.story_selected = 0;
            self.story_state.select(None);
        } else if self.story_selected >= self.story_files.len() {
            self.story_selected = self.story_files.len() - 1;
            self.story_state.select(Some(self.story_selected));
        }
    }

    fn switch_to_file(&mut self, filename: &str) {
        let current = self.content.join("\n");
        let _ = fs::write(&self.file_path, current);

        let new_path = self.dialog_dir.join(filename);
        if !new_path.exists() {
            self.status_message = Some(format!("文件不存在: {}", filename));
            return;
        }

        match fs::read_to_string(&new_path) {
            Ok(text) => {
                self.content = text.lines().map(|s| s.to_string()).collect();
                self.file_path = new_path;
                self.content_cursor = 0;
                self.content_col = 0;
                self.content_scroll = 0;
                self.content_hscroll = 0;
                self.undo_stack.clear();
                self.status_message = Some(format!("已切换到 {}", filename));
            }
            Err(e) => {
                self.status_message = Some(format!("读取失败: {}", e));
            }
        }
    }

    fn save(&mut self) {
        let text = self.content.join("\n");
        match fs::write(&self.file_path, text) {
            Ok(_) => {
                self.status_message = Some(format!("已保存到 {}", self.file_path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("保存失败: {}", e));
            }
        }
    }

    fn build_insert_line(&self) -> String {
        match self.sidebar_selected {
            0 => self.input_buffer.clone(),
            4 => format!("choose:{}", self.input_buffer),
            5 => format!("[{}]", self.input_buffer),
            7 => format!("$(command {})", self.input_buffer),
            _ => self.input_buffer.clone(),
        }
    }

    fn open_file_picker(&mut self, target: usize) {
        let (dir, title) = match target {
            0 => (self.assets_dir.join("music"), "选择音乐文件"),
            1 => (self.assets_dir.join("portraits"), "选择背景图片"),
            2 => (self.assets_dir.join("portraits"), "选择立绘图片"),
            _ => return,
        };

        let mut files = Vec::new();
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_file() {
                            if let Some(name) = entry.file_name().to_str() {
                                files.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        files.sort();

        self.picker_files = files;
        self.picker_selected = 0;
        self.picker_target = target;
        self.picker_title = title.to_string();
        self.mode = EditorMode::FilePicker;
    }

    fn open_scene_picker(&mut self) {
        let mut scenes: Vec<(String, String)> = Vec::new();

        if self.dialog_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.dialog_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        if ext == "ng" || ext == "txt" {
                            let filename = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();
                            if let Ok(content) = fs::read_to_string(&path) {
                                for line in content.lines() {
                                    let line = line.split('#').next().unwrap_or("").trim();
                                    if line.starts_with('[') && line.ends_with(']') {
                                        let scene_name =
                                            line[1..line.len() - 1].trim().to_string();
                                        if !scene_name.is_empty() {
                                            scenes.push((scene_name, filename.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        scenes.sort_by(|a, b| a.0.cmp(&b.0));

        self.scene_list = scenes;
        self.scene_selected = 0;
        self.scene_title = "选择场景".to_string();
        self.mode = EditorMode::ScenePicker;
    }

    fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.push_undo();
                let line = self.build_insert_line();
                let insert_pos = self.content_cursor.min(self.content.len());
                self.content.insert(insert_pos, line);
                self.content_cursor = insert_pos + 1;
                self.mode = EditorMode::Normal;
                self.input_buffer.clear();
                self.input_prompt.clear();
                self.status_message = Some("已插入".to_string());
            }
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.input_buffer.clear();
                self.input_prompt.clear();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_file_picker(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已取消选择".to_string());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.picker_selected > 0 {
                    self.picker_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.picker_files.is_empty()
                    && self.picker_selected < self.picker_files.len() - 1
                {
                    self.picker_selected += 1;
                }
            }
            KeyCode::Home => {
                self.picker_selected = 0;
            }
            KeyCode::End => {
                if !self.picker_files.is_empty() {
                    self.picker_selected = self.picker_files.len() - 1;
                }
            }
            KeyCode::PageUp => {
                self.picker_selected = self.picker_selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if !self.picker_files.is_empty() {
                    self.picker_selected =
                        (self.picker_selected + 10).min(self.picker_files.len() - 1);
                }
            }
            KeyCode::Enter => {
                if let Some(filename) = self.picker_files.get(self.picker_selected).cloned() {
                    self.push_undo();
                    let line = match self.picker_target {
                        0 => format!("music:{}", filename),
                        1 => format!("bg:{}", filename),
                        2 => format!("img:{}", filename),
                        _ => filename.clone(),
                    };
                    let insert_pos = self.content_cursor.min(self.content.len());
                    self.content.insert(insert_pos, line);
                    self.content_cursor = insert_pos + 1;
                    self.status_message = Some(format!("已插入: {}", filename));
                }
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_scene_picker(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已取消选择".to_string());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scene_selected > 0 {
                    self.scene_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.scene_list.is_empty()
                    && self.scene_selected < self.scene_list.len() - 1
                {
                    self.scene_selected += 1;
                }
            }
            KeyCode::Home => {
                self.scene_selected = 0;
            }
            KeyCode::End => {
                if !self.scene_list.is_empty() {
                    self.scene_selected = self.scene_list.len() - 1;
                }
            }
            KeyCode::PageUp => {
                self.scene_selected = self.scene_selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if !self.scene_list.is_empty() {
                    self.scene_selected =
                        (self.scene_selected + 10).min(self.scene_list.len() - 1);
                }
            }
            KeyCode::Enter => {
                if let Some((scene_name, file_name)) =
                    self.scene_list.get(self.scene_selected).cloned()
                {
                    self.push_undo();
                    let current_file = self
                        .file_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let line = if file_name == current_file {
                        format!("load:{}", scene_name)
                    } else {
                        format!("load:{}:{}", file_name, scene_name)
                    };
                    let insert_pos = self.content_cursor.min(self.content.len());
                    self.content.insert(insert_pos, line);
                    self.content_cursor = insert_pos + 1;
                    self.status_message = Some(format!("已插入: {}", scene_name));
                }
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_file_list_focus(&mut self, key: KeyCode) {
        match key {
            KeyCode::F(5) => {
                self.request_run_test();
            }
            KeyCode::Tab => {
                self.mode = EditorMode::DirectEdit;
                self.content_col = 0;
                self.status_message =
                    Some("编辑模式 | Tab切换 | F5运行测试 | Ctrl+Z撤销".to_string());
            }
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.story_selected > 0 {
                    self.story_selected -= 1;
                    self.story_state.select(Some(self.story_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.story_files.is_empty()
                    && self.story_selected < self.story_files.len() - 1
                {
                    self.story_selected += 1;
                    self.story_state.select(Some(self.story_selected));
                }
            }
            KeyCode::Home => {
                self.story_selected = 0;
                self.story_state.select(Some(0));
            }
            KeyCode::End => {
                if !self.story_files.is_empty() {
                    self.story_selected = self.story_files.len() - 1;
                    self.story_state.select(Some(self.story_selected));
                }
            }
            KeyCode::PageUp => {
                let step = 10;
                self.story_selected = self.story_selected.saturating_sub(step);
                self.story_state.select(Some(self.story_selected));
            }
            KeyCode::PageDown => {
                let step = 10;
                if !self.story_files.is_empty() {
                    if self.story_selected + step < self.story_files.len() {
                        self.story_selected += step;
                    } else {
                        self.story_selected = self.story_files.len() - 1;
                    }
                    self.story_state.select(Some(self.story_selected));
                }
            }
            KeyCode::Enter => {
                if let Some(filename) = self.story_files.get(self.story_selected).cloned() {
                    self.switch_to_file(&filename);
                }
            }
            KeyCode::Char('r') => {
                self.load_story_files();
                self.status_message = Some("已刷新文件列表".to_string());
            }
            KeyCode::Char('i') => {
                self.mode = EditorMode::DirectEdit;
                self.content_col = 0;
                self.status_message = Some("编辑模式".to_string());
            }
            KeyCode::Char('h') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn adjust_hscroll(&mut self) {
        let view_width = self.content_area_width.saturating_sub(6);
        let cursor_pos = self.content_col as u16;

        if cursor_pos >= self.content_hscroll + view_width {
            self.content_hscroll = cursor_pos.saturating_sub(view_width) + 1;
        }
        if cursor_pos < self.content_hscroll {
            self.content_hscroll = cursor_pos;
        }
    }

    fn handle_direct_edit(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if modifiers.contains(KeyModifiers::CONTROL) {
            match key {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.save();
                    return;
                }
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    self.undo();
                    return;
                }
                _ => {}
            }
        }

        match key {
            KeyCode::F(5) => {
                self.request_run_test();
            }
            KeyCode::Tab => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已切换到菜单模式".to_string());
            }
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已退出编辑模式".to_string());
            }
            KeyCode::Up => {
                if self.content_cursor > 0 {
                    self.content_cursor -= 1;
                    self.clamp_col();
                    self.adjust_hscroll();
                    if self.content_cursor < self.content_scroll as usize {
                        self.content_scroll = self.content_cursor as u16;
                    }
                }
            }
            KeyCode::Down => {
                if self.content_cursor < self.content.len().saturating_sub(1) {
                    self.content_cursor += 1;
                    self.clamp_col();
                    self.adjust_hscroll();
                    let cursor_line = self.content_cursor as u16;
                    if cursor_line >= self.content_scroll + 20 {
                        self.content_scroll = cursor_line.saturating_sub(20);
                    }
                }
            }
            KeyCode::PageUp => {
                let step = 10;
                self.content_cursor = self.content_cursor.saturating_sub(step);
                self.clamp_col();
                self.adjust_hscroll();
                if self.content_cursor < self.content_scroll as usize {
                    self.content_scroll = self.content_cursor as u16;
                }
            }
            KeyCode::PageDown => {
                let step = 10;
                if self.content_cursor + step < self.content.len() {
                    self.content_cursor += step;
                } else {
                    self.content_cursor = self.content.len().saturating_sub(1);
                }
                self.clamp_col();
                self.adjust_hscroll();
                let cursor_line = self.content_cursor as u16;
                if cursor_line >= self.content_scroll + 20 {
                    self.content_scroll = cursor_line.saturating_sub(20);
                }
            }
            KeyCode::Left => {
                if self.content_col > 0 {
                    self.content_col -= 1;
                    self.adjust_hscroll();
                }
            }
            KeyCode::Right => {
                if self.content_cursor < self.content.len() {
                    let len = self.content[self.content_cursor].chars().count();
                    if self.content_col < len {
                        self.content_col += 1;
                        self.adjust_hscroll();
                    }
                }
            }
            KeyCode::Home => {
                self.content_col = 0;
                self.content_hscroll = 0;
            }
            KeyCode::End => {
                if self.content_cursor < self.content.len() {
                    self.content_col = self.content[self.content_cursor].chars().count();
                    self.adjust_hscroll();
                }
            }
            KeyCode::Enter => {
                self.push_undo();
                if self.content_cursor < self.content.len() {
                    let line = self.content[self.content_cursor].clone();
                    let chars: Vec<char> = line.chars().collect();
                    let pos = self.content_col.min(chars.len());
                    let left: String = chars[..pos].iter().collect();
                    let right: String = chars[pos..].iter().collect();
                    self.content[self.content_cursor] = left;
                    self.content.insert(self.content_cursor + 1, right);
                    self.content_cursor += 1;
                    self.content_col = 0;
                    self.content_hscroll = 0;
                }
            }
            KeyCode::Backspace => {
                self.push_undo();
                if self.content_col > 0 && self.content_cursor < self.content.len() {
                    let line = &mut self.content[self.content_cursor];
                    let chars: Vec<char> = line.chars().collect();
                    let pos = self.content_col.min(chars.len());
                    if pos > 0 {
                        let new_line: String = chars[..pos - 1]
                            .iter()
                            .chain(chars[pos..].iter())
                            .collect();
                        *line = new_line;
                        self.content_col -= 1;
                        self.adjust_hscroll();
                    }
                } else if self.content_col == 0 && self.content_cursor > 0 {
                    let current = self.content.remove(self.content_cursor);
                    self.content_cursor -= 1;
                    let prev_len = self.content[self.content_cursor].chars().count();
                    self.content[self.content_cursor].push_str(&current);
                    self.content_col = prev_len;
                    self.adjust_hscroll();
                }
            }
            KeyCode::Delete => {
                self.push_undo();
                if self.content_cursor < self.content.len() {
                    let line = &mut self.content[self.content_cursor];
                    let chars: Vec<char> = line.chars().collect();
                    let pos = self.content_col.min(chars.len());
                    if pos < chars.len() {
                        let new_line: String = chars[..pos]
                            .iter()
                            .chain(chars[pos + 1..].iter())
                            .collect();
                        *line = new_line;
                    }
                }
            }
            KeyCode::Char(c) => {
                self.push_undo();
                if self.content_cursor >= self.content.len() {
                    return;
                }
                let line = &mut self.content[self.content_cursor];
                let chars: Vec<char> = line.chars().collect();
                let pos = self.content_col.min(chars.len());
                let mut new_chars: Vec<char> = Vec::with_capacity(chars.len() + 1);
                new_chars.extend_from_slice(&chars[..pos]);
                new_chars.push(c);
                new_chars.extend_from_slice(&chars[pos..]);
                *line = new_chars.into_iter().collect();
                self.content_col += 1;
                self.adjust_hscroll();
            }
            _ => {}
        }
    }

    fn clamp_col(&mut self) {
        if self.content_cursor < self.content.len() {
            let len = self.content[self.content_cursor].chars().count();
            if self.content_col > len {
                self.content_col = len;
            }
        } else {
            self.content_col = 0;
        }
    }

    fn request_run_test(&mut self) {
        self.save();
        self.run_test_requested = true;
    }

    fn request_build(&mut self) {
        self.save();
        self.build_requested = true;
    }

    fn cut_line(&mut self) {
        if self.content_cursor < self.content.len() {
            self.push_undo();
            let line = self.content.remove(self.content_cursor);
            self.clipboard = vec![line];
            if self.content_cursor >= self.content.len() && !self.content.is_empty() {
                self.content_cursor = self.content.len() - 1;
            }
            self.clamp_col();
            self.status_message = Some("已剪切当前行".to_string());
        } else {
            self.status_message = Some("没有可剪切的行".to_string());
        }
    }

    fn copy_line(&mut self) {
        if self.content_cursor < self.content.len() {
            self.clipboard = vec![self.content[self.content_cursor].clone()];
            self.status_message = Some("已复制当前行".to_string());
        } else {
            self.status_message = Some("没有可复制的行".to_string());
        }
    }

    fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            self.status_message = Some("剪贴板为空".to_string());
            return;
        }
        self.push_undo();
        let insert_pos = self.content_cursor.min(self.content.len());
        for (i, line) in self.clipboard.iter().enumerate() {
            self.content.insert(insert_pos + i, line.clone());
        }
        self.content_cursor = insert_pos + self.clipboard.len().saturating_sub(1);
        self.clamp_col();
        self.status_message = Some(format!("已粘贴 {} 行", self.clipboard.len()));
    }

    fn handle_normal(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        // 帮助弹窗优先
        if self.show_help {
            self.show_help = false;
            return;
        }

        if key == KeyCode::F(5) {
            self.request_run_test();
            return;
        }
        if modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key, KeyCode::Char('z') | KeyCode::Char('Z'))
        {
            self.undo();
            return;
        }

        match key {
            KeyCode::Tab => {
                self.mode = EditorMode::FileListFocus;
                self.status_message =
                    Some("剧情文件列表 | Enter打开 | F5运行 | Tab切到编辑".to_string());
            }
            KeyCode::Char('q') => {
                self.save();
                self.should_quit = true;
            }
            // 上下移动菜单
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                    self.sidebar_state.select(Some(self.sidebar_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sidebar_selected < MENU_ITEMS.len() - 1 {
                    self.sidebar_selected += 1;
                    self.sidebar_state.select(Some(self.sidebar_selected));
                }
            }
            // 左右键控制内容区光标上下移动
            KeyCode::Left => {
                if self.content_cursor > 0 {
                    self.content_cursor -= 1;
                    self.clamp_col();
                    self.adjust_hscroll();
                    if self.content_cursor < self.content_scroll as usize {
                        self.content_scroll = self.content_cursor as u16;
                    }
                }
            }
            KeyCode::Right => {
                if self.content_cursor < self.content.len().saturating_sub(1) {
                    self.content_cursor += 1;
                    self.clamp_col();
                    self.adjust_hscroll();
                    let cursor_line = self.content_cursor as u16;
                    if cursor_line >= self.content_scroll + 20 {
                        self.content_scroll = cursor_line.saturating_sub(20);
                    }
                }
            }
            // HOME/END 跳转菜单首尾
            KeyCode::Home => {
                self.sidebar_selected = 0;
                self.sidebar_state.select(Some(0));
            }
            KeyCode::End => {
                self.sidebar_selected = MENU_ITEMS.len() - 1;
                self.sidebar_state.select(Some(self.sidebar_selected));
            }
            // PgUp/PgDn 快速移动内容区光标
            KeyCode::PageUp => {
                let step = 10;
                self.content_cursor = self.content_cursor.saturating_sub(step);
                self.clamp_col();
                self.adjust_hscroll();
                if self.content_cursor < self.content_scroll as usize {
                    self.content_scroll = self.content_cursor as u16;
                }
            }
            KeyCode::PageDown => {
                let step = 10;
                if self.content_cursor + step < self.content.len() {
                    self.content_cursor += step;
                } else {
                    self.content_cursor = self.content.len().saturating_sub(1);
                }
                self.clamp_col();
                self.adjust_hscroll();
                let cursor_line = self.content_cursor as u16;
                if cursor_line >= self.content_scroll + 20 {
                    self.content_scroll = cursor_line.saturating_sub(20);
                }
            }
            KeyCode::Char('i') => {
                self.mode = EditorMode::DirectEdit;
                self.status_message = Some("编辑模式 | Tab/ESC退出 | Ctrl+S保存".to_string());
            }
            KeyCode::Char('h') => {
                self.show_help = true;
            }
            KeyCode::Char('x') => {
                self.cut_line();
            }
            KeyCode::Char('c') => {
                self.copy_line();
            }
            KeyCode::Char('v') => {
                self.paste_clipboard();
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                self.push_undo();
                if self.content_cursor < self.content.len() {
                    self.content.remove(self.content_cursor);
                    if self.content_cursor >= self.content.len() && !self.content.is_empty() {
                        self.content_cursor = self.content.len() - 1;
                    }
                    self.clamp_col();
                    self.status_message = Some("已删除一行".to_string());
                }
            }
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.save();
            }
            KeyCode::Enter => match self.sidebar_selected {
                0 => {
                    self.input_prompt = "对话 (格式: 说话人:文本)".to_string();
                    self.input_buffer.clear();
                    self.mode = EditorMode::Input;
                }
                1 => self.open_file_picker(0),
                2 => self.open_file_picker(1),
                3 => self.open_file_picker(2),
                4 => {
                    self.input_prompt = "分支 (格式: 选项1:场景1|选项2:场景2)".to_string();
                    self.input_buffer.clear();
                    self.mode = EditorMode::Input;
                }
                5 => {
                    self.input_prompt = "场景名".to_string();
                    self.input_buffer.clear();
                    self.mode = EditorMode::Input;
                }
                6 => self.open_scene_picker(),
                7 => {
                    self.input_prompt = "系统命令 (如 date, uptime, whoami)".to_string();
                    self.input_buffer.clear();
                    self.mode = EditorMode::Input;
                }
                8 => {
                    self.push_undo();
                    let insert_pos = self.content_cursor.min(self.content.len());
                    self.content.insert(insert_pos, "end".to_string());
                    self.content_cursor = insert_pos + 1;
                    self.status_message = Some("已插入 end".to_string());
                }
                9 => {}
                10 => self.request_run_test(),
                11 => self.request_build(),
                12 => self.save(),
                13 => self.undo(),
                14 => {
                    self.save();
                    self.should_quit = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_event(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match self.mode {
            EditorMode::Normal => self.handle_normal(key, modifiers),
            EditorMode::FileListFocus => self.handle_file_list_focus(key),
            EditorMode::Input => self.handle_input(key),
            EditorMode::FilePicker => self.handle_file_picker(key),
            EditorMode::ScenePicker => self.handle_scene_picker(key),
            EditorMode::DirectEdit => self.handle_direct_edit(key, modifiers),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
            original_hook(panic_info);
        }));

        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;

            if self.run_test_requested {
                self.run_test_requested = false;

                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;

                self.spawn_test_game();

                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.clear()?;

                self.status_message = Some("已从测试返回".to_string());
                continue;
            }

            if self.build_requested {
                self.build_requested = false;

                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;

                self.spawn_build();

                println!("\n按回车返回编辑器...");
                let _ = std::io::stdin().read_line(&mut String::new());

                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.clear()?;

                self.status_message = Some("已从打包返回".to_string());
                continue;
            }

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_event(key.code, key.modifiers);
                }
            }
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn game_root(&self) -> PathBuf {
        if let Ok(abs) = self.file_path.canonicalize() {
            if let Some(root) = abs
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                return root.to_path_buf();
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn spawn_test_game(&self) {
        let root = self.game_root();

        println!("正在启动测试: {}", root.display());

        if !root.exists() {
            println!("❌ 目录不存在: {}", root.display());
            println!("按回车返回编辑器...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return;
        }

        if !root.join("assets/game.json").exists() {
            println!("❌ 目录中没有游戏文件: {}", root.display());
            println!("按回车返回编辑器...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return;
        }

        println!("按 ESC 或 q 退出游戏返回编辑器...");
        println!();

        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ngal"));

        let status = std::process::Command::new(exe).arg(&root).status();

        match status {
            Ok(s) if s.success() => {
                println!("\n游戏已退出");
            }
            Ok(s) => {
                println!("\n游戏退出码: {:?}", s.code());
            }
            Err(e) => {
                println!("\n无法启动游戏: {}", e);
                println!("提示: 请确保 ngal 已安装或使用 cargo build 生成可执行文件");
            }
        }
    }

    fn spawn_build(&self) {
        let root = self.game_root();

        println!("正在打包: {}", root.display());
        println!();

        if !root.exists() {
            println!("❌ 目录不存在: {}", root.display());
            return;
        }

        if !root.join("assets/game.json").exists() {
            println!("❌ 目录中没有游戏文件: {}", root.display());
            return;
        }

        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ngal"));

        let status = std::process::Command::new(exe)
            .arg("build")
            .arg(&root)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("\n✅ 打包完成");
            }
            Ok(s) => {
                println!("\n打包退出码: {:?}", s.code());
            }
            Err(e) => {
                println!("\n无法启动打包: {}", e);
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        let main_area = chunks[0];
        let status_area = chunks[1];

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(main_area);

        let sidebar_area = cols[0];
        let content_area = cols[1];

        self.content_area_width = content_area.width;

        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(sidebar_area);

        let menu_area = sidebar_chunks[0];
        let file_list_area = sidebar_chunks[1];

        let is_editing = matches!(self.mode, EditorMode::DirectEdit);
        let is_menu_focus = matches!(self.mode, EditorMode::Normal);
        let is_filelist_focus = matches!(self.mode, EditorMode::FileListFocus);

        // 功能菜单
        let items: Vec<ListItem> = MENU_ITEMS
            .iter()
            .map(|s| {
                if *s == "────────────────" {
                    ListItem::new(Line::from(Span::styled(
                        *s,
                        Style::default().fg(Color::Rgb(100, 100, 100)),
                    )))
                } else {
                    ListItem::new(*s)
                }
            })
            .collect();

        let menu_border_color = if is_menu_focus {
            Color::Rgb(212, 112, 212)
        } else {
            Color::Rgb(80, 80, 80)
        };

        let sidebar_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 功能 ")
                    .border_style(Style::default().fg(menu_border_color)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 255, 0))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(sidebar_list, menu_area, &mut self.sidebar_state);

        // 剧情文件列表
        let file_items: Vec<ListItem> = if self.story_files.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "(无文件)",
                Style::default().fg(Color::Rgb(120, 120, 120)),
            )))]
        } else {
            self.story_files
                .iter()
                .map(|f| {
                    let current_name = self
                        .file_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let is_current = f == current_name;
                    if is_current {
                        ListItem::new(Line::from(Span::styled(
                            format!("● {}", f),
                            Style::default()
                                .fg(Color::Rgb(100, 255, 100))
                                .add_modifier(Modifier::BOLD),
                        )))
                    } else {
                        ListItem::new(Line::from(Span::styled(
                            format!("  {}", f),
                            Style::default().fg(Color::Rgb(200, 200, 200)),
                        )))
                    }
                })
                .collect()
        };

        let file_border_color = if is_filelist_focus {
            Color::Rgb(100, 200, 255)
        } else {
            Color::Rgb(80, 80, 80)
        };

        let file_list = List::new(file_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 剧情文件 ")
                    .border_style(Style::default().fg(file_border_color)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 255, 0))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(file_list, file_list_area, &mut self.story_state);

        // 右侧内容（带行号 + 语法高亮）
        let total_lines = self.content.len();
        let line_num_width = if total_lines < 10 {
            1
        } else if total_lines < 100 {
            2
        } else if total_lines < 1000 {
            3
        } else {
            4
        };

        let content_lines: Vec<Line> = self
            .content
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let line_num = format!("{:>width$} │ ", i + 1, width = line_num_width);

                let line_num_style = if i == self.content_cursor {
                    Style::default()
                        .fg(Color::Rgb(255, 200, 100))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(100, 100, 100))
                };

                let mut spans = vec![Span::styled(line_num, line_num_style)];

                if i == self.content_cursor && is_editing {
                    // 编辑模式：显示光标
                    spans.extend(highlight_line_with_cursor(s, self.content_col));
                } else if i == self.content_cursor {
                    // 非编辑但当前行：加粗底色
                    let line_spans = highlight_line(s);
                    for sp in line_spans {
                        spans.push(Span::styled(
                            sp.content.to_string(),
                            sp.style.add_modifier(Modifier::BOLD),
                        ));
                    }
                } else {
                    spans.extend(highlight_line(s));
                }

                Line::from(spans)
            })
            .collect();

        let undo_hint = if self.undo_stack.is_empty() {
            String::new()
        } else {
            format!(" [撤销:{}]", self.undo_stack.len())
        };

        let clipboard_hint = if self.clipboard.is_empty() {
            String::new()
        } else {
            format!(" [剪贴板:{}行]", self.clipboard.len())
        };

        let hscroll_hint = if self.content_hscroll > 0 {
            format!(" [→{}]", self.content_hscroll)
        } else {
            String::new()
        };

        let title = if is_editing {
            format!(
                " {} (编辑 行 {}/{}){}{}{} ",
                self.file_path.display(),
                self.content_cursor + 1,
                self.content.len(),
                undo_hint,
                clipboard_hint,
                hscroll_hint
            )
        } else {
            format!(
                " {} (行 {}/{}){}{}{} ",
                self.file_path.display(),
                self.content_cursor + 1,
                self.content.len(),
                undo_hint,
                clipboard_hint,
                hscroll_hint
            )
        };

        let content_border_color = if is_editing {
            Color::Rgb(100, 255, 100)
        } else {
            Color::Rgb(212, 112, 212)
        };

        let content_para = Paragraph::new(content_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(content_border_color)),
            )
            .scroll((self.content_scroll, self.content_hscroll));

        frame.render_widget(content_para, content_area);

        // 底部状态栏
        let status_text = match self.mode {
            EditorMode::Normal => {
                if let Some(msg) = &self.status_message {
                    msg.clone()
                } else {
                    "hjkl/↑↓菜单 | ←→内容光标 | i编辑 | x/c/v | h帮助 | F5测试 | q退出"
                        .to_string()
                }
            }
            EditorMode::FileListFocus => {
                if let Some(msg) = &self.status_message {
                    msg.clone()
                } else {
                    "↑↓/jk选择 | Home/End首尾 | Enter打开 | r刷新 | F5测试 | Tab编辑"
                        .to_string()
                }
            }
            EditorMode::Input => format!("{}: {}", self.input_prompt, self.input_buffer),
            EditorMode::FilePicker => {
                "↑↓/jk选择 | Home/End首尾 | Enter确认 | ESC取消".to_string()
            }
            EditorMode::ScenePicker => {
                "↑↓/jk选择 | Home/End首尾 | Enter确认 | ESC取消".to_string()
            }
            EditorMode::DirectEdit => {
                if let Some(msg) = &self.status_message {
                    msg.clone()
                } else {
                    "编辑 | 方向键移动 | Home/End行首尾 | F5测试 | Ctrl+S保存 | Ctrl+Z撤销 | Tab退出"
                        .to_string()
                }
            }
        };

        let status_style = match self.mode {
            EditorMode::Input => Style::default().fg(Color::Rgb(100, 255, 100)),
            EditorMode::FilePicker => Style::default().fg(Color::Rgb(100, 200, 255)),
            EditorMode::ScenePicker => Style::default().fg(Color::Rgb(255, 200, 100)),
            EditorMode::FileListFocus => Style::default().fg(Color::Rgb(100, 200, 255)),
            EditorMode::DirectEdit => Style::default().fg(Color::Rgb(100, 255, 100)),
            EditorMode::Normal => Style::default().fg(Color::Rgb(200, 200, 200)),
        };

        let status_border = match self.mode {
            EditorMode::DirectEdit => Color::Rgb(100, 255, 100),
            EditorMode::FileListFocus => Color::Rgb(100, 200, 255),
            EditorMode::ScenePicker => Color::Rgb(255, 200, 100),
            _ => Color::Rgb(100, 100, 100),
        };

        let status_para = Paragraph::new(status_text)
            .style(status_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(status_border)),
            );

        frame.render_widget(status_para, status_area);

        // 文件选择器弹窗
        if let EditorMode::FilePicker = self.mode {
            let popup_area = centered_rect(60, 60, area);
            frame.render_widget(Clear, popup_area);

            let items: Vec<ListItem> = if self.picker_files.is_empty() {
                vec![ListItem::new(Line::from(Span::styled(
                    "  (目录为空，按 ESC 返回)",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                )))]
            } else {
                self.picker_files
                    .iter()
                    .map(|f| ListItem::new(f.as_str()))
                    .collect()
            };

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {} ", self.picker_title))
                        .border_style(Style::default().fg(Color::Rgb(100, 200, 255))),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Rgb(255, 255, 0))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            let mut list_state = ListState::default();
            if !self.picker_files.is_empty() {
                list_state.select(Some(self.picker_selected));
            }
            frame.render_stateful_widget(list, popup_area, &mut list_state);
        }

        // 场景选择器弹窗
        if let EditorMode::ScenePicker = self.mode {
            let popup_area = centered_rect(70, 70, area);
            frame.render_widget(Clear, popup_area);

            let items: Vec<ListItem> = if self.scene_list.is_empty() {
                vec![ListItem::new(Line::from(Span::styled(
                    "  (没有找到场景，请检查 assets/dialog/)",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                )))]
            } else {
                self.scene_list
                    .iter()
                    .map(|(scene, file)| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:<20}", scene),
                                Style::default().fg(Color::Rgb(255, 255, 255)),
                            ),
                            Span::styled(
                                format!("  [{}]", file),
                                Style::default().fg(Color::Rgb(150, 150, 150)),
                            ),
                        ]))
                    })
                    .collect()
            };

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {} ", self.scene_title))
                        .border_style(Style::default().fg(Color::Rgb(255, 200, 100))),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Rgb(255, 255, 0))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            let mut list_state = ListState::default();
            if !self.scene_list.is_empty() {
                list_state.select(Some(self.scene_selected));
            }
            frame.render_stateful_widget(list, popup_area, &mut list_state);
        }

        // 帮助弹窗
        if self.show_help {
            let popup_area = centered_rect(70, 85, area);
            frame.render_widget(Clear, popup_area);

            let help_lines = vec![
                Line::from(Span::styled(
                    "  ngal 编辑器 - 快捷键帮助  ",
                    Style::default()
                        .fg(Color::Rgb(255, 215, 0))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " 【功能区】",
                    Style::default().fg(Color::Rgb(212, 112, 212)),
                )),
                Line::from("  ↑/↓/j/k     上下选择菜单"),
                Line::from("  ←/→          内容区光标上移/下移"),
                Line::from("  Home/End     跳到菜单首/尾"),
                Line::from("  PgUp/PgDn    内容区快速跳转"),
                Line::from("  Enter        执行选中功能"),
                Line::from("  i            进入右侧编辑模式"),
                Line::from("  h            显示本帮助"),
                Line::from("  x/c/v        剪切/复制/粘贴行"),
                Line::from("  d/Delete     删除当前行"),
                Line::from("  Tab          切换到文件列表"),
                Line::from("  F5           运行测试"),
                Line::from("  q            保存并退出"),
                Line::from(""),
                Line::from(Span::styled(
                    " 【剧情文件列表】",
                    Style::default().fg(Color::Rgb(100, 200, 255)),
                )),
                Line::from("  ↑/↓/j/k      选择文件"),
                Line::from("  Home/End     首/尾"),
                Line::from("  PgUp/PgDn    快速翻页"),
                Line::from("  Enter        打开选中文件"),
                Line::from("  r            刷新文件列表"),
                Line::from("  Tab          切到编辑区"),
                Line::from("  ESC          返回功能区"),
                Line::from(""),
                Line::from(Span::styled(
                    " 【编辑模式】",
                    Style::default().fg(Color::Rgb(100, 255, 100)),
                )),
                Line::from("  ←→↑↓         移动光标"),
                Line::from("  Home/End     行首/行尾"),
                Line::from("  PgUp/PgDn    翻页"),
                Line::from("  Enter        插入换行"),
                Line::from("  Backspace    删除字符"),
                Line::from("  Ctrl+S       保存文件"),
                Line::from("  Ctrl+Z       撤销"),
                Line::from("  F5           运行测试"),
                Line::from("  Tab/ESC      退出编辑"),
                Line::from(""),
                Line::from(Span::styled(
                    " 【语法高亮】",
                    Style::default().fg(Color::Rgb(200, 130, 255)),
                )),
                Line::from(Span::styled(
                    "  [场景]       黄色",
                    Style::default().fg(Color::Rgb(255, 220, 100)),
                )),
                Line::from(Span::styled(
                    "  说话人:      橙色",
                    Style::default().fg(Color::Rgb(255, 200, 100)),
                )),
                Line::from(Span::styled(
                    "  music/bg/img 青色",
                    Style::default().fg(Color::Rgb(100, 200, 255)),
                )),
                Line::from(Span::styled(
                    "  choose       紫色",
                    Style::default().fg(Color::Rgb(200, 130, 255)),
                )),
                Line::from(Span::styled(
                    "  load/end     红色",
                    Style::default().fg(Color::Rgb(255, 130, 130)),
                )),
                Line::from(Span::styled(
                    "  input        绿色",
                    Style::default().fg(Color::Rgb(100, 255, 150)),
                )),
                Line::from(Span::styled(
                    "  变量赋值     蓝色",
                    Style::default().fg(Color::Rgb(150, 220, 255)),
                )),
                Line::from(Span::styled(
                    "  # 注释       灰色",
                    Style::default().fg(Color::Rgb(100, 100, 100)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  按任意键关闭本窗口",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                )),
            ];

            let help_para = Paragraph::new(help_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 帮助 ")
                    .border_style(
                        Style::default()
                            .fg(Color::Rgb(255, 215, 0))
                            .add_modifier(Modifier::BOLD),
                    )
                    .style(Style::default().bg(Color::Rgb(20, 20, 30))),
            );

            frame.render_widget(help_para, popup_area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}