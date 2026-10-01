// src/edit/mouse.rs
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::state::{EditorMode, MENU_ITEMS};
use super::Editor;

impl Editor {
    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        // 滚轮事件
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.handle_scroll(-1);
                return;
            }
            MouseEventKind::ScrollDown => {
                self.handle_scroll(1);
                return;
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // 继续走点击处理
            }
            _ => return,
        }

        let col = mouse.column;
        let row = mouse.row;

        if self.show_help {
            self.show_help = false;
            return;
        }

        if matches!(self.mode, EditorMode::ShowStats) {
            self.pending_stats = None;
            self.mode = EditorMode::Normal;
            return;
        }

        // 补全激活时点击任何地方关闭
        if self.completion_active {
            self.completion_active = false;
        }

        match &self.mode {
            EditorMode::Normal => self.handle_mouse_normal(col, row),
            EditorMode::FileListFocus => self.handle_mouse_filelist(col, row),
            EditorMode::DirectEdit => self.handle_mouse_edit(col, row),
            _ => {}
        }
    }

    fn handle_scroll(&mut self, delta: i32) {
        match &self.mode {
            EditorMode::Normal | EditorMode::DirectEdit | EditorMode::FileListFocus => {
                if delta < 0 {
                    self.content_scroll = self.content_scroll.saturating_sub((-delta) as u16);
                } else {
                    let max = self.content.len().saturating_sub(1) as u16;
                    self.content_scroll = (self.content_scroll + delta as u16).min(max);
                }
            }
            EditorMode::FilePicker => {
                if delta < 0 {
                    if self.picker_selected > 0 {
                        self.picker_selected -= 1;
                    }
                } else if !self.picker_files.is_empty()
                    && self.picker_selected < self.picker_files.len() - 1
                {
                    self.picker_selected += 1;
                }
            }
            EditorMode::ScenePicker => {
                if delta < 0 {
                    if self.scene_selected > 0 {
                        self.scene_selected -= 1;
                    }
                } else if !self.scene_list.is_empty()
                    && self.scene_selected < self.scene_list.len() - 1
                {
                    self.scene_selected += 1;
                }
            }
            EditorMode::CharacterPicker => {
                if delta < 0 {
                    if self.character_selected > 0 {
                        self.character_selected -= 1;
                    }
                } else if !self.character_list.is_empty()
                    && self.character_selected < self.character_list.len() - 1
                {
                    self.character_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_normal(&mut self, col: u16, row: u16) {
        if self.contains(self.menu_area, col, row) {
            let inner_y = self.menu_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < MENU_ITEMS.len() && MENU_ITEMS[idx] != "────────────────" {
                    if idx == self.sidebar_selected {
                        self.execute_menu_item();
                    } else {
                        self.sidebar_selected = idx;
                        self.sidebar_state.select(Some(idx));
                    }
                }
            }
            return;
        }

        if self.contains(self.file_list_area, col, row) {
            let inner_y = self.file_list_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < self.story_files.len() {
                    self.story_selected = idx;
                    self.story_state.select(Some(idx));
                    self.mode = EditorMode::FileListFocus;
                }
            }
            return;
        }

        if self.contains(self.content_area, col, row) {
            let inner_y = self.content_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize + self.content_scroll as usize;
                if idx < self.content.len() {
                    self.content_cursor = idx;
                    self.clamp_col();
                }
            }
            self.mode = EditorMode::DirectEdit;
        }
    }

    fn handle_mouse_filelist(&mut self, col: u16, row: u16) {
        if self.contains(self.file_list_area, col, row) {
            let inner_y = self.file_list_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < self.story_files.len() {
                    if idx == self.story_selected {
                        if let Some(filename) = self.story_files.get(idx).cloned() {
                            self.switch_to_file(&filename);
                        }
                    } else {
                        self.story_selected = idx;
                        self.story_state.select(Some(idx));
                    }
                }
            }
            return;
        }

        if self.contains(self.menu_area, col, row) {
            self.mode = EditorMode::Normal;
            return;
        }

        if self.contains(self.content_area, col, row) {
            let inner_y = self.content_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize + self.content_scroll as usize;
                if idx < self.content.len() {
                    self.content_cursor = idx;
                    self.clamp_col();
                }
            }
            self.mode = EditorMode::DirectEdit;
        }
    }

    fn handle_mouse_edit(&mut self, col: u16, row: u16) {
        if self.contains(self.menu_area, col, row) {
            self.mode = EditorMode::Normal;
            let inner_y = self.menu_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < MENU_ITEMS.len() && MENU_ITEMS[idx] != "────────────────" {
                    if idx == self.sidebar_selected {
                        self.execute_menu_item();
                    } else {
                        self.sidebar_selected = idx;
                        self.sidebar_state.select(Some(idx));
                    }
                }
            }
            return;
        }

        if self.contains(self.file_list_area, col, row) {
            self.mode = EditorMode::FileListFocus;
            let inner_y = self.file_list_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < self.story_files.len() {
                    self.story_selected = idx;
                    self.story_state.select(Some(idx));
                }
            }
            return;
        }

        if self.contains(self.content_area, col, row) {
            let inner_y = self.content_area.y + 1;
            let inner_x = self.content_area.x + 1;
            if row >= inner_y && col >= inner_x {
                let line_idx = (row - inner_y) as usize + self.content_scroll as usize;
                if line_idx < self.content.len() {
                    self.content_cursor = line_idx;
                    let line_num_width = if self.content.len() < 10 {
                        1
                    } else if self.content.len() < 100 {
                        2
                    } else {
                        3
                    };
                    let prefix_width = line_num_width + 3;
                    let col_in_line = if (col as usize) > inner_x as usize + prefix_width {
                        (col as usize) - inner_x as usize - prefix_width
                    } else {
                        0
                    };
                    self.content_col = col_in_line;
                    self.clamp_col();
                    self.adjust_hscroll();
                }
            }
        }
    }
}