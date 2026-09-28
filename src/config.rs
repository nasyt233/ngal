// src/config.rs
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

pub const DEFAULT_CONFIG: &str = r##"{
  "bgm_volume": 70,
  "voice_volume": 80,
  "auto_play": false,
  "auto_play_speed": 3.0,
  "text_animation": true,
  "text_speed": 50,
  "background_color": "#2A2A3E",
  "version": "1.0.3"
}"##;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bgm_volume: u8,
    pub voice_volume: u8,
    pub auto_play: bool,
    pub auto_play_speed: f64,
    pub text_animation: bool,
    pub text_speed: u64,
    pub background_color: String,
    pub version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bgm_volume: 70,
            voice_volume: 80,
            auto_play: false,
            auto_play_speed: 3.0,
            text_animation: true,
            text_speed: 50,
            background_color: "#2A2A3E".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Config {
    /// 配置文件路径：
    /// - 文件系统模式 → assets/config.json
    /// - 打包模式     → save/config.json
    fn config_path() -> std::path::PathBuf {
        if crate::assets::is_memory() {
            std::path::PathBuf::from("save/config.json")
        } else {
            std::path::PathBuf::from("assets/config.json")
        }
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&path, DEFAULT_CONFIG)?;
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)?;
        match serde_json::from_str::<Config>(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                // 兼容旧版本字段：逐项从 JSON 中取，缺的用默认值
                let mut config = Config::default();
                if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(bgm) = existing.get("bgm_volume").and_then(|v| v.as_u64()) {
                        config.bgm_volume = bgm as u8;
                    }
                    if let Some(voice) = existing.get("voice_volume").and_then(|v| v.as_u64()) {
                        config.voice_volume = voice as u8;
                    }
                    if let Some(ap) = existing.get("auto_play").and_then(|v| v.as_bool()) {
                        config.auto_play = ap;
                    }
                    if let Some(sp) = existing.get("auto_play_speed").and_then(|v| v.as_f64()) {
                        config.auto_play_speed = sp;
                    }
                    if let Some(ta) = existing.get("text_animation").and_then(|v| v.as_bool()) {
                        config.text_animation = ta;
                    }
                    if let Some(ts) = existing.get("text_speed").and_then(|v| v.as_u64()) {
                        config.text_speed = ts;
                    }
                    if let Some(bg) = existing.get("background_color").and_then(|v| v.as_str()) {
                        config.background_color = bg.to_string();
                    }
                    if let Some(ver) = existing.get("version").and_then(|v| v.as_str()) {
                        config.version = ver.to_string();
                    }
                }
                let _ = fs::write(&path, serde_json::to_string_pretty(&config)?);
                Ok(config)
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }
}