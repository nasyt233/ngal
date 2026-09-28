// src/app/event.rs
use crossterm::event::KeyCode;
use std::time::Instant;
use crate::save::SaveData;
use super::{App, AppState};

impl App {
    // ==================== 游戏内菜单 ====================

    pub(crate) fn handle_game_menu(&mut self, key: KeyCode) {
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
                }
            }
            KeyCode::Char('2') => {
                self.open_save_slot();
            }
            KeyCode::Char('3') => {
                self.open_load_slot();
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
                    }
                }
                1 => self.open_save_slot(),
                2 => self.open_load_slot(),
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

    // ==================== 事件处理 ====================

    pub fn handle_event(&mut self, key: KeyCode) {
        self.status_message = None;

        // ---------- 第一段：需要提前 return 的状态 ----------
        match self.state {
            AppState::History => {
                let len = self.history.len();
                match key {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.history_selected > 0 {
                            self.history_selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if len > 0 && self.history_selected + 1 < len {
                            self.history_selected += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        self.history_selected = self.history_selected.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        if len > 0 {
                            self.history_selected = (self.history_selected + 10).min(len - 1);
                        }
                    }
                    KeyCode::Home => {
                        self.history_selected = 0;
                    }
                    KeyCode::End => {
                        if len > 0 {
                            self.history_selected = len - 1;
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
                let total = SaveData::next_empty_slot();
                match key {
                    KeyCode::Up => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if total > 0 && self.selected < total - 1 {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.save_game_slot(self.selected + 1);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if let Some(slot) = c.to_digit(10) {
                            let slot = slot as usize;
                            if slot >= 1 {
                                self.save_game_slot(slot);
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            AppState::LoadSlot => {
                let valid_slots: Vec<usize> = SaveData::list_slots();
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
                    KeyCode::Esc | KeyCode::Char('q') => {
                        if let Some(prev) = self.prev_state.take() {
                            self.state = *prev;
                        } else {
                            self.state = AppState::Menu;
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if let Some(slot) = c.to_digit(10) {
                            let slot = slot as usize;
                            if slot >= 1 && SaveData::exists(slot) {
                                self.load_game_slot(slot);
                                return;
                            }
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
                self.state = AppState::Menu;
                self.play_title_bgm();
                return;
            }
            _ => {}
        }

        // ---------- 第二段：菜单、设置、对话、选择 ----------
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
                        self.history_selected = self.history.len().saturating_sub(1);
                        self.state = AppState::History;
                    }
                    _ => {}
                }
                return;
            }
            AppState::Settings => {
                match key {
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.handle_settings(super::SettingsAction::BgmUp)
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        self.handle_settings(super::SettingsAction::BgmDown)
                    }
                    KeyCode::Char('[') => {
                        self.handle_settings(super::SettingsAction::VoiceDown)
                    }
                    KeyCode::Char(']') => {
                        self.handle_settings(super::SettingsAction::VoiceUp)
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.handle_settings(super::SettingsAction::AutoPlayToggle)
                    }
                    KeyCode::Char('1') => {
                        self.handle_settings(super::SettingsAction::AutoPlaySpeedDown)
                    }
                    KeyCode::Char('2') => {
                        self.handle_settings(super::SettingsAction::AutoPlaySpeedUp)
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        self.handle_settings(super::SettingsAction::TextAnimationToggle)
                    }
                    KeyCode::Char('3') => {
                        self.handle_settings(super::SettingsAction::TextSpeedDown)
                    }
                    KeyCode::Char('4') => {
                        self.handle_settings(super::SettingsAction::TextSpeedUp)
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        self.handle_settings(super::SettingsAction::BgColorNext)
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.handle_settings(super::SettingsAction::Save)
                    }
                    KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Menu,
                    _ => {}
                }
                return;
            }
            AppState::InDialogue { .. } => {
                match key {
                    KeyCode::Left => {
                        self.rewind_dialogue();
                    }
                    KeyCode::Right => {
                        self.advance_dialogue();
                        if self.config.auto_play {
                            self.auto_play_timer = Some(Instant::now());
                        }
                    }
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
                        self.history_selected = self.history.len().saturating_sub(1);
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
                KeyCode::Left => {
                    self.rewind_dialogue();
                    return;
                }
                KeyCode::Right => {
                    self.select_option();
                    return;
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.prev_state = Some(Box::new(self.state.clone()));
                    self.history_selected = self.history.len().saturating_sub(1);
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

        // ---------- 第三段：InChoice 上下选择 ----------
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
}