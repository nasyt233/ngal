// src/app/settings.rs
use std::time::Instant;
use crate::audio;
use super::{App, SettingsAction};

impl App {
    pub fn handle_settings(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::BgmUp => {
                if self.config.bgm_volume <= 90 {
                    self.config.bgm_volume += 10;
                    self.apply_bgm_volume();
                    self.status_message = Some(format!("BGM音量: {}%", self.config.bgm_volume));
                }
            }
            SettingsAction::BgmDown => {
                if self.config.bgm_volume >= 10 {
                    self.config.bgm_volume -= 10;
                    self.apply_bgm_volume();
                    self.status_message = Some(format!("BGM音量: {}%", self.config.bgm_volume));
                }
            }
            SettingsAction::VoiceUp => {
                if self.config.voice_volume <= 90 {
                    self.config.voice_volume += 10;
                    audio::set_voice_volume(self.config.voice_volume);
                    self.status_message = Some(format!("语音音量: {}%", self.config.voice_volume));
                }
            }
            SettingsAction::VoiceDown => {
                if self.config.voice_volume >= 10 {
                    self.config.voice_volume -= 10;
                    audio::set_voice_volume(self.config.voice_volume);
                    self.status_message = Some(format!("语音音量: {}%", self.config.voice_volume));
                }
            }
            SettingsAction::AutoPlayToggle => {
                self.config.auto_play = !self.config.auto_play;
                if self.config.auto_play {
                    self.auto_play_timer = Some(Instant::now());
                    self.status_message = Some("自动播放开启".to_string());
                } else {
                    self.auto_play_timer = None;
                    self.status_message = Some("自动播放关闭".to_string());
                }
            }
            SettingsAction::AutoPlaySpeedUp => {
                let new_speed = (self.config.auto_play_speed + 0.5).min(5.0);
                self.config.auto_play_speed = new_speed;
                self.status_message = Some(format!("自动播放速度: {:.1}秒", new_speed));
            }
            SettingsAction::AutoPlaySpeedDown => {
                let new_speed = (self.config.auto_play_speed - 0.5).max(0.5);
                self.config.auto_play_speed = new_speed;
                self.status_message = Some(format!("自动播放速度: {:.1}秒", new_speed));
            }
            SettingsAction::TextAnimationToggle => {
                self.config.text_animation = !self.config.text_animation;
                if !self.config.text_animation && self.display_text != self.target_text {
                    self.display_text = self.target_text.clone();
                }
                self.status_message = Some(
                    if self.config.text_animation {
                        "文字动画开启"
                    } else {
                        "文字动画关闭"
                    }
                    .to_string(),
                );
            }
            SettingsAction::TextSpeedUp => {
                if self.config.text_speed <= 90 {
                    self.config.text_speed += 10;
                    self.status_message = Some(format!("文字速度: {}ms", self.config.text_speed));
                }
            }
            SettingsAction::TextSpeedDown => {
                if self.config.text_speed >= 20 {
                    self.config.text_speed -= 10;
                    self.status_message = Some(format!("文字速度: {}ms", self.config.text_speed));
                }
            }
            SettingsAction::BgColorNext => {
                let colors = vec![
                    "default".to_string(),
                    "#2A2A3E".to_string(),
                    "#1E1E2E".to_string(),
                    "#222436".to_string(),
                    "#2C2C3C".to_string(),
                    "#3A2C3C".to_string(),
                ];
                let current = colors
                    .iter()
                    .position(|c| c == &self.config.background_color)
                    .unwrap_or(0);
                let next = (current + 1) % colors.len();
                self.config.background_color = colors[next].clone();
                let color_name = match self.config.background_color.as_str() {
                    "default" => "终端默认",
                    "#2A2A3E" => "深灰紫",
                    "#1E1E2E" => "猫鼬暗色",
                    "#222436" => "深藏青",
                    "#2C2C3C" => "暖灰",
                    "#3A2C3C" => "紫罗兰灰",
                    _ => &self.config.background_color,
                };
                self.status_message = Some(format!("背景颜色: {}", color_name));
            }
            SettingsAction::Save => {
                if let Err(e) = self.config.save() {
                    self.status_message = Some(format!("保存配置失败: {}", e));
                } else {
                    self.status_message = Some("配置已保存".to_string());
                }
            }
        }
    }
}