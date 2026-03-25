use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::Result;
use crate::defaults;

#[derive(Debug, Clone)]
pub enum DialogueCommand {
    Text { speaker: Option<String>, text: String, voice: Option<String> },
    Image { filename: Option<String> },
    Music { filename: String },
    MusicStop,  // 新增：停止音乐
    Choose { options: Vec<(String, String)> },
    Load { target: String },
    End,
}
pub struct SceneData {
    pub commands: Vec<DialogueCommand>,
}

/// 解析剧情文件，返回场景名到命令列表的映射
pub fn parse_dialogue_file(content: &str) -> Result<HashMap<String, SceneData>> {
    let mut scenes = HashMap::new();
    let mut current_scene = String::new();
    let mut current_commands = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            // 新场景
            if !current_scene.is_empty() {
                scenes.insert(current_scene, SceneData { commands: current_commands });
                current_commands = Vec::new();
            }
            current_scene = line[1..line.len()-1].to_string();
        } else if line.starts_with("img:") {
            let filename = line[4..].trim();
            let cmd = if filename.is_empty() {
                DialogueCommand::Image { filename: None }
            } else {
                DialogueCommand::Image { filename: Some(filename.to_string()) }
            };
            current_commands.push(cmd);
        } else if line.starts_with("music:") {
            let filename = line[6..].trim();
            if filename.is_empty() {
                current_commands.push(DialogueCommand::MusicStop);
            } else {
                current_commands.push(DialogueCommand::Music { filename: filename.to_string() });
            }
        } else if line.starts_with("choose:") {
            let rest = &line[7..];
            let mut options = Vec::new();
            for part in rest.split('|') {
                let parts: Vec<&str> = part.splitn(2, ':').collect();
                if parts.len() == 2 {
                    options.push((parts[0].to_string(), parts[1].to_string()));
                }
            }
            current_commands.push(DialogueCommand::Choose { options });
        } else if line.starts_with("load:") {
            let target = line[5..].trim().to_string();
            current_commands.push(DialogueCommand::Load { target });
        } else if line == "end" {
            current_commands.push(DialogueCommand::End);
        } else {
            // 普通对话
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            let (speaker, text, voice) = match parts.len() {
                1 => (None, parts[0].to_string(), None),
                2 => (Some(parts[0].to_string()), parts[1].to_string(), None),
                3 => (Some(parts[0].to_string()), parts[1].to_string(), Some(parts[2].to_string())),
                _ => continue,
            };
            let text = text.replace("\\n", "\n");
            current_commands.push(DialogueCommand::Text { speaker, text, voice });
        }
    }
    if !current_scene.is_empty() {
        scenes.insert(current_scene, SceneData { commands: current_commands });
    }
    Ok(scenes)
}

/// 加载游戏配置
#[derive(serde::Deserialize)]
pub struct GameConfig {
    pub title: String,
    pub footer: String,
    pub index: String,
}

/// 加载游戏配置，如果文件不存在则使用默认配置
pub fn load_game_config() -> Result<GameConfig> {
    let path = Path::new("assets/game.json");
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        // 确保 assets 目录存在
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, defaults::DEFAULT_GAME_CONFIG)?;
        defaults::DEFAULT_GAME_CONFIG.to_string()
    };
    Ok(serde_json::from_str(&content)?)
}

/// 加载剧情文件，如果文件不存在则使用默认剧情
pub fn load_dialogue() -> Result<String> {
    let path = Path::new("assets/dialog/dialogue.txt");
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        // 确保 assets/dialog 目录存在
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, defaults::DEFAULT_DIALOGUE)?;
        defaults::DEFAULT_DIALOGUE.to_string()
    };
    Ok(content)
}