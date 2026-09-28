// src/edit/keys.rs
use crossterm::event::{KeyCode, KeyModifiers};

use super::shell::execute_shell_command;
use super::state::{EditorMode, FileNameAction};
use super::Editor;

impl Editor {
    pub(crate) fn handle_event(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match &self.mode {
            EditorMode::Normal => self.handle_normal(key, modifiers),
            EditorMode::FileListFocus => self.handle_file_list_focus(key),
            EditorMode::Input => self.handle_input(key),
            EditorMode::FilePicker => self.handle_file_picker(key),
            EditorMode::ScenePicker => self.handle_scene_picker(key),
            EditorMode::DirectEdit => self.handle_direct_edit(key, modifiers),
            EditorMode::ConfirmDelete => self.handle_confirm_delete(key),
            EditorMode::FileNameInput { .. } => self.handle_file_name_input(key),
            EditorMode::Shell { .. } => self.handle_shell(key),
            EditorMode::ShowStats => {
                self.pending_stats = None;
                self.mode = EditorMode::Normal;
            }
        }
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
                self.status_message = Some("✓ 已插入".to_string());
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
                    self.status_message = Some(format!("✓ 已插入 {}", filename));
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
                    self.status_message = Some(format!("✓ 已插入 {}", scene_name));
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
                self.story_selected = self.story_selected.saturating_sub(10);
                self.story_state.select(Some(self.story_selected));
            }
            KeyCode::PageDown => {
                if !self.story_files.is_empty() {
                    if self.story_selected + 10 < self.story_files.len() {
                        self.story_selected += 10;
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
                self.status_message = Some("↻ 已刷新文件列表".to_string());
            }
            KeyCode::Char('i') => {
                self.mode = EditorMode::DirectEdit;
                self.content_col = 0;
                self.status_message = Some("编辑模式".to_string());
            }
            KeyCode::Char('h') => {
                self.show_help = true;
            }
            KeyCode::Char('n') => {
                self.mode = EditorMode::FileNameInput {
                    action: FileNameAction::Create,
                    buffer: String::new(),
                };
            }
            KeyCode::Char('d') => {
                if !self.story_files.is_empty() {
                    self.mode = EditorMode::ConfirmDelete;
                }
            }
            KeyCode::Char('m') => {
                if let Some(name) = self.story_files.get(self.story_selected).cloned() {
                    self.mode = EditorMode::FileNameInput {
                        action: FileNameAction::Rename(name),
                        buffer: String::new(),
                    };
                }
            }
            _ => {}
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                self.delete_file();
                self.mode = EditorMode::FileListFocus;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = EditorMode::FileListFocus;
                self.status_message = Some("已取消删除".to_string());
            }
            _ => {}
        }
    }

    fn handle_file_name_input(&mut self, key: KeyCode) {
        let mut buffer = match &self.mode {
            EditorMode::FileNameInput { buffer, .. } => buffer.clone(),
            _ => return,
        };

        match key {
            KeyCode::Enter => {
                let name = buffer.trim().to_string();
                if let EditorMode::FileNameInput { action, .. } = &self.mode {
                    match action {
                        FileNameAction::Create => {
                            self.mode = EditorMode::FileListFocus;
                            self.create_file(&name);
                        }
                        FileNameAction::Rename(old) => {
                            let old = old.clone();
                            self.mode = EditorMode::FileListFocus;
                            self.rename_file(&old, &name);
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.mode = EditorMode::FileListFocus;
                self.status_message = Some("已取消".to_string());
            }
            KeyCode::Backspace => {
                buffer.pop();
                if let EditorMode::FileNameInput { buffer: b, .. } = &mut self.mode {
                    *b = buffer;
                }
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                if let EditorMode::FileNameInput { buffer: b, .. } = &mut self.mode {
                    *b = buffer;
                }
            }
            _ => {}
        }
    }

    fn handle_shell(&mut self, key: KeyCode) {
        let mut buffer = match &self.mode {
            EditorMode::Shell { buffer, .. } => buffer.clone(),
            _ => return,
        };

        match key {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已退出终端".to_string());
            }
            KeyCode::Enter => {
                let cmd = buffer.trim().to_string();
                if !cmd.is_empty() {
                    let output = execute_shell_command(&cmd);
                    if let EditorMode::Shell {
                        output: out,
                        buffer: b,
                    } = &mut self.mode
                    {
                        out.push(format!("$ {}", cmd));
                        for line in output.lines() {
                            out.push(line.to_string());
                        }
                        out.push(String::new());
                        b.clear();
                    }
                }
            }
            KeyCode::Backspace => {
                buffer.pop();
                if let EditorMode::Shell { buffer: b, .. } = &mut self.mode {
                    *b = buffer;
                }
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                if let EditorMode::Shell { buffer: b, .. } = &mut self.mode {
                    *b = buffer;
                }
            }
            _ => {}
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
            KeyCode::Tab | KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = Some("已切换到菜单模式".to_string());
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
                self.content_cursor = self.content_cursor.saturating_sub(10);
                self.clamp_col();
                self.adjust_hscroll();
                if self.content_cursor < self.content_scroll as usize {
                    self.content_scroll = self.content_cursor as u16;
                }
            }
            KeyCode::PageDown => {
                if self.content_cursor + 10 < self.content.len() {
                    self.content_cursor += 10;
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

    fn handle_normal(&mut self, key: KeyCode, modifiers: KeyModifiers) {
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
                    Some("文件列表 | Enter打开 | n新建 d删除 m重命名".to_string());
            }
            KeyCode::Char('q') => {
                self.save();
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                    self.sidebar_state.select(Some(self.sidebar_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sidebar_selected < super::state::MENU_ITEMS.len() - 1 {
                    self.sidebar_selected += 1;
                    self.sidebar_state.select(Some(self.sidebar_selected));
                }
            }
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
            KeyCode::Home => {
                self.sidebar_selected = 0;
                self.sidebar_state.select(Some(0));
            }
            KeyCode::End => {
                self.sidebar_selected = super::state::MENU_ITEMS.len() - 1;
                self.sidebar_state.select(Some(self.sidebar_selected));
            }
            KeyCode::PageUp => {
                self.content_cursor = self.content_cursor.saturating_sub(10);
                self.clamp_col();
                self.adjust_hscroll();
                if self.content_cursor < self.content_scroll as usize {
                    self.content_scroll = self.content_cursor as u16;
                }
            }
            KeyCode::PageDown => {
                if self.content_cursor + 10 < self.content.len() {
                    self.content_cursor += 10;
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
                self.status_message = Some("编辑模式 | Tab/ESC退出".to_string());
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
            KeyCode::Enter => {
                self.execute_menu_item();
            }
            _ => {}
        }
    }
}