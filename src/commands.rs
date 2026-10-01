use std::fs;
use std::io::{Read, Write, Seek};
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::defaults;
use std::collections::HashMap;

/// XOR 加密密钥（仅防普通解压，非真正安全）
const XOR_KEY: &[u8] = b"ngal_2026_secret_key_please_change";

/// 创建新项目
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

    // 生成启动脚本
    let start_sh = base.join("start.sh");
    if !start_sh.exists() {
        fs::write(&start_sh, defaults::DEFAULT_START_SH)?;

        // 赋予可执行权限（仅 Unix）
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

/// 显示项目状态
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

    // 读取项目名
    let game_content = fs::read_to_string(&game_json)?;
    let game_config: crate::parser::GameConfig = serde_json::from_str(&game_content)?;
    let project_name = game_config.title.clone();

    // 读取版本号
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

    // 统计剧情字数
    let mut total_words = 0usize;
    if dialog_dir.exists() {
        for entry in fs::read_dir(&dialog_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ng" || e == "txt") {
                if let Ok(content) = fs::read_to_string(&path) {
                    total_words += content.chars().filter(|c| !c.is_whitespace()).count();
                }
            }
        }
    }

    // 缺失资源检测
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
                    // 去除注释
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
    println!("剧情: {:>4} 个   {} 字", story_count, total_words);

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

/// 打包项目为加密的 .ngal 文件
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

    // 1. 打包到内存 zip
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, &assets, "assets", options)?;
    let cursor = zip.finish()?;
    let zip_data = cursor.into_inner();

    // 2. ChaCha20Poly1305 加密
    let encrypted = crate::crypto::encrypt(&zip_data)?;

    // 3. 写入（版本 3 = 内存运行模式）
    let mut file = fs::File::create(&output_path)?;
    file.write_all(b"NGAL")?;
    file.write_all(&3u32.to_le_bytes())?;
    file.write_all(&encrypted)?;

    let size = fs::metadata(&output_path)?.len();
    println!("✅ 打包完成: {}", output_path.display());
    println!("   大小: {:.2} MB", size as f64 / 1024.0 / 1024.0);

    Ok(())
}

/// 清理文件名中的非法字符（/ \ : * ? " < > |）
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

/// 运行打包的 .ngal 文件（自动解密并解压到隐藏目录）
/// 运行打包的 .ngal 文件
/// 资源解压到系统临时目录，save/ 保留在当前工作目录，退出后自动清理
pub fn run_packed(packed_file: &Path, debug: bool) -> Result<()> {
    // 1. 读取并验证
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

    // 2. 解密
    let zip_data = match version {
        3 | 2 => crate::crypto::decrypt(&data)?,
        1 => xor_decrypt(&data),
        _ => {
            eprintln!("❌ 不支持的版本: {}", version);
            return Ok(());
        }
    };

    // 3. 存档目录（当前工作目录下的 save/）
    let original_dir = std::env::current_dir()?;
    let save_dir = original_dir.join("save");
    fs::create_dir_all(&save_dir)?;
    std::env::set_var("NGAL_SAVE_DIR", save_dir.to_string_lossy().to_string());

    // 4. 从 zip 加载所有文件到内存
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_file() {
            let name = entry.name().to_string();
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            files.insert(name, buf);
        }
    }

    // 5. 设置内存资源源
    crate::assets::set_memory(files);

    if debug {
        println!("=====================================");
        println!("运行模式: 内存 + LRU");
        println!("存档目录: {}", save_dir.display());
        println!("=====================================\n");
    }

    // 6. 运行游戏
    let result = crate::runner::run_game();

    // 7. 清理临时缓存
    crate::assets::cleanup_cache();

    // 恢复工作目录
    let _ = std::env::set_current_dir(&original_dir);

    result
}

/// XOR 加密（对称，解密用同一函数）
fn xor_encrypt(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

/// XOR 解密（对称）
fn xor_decrypt(data: &[u8]) -> Vec<u8> {
    xor_encrypt(data)
}

/// 递归添加目录到 zip
fn add_dir_to_zip<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: zip::write::FileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let zip_name = format!("{}/{}", prefix, name);

        if path.is_dir() {
            add_dir_to_zip(zip, &path, &zip_name, options)?;
        } else {
            zip.start_file(zip_name, options)?;
            let mut f = fs::File::open(&path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        }
    }
    Ok(())
}

/// 统计目录下文件数量和总大小（不递归）
fn count_files(dir: &Path) -> Result<(usize, u64)> {
    let mut count = 0;
    let mut size = 0u64;
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                count += 1;
                size += fs::metadata(&path)?.len();
            }
        }
    }
    Ok((count, size))
}

/// 提取纯文件名（去掉路径）
fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// 执行在线更新脚本
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