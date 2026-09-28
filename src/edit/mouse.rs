// src/edit/mouse.rs
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::state::{EditorMode, MENU_ITEMS};
use super::Editor;

impl Editor {
    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return;
        }

        let col = mouse.column;
        let row = mouse.row;

        // 帮助弹窗打开时，任意点击关闭
        if self.show_help {
            self.show_help = false;
            return;
        }

        // 统计弹窗：任意点击关闭
        if matches!(self.mode, EditorMode::ShowStats) {
            self.pending_stats = None;
            self.mode = EditorMode::Normal;
            return;
        }

        match &self.mode {
            EditorMode::Normal => self.handle_mouse_normal(col, row),
            EditorMode::FileListFocus => self.handle_mouse_filelist(col, row),
            EditorMode::DirectEdit => self.handle_mouse_edit(col, row),
            _ => {}
        }
    }

    fn handle_mouse_normal(&mut self, col: u16, row: u16) {
        // 点击功能菜单
        if self.contains(self.menu_area, col, row) {
            let inner_y = self.menu_area.y + 1;
            if row >= inner_y {
                let idx = (row - inner_y) as usize;
                if idx < MENU_ITEMS.len() && MENU_ITEMS[idx] != "────────────────" {
                    if idx == self.sidebar_selected {
                        // 已在选中项上 → 执行
                        self.execute_menu_item();
                    } else {
                        // 切换选中（需要再次点击才执行）
                        self.sidebar_selected = idx;
                        self.sidebar_state.select(Some(idx));
                    }
                }
            }
            return;
        }

        // 点击剧情文件列表
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

        // 点击右侧内容区
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
                        // 已在选中项上 → 打开文件
                        if let Some(filename) = self.story_files.get(idx).cloned() {
                            self.switch_to_file(&filename);
                        }
                    } else {
                        // 切换选中
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
        // 编辑模式下点击左侧，切换到相应焦点
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

        // 点击内容区：定位光标
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