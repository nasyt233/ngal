// src/app/save_load.rs
use crate::audio;
use crate::save::SaveData;
use super::{App, AppState};

impl App {
    pub fn save_game_slot(&mut self, slot: usize) {
        let save_state = if let Some(prev) = &self.prev_state {
            prev.as_ref().clone()
        } else {
            self.state.clone()
        };
        let image_params = self.current_image_params.clone();
        let background = self.current_background.clone();
        let bgm = self.current_bgm.clone();
        let current_file = self.current_file.clone().and_then(|f| {
            if f == "null" || f.is_empty() {
                None
            } else {
                Some(f)
            }
        });

        if let Err(e) = SaveData::save(
            slot,
            &save_state,
            self.selected,
            &self.variables,
            current_file,
            background,
            bgm,
            image_params,
        ) {
            self.status_message = Some(format!("存档失败: {}", e));
        } else {
            self.status_message = Some(format!("已存档到槽位 {}", slot));
        }

        if let Some(prev) = self.prev_state.take() {
            self.state = *prev;
        } else {
            self.state = AppState::Menu;
        }
    }

    pub fn load_game_slot(&mut self, slot: usize) {
        match SaveData::load(slot) {
            Ok(data) => {
                self.state = data.state;
                self.selected = data.menu_selected;
                self.variables.deserialize(data.variables);
                self.current_image_params = data.image_params;
                self.current_background = data.background;

                if let Some(file) = data.current_file {
                    if file == "null" || file.is_empty() {
                        self.current_file = None;
                    } else {
                        self.current_file = Some(file.clone());
                        if !self.file_scene_order.contains_key(&file) {
                            let _ = self.load_external_file(&file);
                        }
                        if let AppState::InDialogue { scene_id, .. } = &self.state {
                            if !self.scenes.contains_key(scene_id) {
                                let _ = self.load_external_file(&file);
                            }
                        }
                    }
                } else {
                    self.current_file = None;
                }

                if self.current_file.is_none() {
                    if let AppState::InDialogue { scene_id, .. } = &self.state {
                        if self.scenes.contains_key(scene_id) {
                            self.current_file = Some("dialogue.ng".to_string());
                        }
                    }
                }

                if let Some(bgm) = &data.bgm {
                    self.current_bgm = Some(bgm.clone());
                    self.play_bgm(bgm);
                } else {
                    self.current_bgm = None;
                    audio::stop_bgm();
                }

                self.status_message = Some(format!("从槽位 {} 读档成功", slot));
                self.prev_state = None;

                if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(cmd) = scene.commands.get(*cmd_index) {
                            self.execute_command(cmd.clone());
                        }
                    }
                }
            }
            Err(e) => {
                self.status_message = Some(format!("读档失败: {}", e));
                if let Some(prev) = self.prev_state.take() {
                    self.state = *prev;
                } else {
                    self.state = AppState::Menu;
                    self.play_title_bgm();
                }
            }
        }
    }

    pub fn open_save_slot(&mut self) {
        self.prev_state = Some(Box::new(self.state.clone()));
        self.selected = 0;
        self.state = AppState::SaveSlot;
    }

    pub fn open_load_slot(&mut self) {
        self.prev_state = Some(Box::new(self.state.clone()));
        self.selected = 0;
        self.state = AppState::LoadSlot;
    }
}