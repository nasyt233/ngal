use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::app::AppState;
use crate::variables::Variables;
use crate::parser::ImageParams;

pub const MAX_SLOTS: usize = 10;

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub state: AppState,
    pub menu_selected: usize,
    pub variables: std::collections::HashMap<String, String>,
    pub current_image: Option<String>,          // 保留兼容
    pub timestamp: String,
    pub current_file: Option<String>,           // 当前场景所在文件名
    pub background: Option<String>,             // 背景图片
    pub bgm: Option<String>,                    // 背景音乐
    pub image_params: Option<ImageParams>,      // 立绘参数
}

impl SaveData {
    pub fn new(
        state: &AppState,
        menu_selected: usize,
        variables: &Variables,
        current_file: Option<String>,
        background: Option<String>,
        bgm: Option<String>,
        image_params: Option<ImageParams>,
    ) -> Self {
        use chrono::Local;
        Self {
            state: state.clone(),
            menu_selected,
            variables: variables.serialize(),
            current_image: image_params.as_ref().and_then(|p| p.filename.clone()),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            current_file,
            background,
            bgm,
            image_params,
        }
    }

    pub fn save(
        slot: usize,
        state: &AppState,
        menu_selected: usize,
        variables: &Variables,
        current_file: Option<String>,
        background: Option<String>,
        bgm: Option<String>,
        image_params: Option<ImageParams>,
    ) -> Result<()> {
        if slot == 0 || slot > MAX_SLOTS {
            anyhow::bail!("存档槽位无效: {}", slot);
        }
        let data = SaveData::new(
            state,
            menu_selected,
            variables,
            current_file,
            background,
            bgm,
            image_params,
        );
        let json = serde_json::to_string_pretty(&data)?;
        let path = Self::slot_path(slot);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(slot: usize) -> Result<Self> {
        if slot == 0 || slot > MAX_SLOTS {
            anyhow::bail!("存档槽位无效: {}", slot);
        }
        let path = Self::slot_path(slot);
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn exists(slot: usize) -> bool {
        if slot == 0 || slot > MAX_SLOTS {
            return false;
        }
        Self::slot_path(slot).exists()
    }

    fn slot_path(slot: usize) -> PathBuf {
        PathBuf::from(format!("save/slot{}.json", slot))
    }
}