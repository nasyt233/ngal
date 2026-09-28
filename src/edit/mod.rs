// src/edit/mod.rs
pub mod keys;
pub mod mouse;
pub mod render;
pub mod shell;
pub mod state;
pub mod stats;
pub mod syntax;

pub use state::{
    EditorMode, FileNameAction, StatInfo, C_ACCENT, C_BG, C_BG_ALT, C_BLUE, C_BORDER_DIM,
    C_BORDER_FOCUS, C_GRAY, C_GREEN, C_PINK, C_RED, C_YELLOW, MENU_ITEMS, UNDO_MAX,
};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::ListState,
    Terminal,
};
use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

pub struct Editor {
    pub(crate) content: Vec<String>,
    pub(crate) sidebar_selected: usize,
    pub(crate) sidebar_state: ListState,
    pub(crate) content_scroll: u16,
    pub(crate) content_hscroll: u16,
    pub(crate) content_cursor: usize,
    pub(crate) content_col: usize,
    pub(crate) mode: EditorMode,
    pub(crate) input_prompt: String,
    pub(crate) input_buffer: String,
    pub(crate) file_path: PathBuf,
    pub(crate) assets_dir: PathBuf,
    pub(crate) status_message: Option<String>,
    pub(crate) should_quit: bool,
    pub(crate) picker_files: Vec<String>,
    pub(crate) picker_selected: usize,
    pub(crate) picker_target: usize,
    pub(crate) picker_title: String,
    pub(crate) scene_list: Vec<(String, String)>,
    pub(crate) scene_selected: usize,
    pub(crate) scene_title: String,
    pub(crate) story_files: Vec<String>,
    pub(crate) story_selected: usize,
    pub(crate) story_state: ListState,
    pub(crate) dialog_dir: PathBuf,
    pub(crate) run_test_requested: bool,
    pub(crate) build_requested: bool,
    pub(crate) undo_stack: Vec<(Vec<String>, usize, usize)>,
    pub(crate) content_area_width: u16,
    pub(crate) clipboard: Vec<String>,
    pub(crate) show_help: bool,
    pub(crate) menu_area: Rect,
    pub(crate) file_list_area: Rect,
    pub(crate) content_area: Rect,
    pub(crate) pending_stats: Option<StatInfo>,
}

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
            menu_area: Rect::default(),
            file_list_area: Rect::default(),
            content_area: Rect::default(),
            pending_stats: None,
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

    pub(crate) fn push_undo(&mut self) {
        if self.undo_stack.len() >= UNDO_MAX {
            self.undo_stack.remove(0);
        }
        self.undo_stack
            .push((self.content.clone(), self.content_cursor, self.content_col));
    }

    pub(crate) fn undo(&mut self) {
        if let Some((content, cursor, col)) = self.undo_stack.pop() {
            self.content = content;
            self.content_cursor = cursor.min(self.content.len().saturating_sub(1));
            self.content_col = col;
            self.clamp_col();
            self.adjust_hscroll();
            self.status_message = Some(format!("↶ 已撤销 (剩余 {} 步)", self.undo_stack.len()));
        } else {
            self.status_message = Some("没有可撤销的操作".to_string());
        }
    }

    pub(crate) fn load_story_files(&mut self) {
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

    pub(crate) fn switch_to_file(&mut self, filename: &str) {
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

    pub(crate) fn save(&mut self) {
        let text = self.content.join("\n");
        match fs::write(&self.file_path, text) {
            Ok(_) => {
                self.status_message = Some(format!("✓ 已保存 {}", self.file_path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("保存失败: {}", e));
            }
        }
    }

    pub(crate) fn create_file(&mut self, name: &str) {
        let mut name = name.trim().to_string();
        if name.is_empty() {
            self.status_message = Some("文件名不能为空".to_string());
            return;
        }
        if !name.ends_with(".ng") && !name.ends_with(".txt") {
            name.push_str(".ng");
        }
        let path = self.dialog_dir.join(&name);
        if path.exists() {
            self.status_message = Some(format!("文件已存在: {}", name));
            return;
        }
        let default_content = "[welcome]\n系统:这里是新的剧情文件。\nend\n";
        match fs::write(&path, default_content) {
            Ok(_) => {
                self.load_story_files();
                if let Some(idx) = self.story_files.iter().position(|f| f == &name) {
                    self.story_selected = idx;
                    self.story_state.select(Some(idx));
                }
                self.status_message = Some(format!("✓ 已创建 {}", name));
            }
            Err(e) => {
                self.status_message = Some(format!("创建失败: {}", e));
            }
        }
    }

    pub(crate) fn delete_file(&mut self) {
        if self.story_files.is_empty() {
            return;
        }
        let filename = self.story_files[self.story_selected].clone();
        let path = self.dialog_dir.join(&filename);
        match fs::remove_file(&path) {
            Ok(_) => {
                self.load_story_files();
                if self.story_selected >= self.story_files.len() && !self.story_files.is_empty() {
                    self.story_selected = self.story_files.len() - 1;
                }
                if !self.story_files.is_empty() {
                    self.story_state.select(Some(self.story_selected));
                }
                self.status_message = Some(format!("✓ 已删除 {}", filename));
            }
            Err(e) => {
                self.status_message = Some(format!("删除失败: {}", e));
            }
        }
    }

    pub(crate) fn rename_file(&mut self, old_name: &str, new_name: &str) {
        let mut new_name = new_name.trim().to_string();
        if new_name.is_empty() {
            self.status_message = Some("文件名不能为空".to_string());
            return;
        }
        if !new_name.ends_with(".ng") && !new_name.ends_with(".txt") {
            new_name.push_str(".ng");
        }
        if new_name == old_name {
            self.status_message = Some("文件名未改变".to_string());
            return;
        }
        let old_path = self.dialog_dir.join(old_name);
        let new_path = self.dialog_dir.join(&new_name);
        if new_path.exists() {
            self.status_message = Some(format!("目标文件已存在: {}", new_name));
            return;
        }
        match fs::rename(&old_path, &new_path) {
            Ok(_) => {
                self.load_story_files();
                if let Some(idx) = self.story_files.iter().position(|f| f == &new_name) {
                    self.story_selected = idx;
                    self.story_state.select(Some(idx));
                }
                if self
                    .file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s == old_name)
                    .unwrap_or(false)
                {
                    self.file_path = new_path;
                }
                self.status_message = Some(format!("✓ 已重命名为 {}", new_name));
            }
            Err(e) => {
                self.status_message = Some(format!("重命名失败: {}", e));
            }
        }
    }

    pub(crate) fn build_insert_line(&self) -> String {
        match self.sidebar_selected {
            0 => self.input_buffer.clone(),
            4 => format!("choose:{}", self.input_buffer),
            5 => format!("[{}]", self.input_buffer),
            7 => format!("$(command {})", self.input_buffer),
            _ => self.input_buffer.clone(),
        }
    }

    pub(crate) fn open_file_picker(&mut self, target: usize) {
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

    pub(crate) fn open_scene_picker(&mut self) {
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

    pub(crate) fn request_run_test(&mut self) {
        self.save();
        self.run_test_requested = true;
    }

    pub(crate) fn request_build(&mut self) {
        self.save();
        self.build_requested = true;
    }

    pub(crate) fn request_stats(&mut self) {
        self.save();
        self.pending_stats = Some(stats::compute_stats(self));
        self.mode = EditorMode::ShowStats;
    }

    pub(crate) fn cut_line(&mut self) {
        if self.content_cursor < self.content.len() {
            self.push_undo();
            let line = self.content.remove(self.content_cursor);
            self.clipboard = vec![line];
            if self.content_cursor >= self.content.len() && !self.content.is_empty() {
                self.content_cursor = self.content.len() - 1;
            }
            self.clamp_col();
            self.status_message = Some("✂ 已剪切当前行".to_string());
        } else {
            self.status_message = Some("没有可剪切的行".to_string());
        }
    }

    pub(crate) fn copy_line(&mut self) {
        if self.content_cursor < self.content.len() {
            self.clipboard = vec![self.content[self.content_cursor].clone()];
            self.status_message = Some("📋 已复制当前行".to_string());
        } else {
            self.status_message = Some("没有可复制的行".to_string());
        }
    }

    pub(crate) fn paste_clipboard(&mut self) {
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
        self.status_message = Some(format!("📋 已粘贴 {} 行", self.clipboard.len()));
    }

    pub(crate) fn clamp_col(&mut self) {
        if self.content_cursor < self.content.len() {
            let len = self.content[self.content_cursor].chars().count();
            if self.content_col > len {
                self.content_col = len;
            }
        } else {
            self.content_col = 0;
        }
    }

    pub(crate) fn adjust_hscroll(&mut self) {
        let view_width = self.content_area_width.saturating_sub(6);
        let cursor_pos = self.content_col as u16;

        if cursor_pos >= self.content_hscroll + view_width {
            self.content_hscroll = cursor_pos.saturating_sub(view_width) + 1;
        }
        if cursor_pos < self.content_hscroll {
            self.content_hscroll = cursor_pos;
        }
    }

    pub(crate) fn game_root(&self) -> PathBuf {
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

    pub(crate) fn contains(&self, rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
    }

    pub(crate) fn execute_menu_item(&mut self) {
        let idx = self.sidebar_selected;
        match idx {
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
            10 => self.request_stats(),
            11 => self.request_run_test(),
            12 => self.request_build(),
            13 => self.save(),
            14 => {
                self.mode = EditorMode::Shell {
                    buffer: String::new(),
                    output: vec!["输入命令，回车执行，ESC 退出".to_string(), String::new()],
                };
            }
            15 => self.undo(),
            16 => {
                self.save();
                self.should_quit = true;
            }
            _ => {}
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
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_event(key.code, key.modifiers);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    _ => {}
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

    pub(crate) fn spawn_test_game(&self) {
        let root = self.game_root();
        println!("正在启动测试: {}", root.display());
        if !root.exists() || !root.join("assets/game.json").exists() {
            println!("❌ 目录无效: {}", root.display());
            println!("按回车返回编辑器...");
            let _ = std::io::stdin().read_line(&mut String::new());
            return;
        }
        println!("按 ESC 或 q 退出游戏返回编辑器...\n");

        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ngal"));
        match std::process::Command::new(exe).arg(&root).status() {
            Ok(_) => println!("\n游戏已退出"),
            Err(e) => println!("\n无法启动游戏: {}", e),
        }
    }

    pub(crate) fn spawn_build(&self) {
        let root = self.game_root();
        println!("正在打包: {}\n", root.display());
        if !root.exists() || !root.join("assets/game.json").exists() {
            println!("❌ 目录无效: {}", root.display());
            return;
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ngal"));
        match std::process::Command::new(exe)
            .arg("build")
            .arg(&root)
            .status()
        {
            Ok(_) => println!("\n✅ 打包完成"),
            Err(e) => println!("\n无法启动打包: {}", e),
        }
    }
}
