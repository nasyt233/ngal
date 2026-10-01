// src/app/script.rs
use anyhow::Result;
use crate::parser::{self, DialogueCommand};
use crate::variables::Variables;
use super::{App, AppState};
use std::time::{Duration, Instant};

impl App {
    // ==================== 执行命令 ====================

    pub fn execute_command(&mut self, cmd: DialogueCommand) {
        match cmd {
            DialogueCommand::Text { speaker, text, voice } => {
                let interpolated = self.interpolate_text(&text);
                self.target_text = interpolated.clone();
                self.display_text = String::new();
                self.last_char_time = std::time::Instant::now();
                if !self.target_text.is_empty() {
                    let first_char = self.target_text.chars().next().unwrap();
                    let char_len = first_char.len_utf8();
                    self.display_text.push_str(&self.target_text[0..char_len]);
                }

                let interpolated_speaker =
                    speaker.as_ref().map(|s| self.variables.interpolate(s));
                let final_speaker = interpolated_speaker.as_deref();

                if let Some(s) = final_speaker {
                    self.add_to_history(Some(s), &interpolated);
                } else {
                    self.add_to_history(None, &interpolated);
                }

                if let Some(v) = voice {
                    self.play_voice_by_file(final_speaker.unwrap_or(""), Some(&v));
                } else if let Some(s) = final_speaker {
                    self.play_voice_by_file(s, None);
                }
            }
            DialogueCommand::Sleep { seconds } => {
                let dur = Duration::from_secs_f64(seconds);
                self.sleep_until = Some(Instant::now() + dur);
            }
            DialogueCommand::Image(params) => {
                self.current_image_params = Some(params);
            }
            DialogueCommand::Background { filename } => {
                self.current_background = filename;
            }
            DialogueCommand::Music { filename } => {
                self.current_bgm = Some(filename.clone());
                self.play_bgm(&filename);
            }
            DialogueCommand::MusicStop => {
                self.stop_bgm();
            }
            DialogueCommand::Choose { options } => {
                if let AppState::InDialogue { scene_id, .. } = &self.state {
                    self.state = AppState::InChoice {
                        scene_id: scene_id.clone(),
                        options,
                        selected: 0,
                    };
                }
            }
            DialogueCommand::Load { file, target } => {
                if let Some(file_name) = file {
                    if file_name == "null" || file_name.is_empty() {
                        // 不改变 current_file
                    } else {
                        self.current_file = Some(file_name.clone());
                        if !self.file_scene_order.contains_key(&file_name) {
                            let _ = self.load_external_file(&file_name);
                        }
                        if !self.scenes.contains_key(&target) {
                            let _ = self.load_external_file(&file_name);
                        }
                    }
                }
                self.state = AppState::InDialogue {
                    scene_id: target,
                    cmd_index: 0,
                };
                if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                    if let Some(scene) = self.scenes.get(scene_id) {
                        if let Some(first_cmd) = scene.commands.get(*cmd_index) {
                            self.execute_command(first_cmd.clone());
                        }
                    }
                }
                self.skip_non_interactive_commands();
            }
            DialogueCommand::End => {
                self.state = AppState::Menu;
                self.current_image_params = None;
                self.current_background = None;
                self.current_file = None;
                self.stop_bgm();
                self.play_title_bgm();
            }
            DialogueCommand::Input { prompt, var_name } => {
                self.prev_state = Some(Box::new(self.state.clone()));
                self.target_text = String::new();
                self.display_text = String::new();
                self.input_buffer = String::new();
                self.state = AppState::Input { prompt, var_name };
            }
            DialogueCommand::SetVar { name, value } => {
                match self.variables.eval_expr(&value) {
                    Some(computed) => {
                        self.variables.set(&name, &computed);
                    }
                    None => {
                        let interpolated = self.interpolate_text(&value);
                        self.variables.set(&name, &interpolated);
                    }
                }
                self.advance_dialogue();
            }
            DialogueCommand::If { condition, target } => {
                if self.variables.eval_condition(&condition) {
                    self.state = AppState::InDialogue {
                        scene_id: target,
                        cmd_index: 0,
                    };
                    if let AppState::InDialogue { scene_id, cmd_index } = &self.state {
                        if let Some(scene) = self.scenes.get(scene_id) {
                            if let Some(first_cmd) = scene.commands.get(*cmd_index) {
                                self.execute_command(first_cmd.clone());
                            }
                        }
                    }
                    self.skip_non_interactive_commands();
                }
            }
        }
    }

    // ==================== 跳过非交互命令 ====================

    pub fn skip_non_interactive_commands(&mut self) {
        loop {
            let (scene_id, cmd_index) = match &self.state {
                AppState::InDialogue { scene_id, cmd_index } => (scene_id.clone(), *cmd_index),
                _ => break,
            };

            let next = {
                let scene = match self.scenes.get(&scene_id) {
                    Some(s) => s,
                    None => break,
                };
                let cmd = match scene.commands.get(cmd_index) {
                    Some(c) => c,
                    None => break,
                };

                let skippable = matches!(
                    cmd,
                    DialogueCommand::Image { .. }
                        | DialogueCommand::Background { .. }
                        | DialogueCommand::Music { .. }
                        | DialogueCommand::MusicStop
                        | DialogueCommand::SetVar { .. }
                        | DialogueCommand::If { .. }
                );
                if !skippable {
                    break;
                }
                scene.commands.get(cmd_index + 1).cloned()
            };

            let next_cmd = match next {
                Some(c) => c,
                None => break,
            };

            self.state = AppState::InDialogue {
                scene_id,
                cmd_index: cmd_index + 1,
            };
            self.execute_command(next_cmd);
        }
    }

    // ==================== 开始游戏 ====================

    // ==================== 开始游戏 ====================

    pub fn start_game(&mut self) {
        // ========== 完全重置游戏状态 ==========
        self.snapshot_stack.clear();
        self.history.clear();
        self.history_selected = 0;
        self.selected = 0;
        self.auto_play_timer = None;
        self.status_message = None;
        self.prev_state = None;
    
        self.target_text.clear();
        self.display_text.clear();
        self.input_buffer.clear();
        self.current_bgm = None;
        self.current_image_params = None;
        self.current_background = None;
        self.current_file = None;
    
        self.stop_bgm();
        self.stop_voice();
    
        self.variables = Variables::new();
    
        // ========== 清空并重新加载主剧情 ==========
        self.scenes.clear();
        self.file_scene_order.clear();
    
        match parser::load_dialogue() {
            Ok(content) => match parser::parse_dialogue_file_with_order(&content) {
                Ok((scenes, order)) => {
                    self.scenes = scenes;
                    self.file_scene_order.insert("dialogue.ng".to_string(), order);
                    self.current_file = Some("dialogue.ng".to_string());
                }
                Err(e) => {
                    self.state = AppState::Menu;
                    self.status_message = Some(format!("解析剧本失败: {}", e));
                    return;
                }
            },
            Err(e) => {
                self.state = AppState::Menu;
                self.status_message = Some(format!("加载剧本失败: {}", e));
                return;
            }
        }
    
        // ========== 从 welcome 开始 ==========
        let initial_scene = "welcome".to_string();
        if self.scenes.contains_key(&initial_scene) {
            let scene_id = initial_scene.clone();
            self.state = AppState::InDialogue {
                scene_id: scene_id.clone(),
                cmd_index: 0,
            };
            if let Some(scene) = self.scenes.get(&scene_id) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            self.skip_non_interactive_commands();
        } else {
            self.state = AppState::Menu;
            self.status_message = Some("未找到起始场景 welcome".to_string());
        }
    }
    
    // ==================== 加载外部文件 ====================

    pub fn load_external_file(&mut self, file_name: &str) -> Result<()> {
        let path = format!("assets/dialog/{}", file_name);
        let content = crate::assets::read_text(&path)
            .ok_or_else(|| anyhow::anyhow!("找不到文件: {}", path))?;
        let (new_scenes, scene_order) = parser::parse_dialogue_file_with_order(&content)?;
        for (k, v) in new_scenes {
            self.scenes.insert(k, v);
        }
        self.file_scene_order
            .insert(file_name.to_string(), scene_order);
        Ok(())
    }

    // ==================== 推进对话 ====================

    pub fn advance_dialogue(&mut self) {
        if matches!(self.state, AppState::InDialogue { .. }) {
            self.push_snapshot();
        }

        let (current_scene_id, current_cmd_index) = match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => (scene_id.clone(), *cmd_index),
            _ => return,
        };

        let scene = match self.scenes.get(&current_scene_id) {
            Some(s) => s,
            None => {
                self.state = AppState::Menu;
                return;
            }
        };

        let next_cmd_index = current_cmd_index + 1;
        if let Some(next_cmd) = scene.commands.get(next_cmd_index) {
            self.state = AppState::InDialogue {
                scene_id: current_scene_id,
                cmd_index: next_cmd_index,
            };
            self.execute_command(next_cmd.clone());
            self.skip_non_interactive_commands();
        } else {
            let next_scene = if let Some(file) = &self.current_file {
                if let Some(order) = self.file_scene_order.get(file) {
                    if let Some(pos) = order.iter().position(|s| s == &current_scene_id) {
                        if pos + 1 < order.len() {
                            Some(order[pos + 1].clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(next_scene_id) = next_scene {
                let next_scene_id_clone = next_scene_id.clone();
                self.target_text = String::new();
                self.display_text = String::new();
                self.state = AppState::InDialogue {
                    scene_id: next_scene_id_clone,
                    cmd_index: 0,
                };
                if let Some(scene) = self.scenes.get(&next_scene_id) {
                    if let Some(first_cmd) = scene.commands.first() {
                        self.execute_command(first_cmd.clone());
                    }
                }
                self.skip_non_interactive_commands();
            } else {
                self.state = AppState::EndOfFile;
                self.current_image_params = None;
                self.current_background = None;
                self.status_message = Some("剧情结束，按任意键返回主菜单".to_string());
            }
        }
    }

    // ==================== 选项选择 ====================

    pub fn select_option(&mut self) {
        let (options, selected, _current_scene_id) = match &self.state {
            AppState::InChoice { options, selected, scene_id } => {
                (options.clone(), *selected, scene_id.clone())
            }
            _ => return,
        };

        if let Some((_, next_scene)) = options.get(selected) {
            self.push_snapshot();

            self.target_text = String::new();
            self.display_text = String::new();
            self.state = AppState::InDialogue {
                scene_id: next_scene.clone(),
                cmd_index: 0,
            };
            if let Some(scene) = self.scenes.get(next_scene) {
                if let Some(first_cmd) = scene.commands.first() {
                    self.execute_command(first_cmd.clone());
                }
            }
            self.skip_non_interactive_commands();
        }
    }
}