// src/edit/stats.rs
use std::fs;
use std::path::Path;

use super::state::StatInfo;
use super::Editor;

pub fn compute_stats(editor: &Editor) -> StatInfo {
    let mut info = StatInfo::default();
    let base = editor.game_root();

    // 项目名
    let game_json = base.join("assets/game.json");
    if game_json.exists() {
        if let Ok(content) = fs::read_to_string(&game_json) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
                    info.project_name = t.to_string();
                }
            }
        }
    }

    // 版本号
    let config_json = base.join("assets/config.json");
    if config_json.exists() {
        if let Ok(content) = fs::read_to_string(&config_json) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                    info.version = ver.to_string();
                }
            }
        }
    }

    let portraits_dir = base.join("assets/portraits");
    let music_dir = base.join("assets/music");
    let voices_dir = base.join("assets/voices");
    let dialog_dir = base.join("assets/dialog");

    let (img_count, img_size) = count_files(&portraits_dir);
    info.img_count = img_count;
    info.img_size = img_size;

    let (music_count, music_size) = count_files(&music_dir);
    info.music_count = music_count;
    info.music_size = music_size;

    let (voice_count, voice_size) = count_files(&voices_dir);
    info.voice_count = voice_count;
    info.voice_size = voice_size;

    // 剧情文件统计
    if dialog_dir.exists() {
        if let Ok(entries) = fs::read_dir(&dialog_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext == "ng" || ext == "txt" {
                            info.story_count += 1;
                            if let Ok(content) = fs::read_to_string(&path) {
                                info.story_size += content.len() as u64;
                                info.total_words +=
                                    content.chars().filter(|c| !c.is_whitespace()).count();
                            }
                        }
                    }
                }
            }
        }
    }

    info.total_size = info.img_size + info.music_size + info.voice_size + info.story_size;

    // 缺失资源检测
    if dialog_dir.exists() {
        if let Ok(entries) = fs::read_dir(&dialog_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    if ext == "ng" || ext == "txt" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            for line in content.lines() {
                                let line = line.split('#').next().unwrap_or("").trim();
                                if line.is_empty() {
                                    continue;
                                }
                                if line.starts_with("img:") || line.starts_with("bg:") {
                                    let rest = if line.starts_with("img:") {
                                        &line[4..]
                                    } else {
                                        &line[3..]
                                    };
                                    let parts: Vec<&str> = rest.split(':').collect();
                                    let filename = parts[0].trim();
                                    if !filename.is_empty() {
                                        if !portraits_dir.join(filename).exists() {
                                            let name = basename(filename);
                                            if !info.missing_images.contains(&name) {
                                                info.missing_images.push(name);
                                            }
                                        }
                                    }
                                } else if line.starts_with("music:") {
                                    let filename = line[6..].trim();
                                    if !filename.is_empty()
                                        && !music_dir.join(filename).exists()
                                    {
                                        let name = basename(filename);
                                        if !info.missing_music.contains(&name) {
                                            info.missing_music.push(name);
                                        }
                                    }
                                } else if line.contains(':') {
                                    let parts: Vec<&str> = line.splitn(3, ':').collect();
                                    if parts.len() == 3 {
                                        let voice_file = parts[2].trim();
                                        if !voice_file.is_empty()
                                            && (voice_file.ends_with(".mp3")
                                                || voice_file.ends_with(".wav")
                                                || voice_file.ends_with(".ogg"))
                                            && !voices_dir.join(voice_file).exists()
                                        {
                                            let name = basename(voice_file);
                                            if !info.missing_voices.contains(&name) {
                                                info.missing_voices.push(name);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

fn count_files(dir: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut size = 0u64;
    if dir.exists() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    count += 1;
                    if let Ok(m) = fs::metadata(&path) {
                        size += m.len();
                    }
                }
            }
        }
    }
    (count, size)
}

fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}