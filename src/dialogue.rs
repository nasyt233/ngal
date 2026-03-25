use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueLine {
    pub speaker: Option<String>,
    pub text: Option<String>,
    pub voice: Option<String>,
    pub music: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionData {
    pub text: String,
    pub next_scene: String,
}

#[derive(Debug, Clone)]
pub struct SceneData {
    pub dialogue: Vec<DialogueLine>,
    pub options: Vec<OptionData>,
}

impl<'de> Deserialize<'de> for SceneData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawSceneData {
            dialogue: Vec<serde_json::Value>,
            options: Vec<OptionData>,
        }
        let raw = RawSceneData::deserialize(deserializer)?;
        let mut dialogue = Vec::new();
        for val in raw.dialogue {
            if val.is_string() {
                let s = val.as_str().unwrap();
                // 尝试解析新格式：speaker:text:voice 或 speaker:text 或 纯文本
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 3 {
                    // 格式: speaker:text:voice
                    let speaker = parts[0].trim();
                    let text = parts[1].trim();
                    let voice = parts[2].trim();
                    dialogue.push(DialogueLine {
                        speaker: Some(speaker.to_string()),
                        text: Some(text.to_string()),
                        voice: Some(voice.to_string()),
                        music: None,
                    });
                } else if parts.len() == 2 {
                    // 格式: speaker:text
                    let speaker = parts[0].trim();
                    let text = parts[1].trim();
                    dialogue.push(DialogueLine {
                        speaker: Some(speaker.to_string()),
                        text: Some(text.to_string()),
                        voice: None,
                        music: None,
                    });
                } else if parts.len() == 1 {
                    // 旁白，无冒号
                    dialogue.push(DialogueLine {
                        speaker: None,
                        text: Some(s.trim().to_string()),
                        voice: None,
                        music: None,
                    });
                } else {
                    // 多于三个冒号，按第一个冒号分割（忽略多余的）
                    if let Some(idx) = s.find(':') {
                        let speaker = s[..idx].trim().to_string();
                        let rest = s[idx+1..].trim();
                        if let Some(idx2) = rest.find(':') {
                            let text = rest[..idx2].trim().to_string();
                            let voice = rest[idx2+1..].trim().to_string();
                            dialogue.push(DialogueLine {
                                speaker: Some(speaker),
                                text: Some(text),
                                voice: Some(voice),
                                music: None,
                            });
                        } else {
                            dialogue.push(DialogueLine {
                                speaker: Some(speaker),
                                text: Some(rest.to_string()),
                                voice: None,
                                music: None,
                            });
                        }
                    } else {
                        dialogue.push(DialogueLine {
                            speaker: None,
                            text: Some(s.trim().to_string()),
                            voice: None,
                            music: None,
                        });
                    }
                }
            } else {
                // 对象格式（音乐指令或带语音的对象）
                let line: DialogueLine = serde_json::from_value(val)
                    .map_err(serde::de::Error::custom)?;
                dialogue.push(line);
            }
        }
        Ok(SceneData {
            dialogue,
            options: raw.options,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct DialogueDB {
    pub title: String,
    pub footer: String,
    pub scenes: HashMap<String, SceneData>,
    pub initial_scene: String,
}