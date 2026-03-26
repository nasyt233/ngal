use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

pub const DEFAULT_CONFIG: &str = r#"{
  "bgm_volume": 50,
  "voice_volume": 80,
  "auto_play": false,
  "auto_play_speed": 2.0,
  "version": "0.5.0"
}"#;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bgm_volume: u8,
    pub voice_volume: u8,
    pub auto_play: bool,
    pub auto_play_speed: f64,
    pub version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bgm_volume: 70,
            voice_volume: 80,
            auto_play: false,
            auto_play_speed: 2.0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = Path::new("assets/config.json");
        if !path.exists() {
            fs::write(path, DEFAULT_CONFIG)?;
            Ok(Config::default())
        } else {
            let content = fs::read_to_string(path)?;
            match serde_json::from_str::<Config>(&content) {
                Ok(config) => Ok(config),
                Err(_) => {
                    // 合并旧配置，补全缺失字段
                    let mut config = Config::default();
                    if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(bgm) = existing.get("bgm_volume").and_then(|v| v.as_u64()) {
                            config.bgm_volume = bgm as u8;
                        }
                        if let Some(voice) = existing.get("voice_volume").and_then(|v| v.as_u64()) {
                            config.voice_volume = voice as u8;
                        }
                        if let Some(ver) = existing.get("version").and_then(|v| v.as_str()) {
                            config.version = ver.to_string();
                        }
                    }
                    let _ = fs::write(path, serde_json::to_string_pretty(&config)?);
                    Ok(config)
                }
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Path::new("assets/config.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }
}