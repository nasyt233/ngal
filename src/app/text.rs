// src/app/text.rs
use std::time::{Duration, Instant};
use crate::parser::DialogueCommand;
use super::{App, AppState};

impl App {
    // ==================== 变量插值 + 命令执行 ====================

    pub(crate) fn interpolate_text(&self, text: &str) -> String {
        use std::sync::OnceLock;
        static SHELL_RE: OnceLock<regex::Regex> = OnceLock::new();
        let re = SHELL_RE.get_or_init(|| regex::Regex::new(r"\$\(([^)]+)\)").unwrap());

        let mut result = self.variables.interpolate(text);

        while let Some(caps) = re.captures(&result) {
            let full_match = caps.get(0).unwrap().as_str().to_string();
            let cmd = caps.get(1).unwrap().as_str().trim().to_string();

            #[cfg(unix)]
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output();

            #[cfg(windows)]
            let output = std::process::Command::new("cmd")
                .arg("/C")
                .arg(&cmd)
                .output();

            let replacement = match output {
                Ok(out) => {
                    if out.status.success() {
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    } else {
                        format!("[命令执行失败: {}]", cmd)
                    }
                }
                Err(e) => format!("[命令错误: {}]", e),
            };

            result = result.replace(&full_match, &replacement);
        }

        result
    }

    // ==================== 文字动画 ====================

    pub fn update_animation(&mut self) {
        if !self.config.text_animation {
            if self.display_text != self.target_text {
                self.display_text = self.target_text.clone();
            }
            return;
        }

        if self.target_text.is_empty() {
            return;
        }

        if self.display_text.len() < self.target_text.len() {
            let elapsed = self.last_char_time.elapsed();
            let speed = Duration::from_millis(self.config.text_speed);
            if elapsed >= speed {
                let add_count = (elapsed.as_millis() / speed.as_millis()).max(1) as usize;
                for _ in 0..add_count {
                    let current_len = self.display_text.len();
                    if current_len < self.target_text.len() {
                        let next_char = self.target_text[current_len..].chars().next();
                        if let Some(c) = next_char {
                            let char_len = c.len_utf8();
                            self.display_text
                                .push_str(&self.target_text[current_len..current_len + char_len]);
                        } else {
                            break;
                        }
                    }
                }
                self.last_char_time = Instant::now();
            }
        }
    }

    // ==================== 当前对话信息 ====================

    pub fn current_speaker(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    // 从当前位置往前找最近的 Text 命令（Sleep 期间显示上一条）
                    for i in (0..=*cmd_index).rev() {
                        if let Some(DialogueCommand::Text { speaker, .. }) =
                            scene.commands.get(i)
                        {
                            if let Some(s) = speaker {
                                return Some(self.variables.interpolate(s));
                            }
                            return None;
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn current_text(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, cmd_index } => {
                if let Some(scene) = self.scenes.get(scene_id) {
                    if let Some(DialogueCommand::Text { text, .. }) =
                        scene.commands.get(*cmd_index)
                    {
                        return Some(self.interpolate_text(text));
                    }
                }
                None
            }
            _ => None,
        }
    }

    // ==================== 自动播放 ====================

    pub fn update_auto_play(&mut self) {
        if self.config.auto_play {
            if let Some(timer) = self.auto_play_timer {
                if timer.elapsed() >= Duration::from_secs_f64(self.config.auto_play_speed) {
                    match self.state {
                        AppState::InDialogue { .. } => {
                            self.advance_dialogue();
                            self.auto_play_timer = Some(Instant::now());
                        }
                        _ => self.auto_play_timer = None,
                    }
                }
            }
        }
    }
    /// 每帧调用：检查 sleep 计时器是否到期
    pub fn tick_sleep(&mut self) {
        if let Some(until) = self.sleep_until {
            if Instant::now() >= until {
                self.sleep_until = None;
                self.advance_dialogue();
            }
        }
    }
}