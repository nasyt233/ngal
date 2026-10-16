use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::defaults;
use std::collections::HashMap;
use zip::write::SimpleFileOptions;

const XOR_KEY: &[u8] = b"ngal_2026_secret_key_please_change";

// ============================================================
// 创建新项目
// ============================================================
pub fn new_project(dir: Option<PathBuf>) -> Result<()> {
    let base = dir.unwrap_or_else(|| PathBuf::from("."));
    let base = if base.exists() {
        base.canonicalize().unwrap_or(base)
    } else {
        base
    };

    let game_json = base.join("assets/game.json");
    if game_json.exists() {
        eprintln!("❌ 项目已存在，不覆盖: {}", base.display());
        return Ok(());
    }

    fs::create_dir_all(base.join("assets/dialog"))?;
    fs::create_dir_all(base.join("assets/portraits"))?;
    fs::create_dir_all(base.join("assets/music"))?;
    fs::create_dir_all(base.join("assets/voices"))?;
    fs::create_dir_all(base.join("save"))?;

    fs::write(base.join("assets/game.json"), defaults::DEFAULT_GAME_CONFIG)?;
    fs::write(
        base.join("assets/dialog/dialogue.ng"),
        defaults::DEFAULT_DIALOGUE,
    )?;

    let start_sh = base.join("start.sh");
    if !start_sh.exists() {
        fs::write(&start_sh, defaults::DEFAULT_START_SH)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&start_sh) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&start_sh, perms);
            }
        }
    }

    println!("✅ 项目创建成功: {}", base.display());
    println!("   运行方式:");
    println!("     1) ngal .                （直接运行）");
    println!("     2) ./start.sh            （带依赖检查的启动脚本）");
    Ok(())
}

// ============================================================
// 显示项目状态
// ============================================================
pub fn show_status(dir: Option<PathBuf>) -> Result<()> {
    let base = dir.unwrap_or_else(|| PathBuf::from("."));
    let base = if base.exists() {
        base.canonicalize().unwrap_or(base)
    } else {
        base
    };

    let game_json = base.join("assets/game.json");
    if !game_json.exists() {
        eprintln!("目录中没有游戏文件: {}", base.display());
        eprintln!("输入 ngal help 查看帮助");
        return Ok(());
    }

    let game_content = fs::read_to_string(&game_json)?;
    let game_config: crate::parser::GameConfig = serde_json::from_str(&game_content)?;
    let project_name = game_config.title.clone();

    let config_path = base.join("assets/config.json");
    let version = if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)?;
        if let Ok(config) = serde_json::from_str::<crate::config::Config>(&config_content) {
            config.version
        } else {
            "unknown".to_string()
        }
    } else {
        "未初始化".to_string()
    };

    let portraits_dir = base.join("assets/portraits");
    let music_dir = base.join("assets/music");
    let voices_dir = base.join("assets/voices");
    let dialog_dir = base.join("assets/dialog");

    let (img_count, img_size) = count_files(&portraits_dir)?;
    let (music_count, music_size) = count_files(&music_dir)?;
    let (voice_count, voice_size) = count_files(&voices_dir)?;
    let (story_count, story_size) = count_files(&dialog_dir)?;

    let total_size = img_size + music_size + voice_size + story_size;

    let mut total_words = 0usize;
    let mut total_lines = 0usize;
    if dialog_dir.exists() {
        for entry in fs::read_dir(&dialog_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ng" || e == "txt") {
                if let Ok(content) = fs::read_to_string(&path) {
                    total_words += content.chars().filter(|c| !c.is_whitespace()).count();
                    total_lines += content.lines().count();
                }
            }
        }
    }

    let mut missing_images = Vec::new();
    let mut missing_music = Vec::new();
    let mut missing_voices = Vec::new();

    if dialog_dir.exists() {
        for entry in fs::read_dir(&dialog_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ng" || e == "txt") {
                let content = fs::read_to_string(&path)?;
                for line in content.lines() {
                    let line = line.split('#').next().unwrap_or("").trim();
                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with("img:") || line.starts_with("bg:") {
                        let rest = if line.starts_with("img:") { &line[4..] } else { &line[3..] };
                        let parts: Vec<&str> = rest.split(':').collect();
                        let filename = parts[0].trim();
                        if !filename.is_empty() {
                            let file_path = portraits_dir.join(filename);
                            if !file_path.exists() {
                                let name = basename(filename);
                                if !missing_images.contains(&name) {
                                    missing_images.push(name);
                                }
                            }
                        }
                    } else if line.starts_with("music:") {
                        let filename = line[6..].trim();
                        if !filename.is_empty() {
                            let file_path = music_dir.join(filename);
                            if !file_path.exists() {
                                let name = basename(filename);
                                if !missing_music.contains(&name) {
                                    missing_music.push(name);
                                }
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
                            {
                                let file_path = voices_dir.join(voice_file);
                                if !file_path.exists() {
                                    let name = basename(voice_file);
                                    if !missing_voices.contains(&name) {
                                        missing_voices.push(name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("项目名: {}", project_name);
    println!("版本号: {}", version);
    println!("当前总大小:  {:.2} MB", total_size as f64 / 1024.0 / 1024.0);
    println!("图片: {:>4} 张   {:.2} MB", img_count, img_size as f64 / 1024.0 / 1024.0);
    println!("音乐: {:>4} 首   {:.2} MB", music_count, music_size as f64 / 1024.0 / 1024.0);
    println!("语音: {:>4} 个   {:.2} MB", voice_count, voice_size as f64 / 1024.0 / 1024.0);
    println!("剧情: {:>4} 个", story_count);
    println!("行数: {:>4} 行", total_lines);
    println!("字数: {:>4} 字", total_words);

    let has_missing =
        !missing_images.is_empty() || !missing_music.is_empty() || !missing_voices.is_empty();

    if has_missing {
        println!("\n缺失内容");

        if !missing_images.is_empty() {
            println!("图片");
            for f in &missing_images {
                println!("{}", f);
            }
        }
        if !missing_music.is_empty() {
            println!("\n音乐");
            for f in &missing_music {
                println!("{}", f);
            }
        }
        if !missing_voices.is_empty() {
            println!("\n语音");
            for f in &missing_voices {
                println!("{}", f);
            }
        }
    }

    Ok(())
}

// ============================================================
// 打包项目（带进度）
// ============================================================
pub fn build_project(dir: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let base = if dir.exists() {
        dir.canonicalize().unwrap_or(dir)
    } else {
        dir
    };

    let assets = base.join("assets");
    if !assets.exists() {
        eprintln!("❌ 找不到 assets 目录: {}", assets.display());
        return Ok(());
    }

    // 读取游戏名
    let game_json = base.join("assets/game.json");
    let default_name = if game_json.exists() {
        if let Ok(content) = fs::read_to_string(&game_json) {
            if let Ok(config) = serde_json::from_str::<crate::parser::GameConfig>(&content) {
                sanitize_filename(&config.title)
            } else {
                "game".to_string()
            }
        } else {
            "game".to_string()
        }
    } else {
        base.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("game")
            .to_string()
    };

    let output_path = match output {
        Some(p) => {
            if p.extension().is_none() {
                p.with_extension("ngal")
            } else {
                p
            }
        }
        None => PathBuf::from(format!("{}.ngal", default_name)),
    };

    // ========== 1. 先收集所有文件 ==========
    println!("📦 正在收集文件...");
    let mut all_files: Vec<(PathBuf, String)> = Vec::new(); // (abs_path, zip_name)
    collect_files_for_zip(&assets, "assets", &mut all_files)?;

    let total = all_files.len();
    if total == 0 {
        eprintln!("❌ assets 目录为空");
        return Ok(());
    }

    let mut total_bytes: u64 = 0;
    for (p, _) in &all_files {
        if let Ok(m) = fs::metadata(p) {
            total_bytes += m.len();
        }
    }

    println!(
        "   共 {} 个文件，{:.2} MB",
        total,
        total_bytes as f64 / 1024.0 / 1024.0
    );

    // ========== 2. 打包并显示进度 ==========
    println!("🗜️  正在压缩...");

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let bar_width = 30;
    let mut last_percent: i32 = -1;

    for (i, (src, zip_name)) in all_files.iter().enumerate() {
        zip.start_file(zip_name.clone(), options)?;
        let mut f = fs::File::open(src)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;

        let percent = (((i + 1) * 100) / total) as i32;
        if percent != last_percent {
            last_percent = percent;
            let filled = (bar_width * (i + 1)) / total;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

            // \r 回到行首覆盖
            print!(
                "\r   [{}] {:>3}%  ({}/{})  {}",
                bar,
                percent,
                i + 1,
                total,
                truncate_path_display(zip_name, 32)
            );
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
        }
    }
    println!(); // 换行

    let cursor = zip.finish()?;
    let zip_data = cursor.into_inner();

    // ========== 3. 加密 ==========
    println!("🔒 正在加密...");
    let encrypted = crate::crypto::encrypt(&zip_data)?;

    // ========== 4. 写入 ==========
    let mut file = fs::File::create(&output_path)?;
    file.write_all(b"NGAL")?;
    file.write_all(&3u32.to_le_bytes())?;
    file.write_all(&encrypted)?;

    let size = fs::metadata(&output_path)?.len();
    println!("✅ 打包完成: {}", output_path.display());
    println!("   大小: {:.2} MB", size as f64 / 1024.0 / 1024.0);
    println!(
        "   压缩率: {:.1}%",
        if total_bytes > 0 {
            (size as f64 / total_bytes as f64) * 100.0
        } else {
            100.0
        }
    );

    Ok(())
}

/// 递归收集文件（不立即压缩，先收集便于显示进度）
fn collect_files_for_zip(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<(PathBuf, String)>,
) -> Result<()> {
    let entries = fs::read_dir(dir)?;
    let mut items: Vec<_> = entries.flatten().collect();
    items.sort_by_key(|e| e.file_name());

    for entry in items {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let zip_name = format!("{}/{}", prefix, name);
        if path.is_dir() {
            collect_files_for_zip(&path, &zip_name, out)?;
        } else {
            out.push((path, zip_name));
        }
    }
    Ok(())
}

fn truncate_path_display(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let start: String = chars[..max.saturating_sub(2)].iter().collect();
        format!("{}..", start)
    }
}

// ============================================================
// 运行打包的 .ngal
// ============================================================
pub fn run_packed(packed_file: &Path, debug: bool) -> Result<()> {
    let mut file = fs::File::open(packed_file)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != b"NGAL" {
        eprintln!("❌ 不是有效的 ngal 文件");
        return Ok(());
    }
    let mut ver_buf = [0u8; 4];
    file.read_exact(&mut ver_buf)?;
    let version = u32::from_le_bytes(ver_buf);

    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let zip_data = match version {
        3 | 2 => crate::crypto::decrypt(&data)?,
        1 => xor_decrypt(&data),
        _ => {
            eprintln!("❌ 不支持的版本: {}", version);
            return Ok(());
        }
    };

    let original_dir = std::env::current_dir()?;
    let save_dir = original_dir.join("save");
    fs::create_dir_all(&save_dir)?;
    std::env::set_var("NGAL_SAVE_DIR", save_dir.to_string_lossy().to_string());

    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_file() {
            let name = entry.name()?.to_string();
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            files.insert(name, buf);
        }
    }

    crate::assets::set_memory(files);

    if debug {
        println!("=====================================");
        println!("运行模式: 内存 + LRU");
        println!("存档目录: {}", save_dir.display());
        println!("=====================================\n");
    }

    let result = crate::runner::run_game();

    crate::assets::cleanup_cache();
    let _ = std::env::set_current_dir(&original_dir);
    result
}

fn xor_encrypt(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

fn xor_decrypt(data: &[u8]) -> Vec<u8> {
    xor_encrypt(data)
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().to_string();
    if trimmed.is_empty() {
        "game".to_string()
    } else {
        trimmed
    }
}

/// 递归统计（用于 show_status）
fn count_files(dir: &Path) -> Result<(usize, u64)> {
    let mut count = 0;
    let mut size = 0u64;
    if dir.is_dir() {
        walk_count(dir, &mut count, &mut size);
    }
    Ok((count, size))
}

fn walk_count(dir: &Path, count: &mut usize, size: &mut u64) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_count(&path, count, size);
        } else if path.is_file() {
            *count += 1;
            if let Ok(m) = fs::metadata(&path) {
                *size += m.len();
            }
        }
    }
}

fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

// ============================================================
// 在线更新
// ============================================================
pub fn update() -> Result<()> {
    println!("正在检查更新...");

    let installer_url = "https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh";

    #[cfg(unix)]
    {
        let script = format!("curl -L {} | sh", installer_url);

        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("✅ 更新完成，请重新运行 ngal");
                Ok(())
            }
            Ok(s) => {
                eprintln!("❌ 更新脚本退出码: {:?}", s.code());
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ 无法执行更新脚本: {}", e);
                Ok(())
            }
        }
    }

    #[cfg(windows)]
    {
        let _ = installer_url;
        eprintln!("Windows 请手动下载最新版：");
        eprintln!("https://gitcode.com/nasyt/ngal/releases");
        Ok(())
    }
}