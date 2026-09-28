// src/app/sounds.rs
use crate::audio;
use super::App;

impl App {
    pub fn play_title_bgm(&mut self) {
        let path = "assets/music/title.mp3";
        if crate::assets::exists(path) {
            audio::stop_bgm();
            if audio::play_bgm(path, self.config.bgm_volume).is_ok() {
                self.current_bgm = Some("title.mp3".to_string());
            }
        }
    }

    pub fn play_bgm(&mut self, filename: &str) {
        audio::stop_bgm();
        let path = format!("assets/music/{}", filename);
        if crate::assets::exists(&path) {
            if audio::play_bgm(&path, self.config.bgm_volume).is_ok() {
                self.current_bgm = Some(filename.to_string());
            }
        }
    }

    pub fn stop_bgm(&mut self) {
        audio::stop_bgm();
        self.current_bgm = None;
    }

    pub fn play_voice_by_file(&mut self, speaker: &str, voice_filename: Option<&str>) {
        audio::stop_voice();
        let filename = if let Some(name) = voice_filename {
            name.to_string()
        } else {
            format!("{}.mp3", speaker)
        };
        let path = format!("assets/voices/{}", filename);
        if crate::assets::exists(&path) {
            let _ = audio::play_voice(&path, self.config.voice_volume);
        }
    }

    pub fn stop_voice(&mut self) {
        audio::stop_voice();
    }

    pub(crate) fn apply_bgm_volume(&mut self) {
        audio::set_bgm_volume(self.config.bgm_volume);
    }
}