use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::app::AppState;
use crate::variables::Variables;

pub const MAX_SLOTS: usize = 10;

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub state: AppState,
    pub menu_selected: usize,
    pub variables: std::collections::HashMap<String, String>,
    pub current_image: Option<String>,
    pub timestamp: String,
}

impl SaveData {
    pub fn new(state: &AppState, menu_selected: usize, variables: &Variables, current_image: Option<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            state: state.clone(),
            menu_selected,
            variables: variables.serialize(),
            current_image,
            timestamp: now.to_string(),
        }
    }

    pub fn save(slot: usize, state: &AppState, menu_selected: usize, variables: &Variables, current_image: Option<String>) -> Result<()> {
        if slot == 0 || slot > MAX_SLOTS {
            anyhow::bail!("存档槽位无效: {}", slot);
        }
        let data = SaveData::new(state, menu_selected, variables, current_image);
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