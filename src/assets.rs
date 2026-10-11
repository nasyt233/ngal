// src/assets.rs
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

pub struct MemoryAssets {
    pub files: HashMap<String, Vec<u8>>,
}

pub enum ResourceSource {
    FileSystem,
    Memory(Arc<MemoryAssets>),
}

static SOURCE: OnceLock<RwLock<Option<ResourceSource>>> = OnceLock::new();

fn source_lock() -> &'static RwLock<Option<ResourceSource>> {
    SOURCE.get_or_init(|| RwLock::new(None))
}

pub fn set_filesystem() {
    *source_lock().write().unwrap() = Some(ResourceSource::FileSystem);
}

pub fn set_memory(files: HashMap<String, Vec<u8>>) {
    *source_lock().write().unwrap() =
        Some(ResourceSource::Memory(Arc::new(MemoryAssets { files })));
}

pub fn is_memory() -> bool {
    matches!(
        source_lock().read().unwrap().as_ref(),
        Some(ResourceSource::Memory(_))
    )
}

pub fn read_text(path: &str) -> Option<String> {
    let guard = source_lock().read().unwrap();
    match guard.as_ref()? {
        ResourceSource::FileSystem => fs::read_to_string(path).ok(),
        ResourceSource::Memory(m) => m
            .files
            .get(path)
            .and_then(|v| String::from_utf8(v.clone()).ok()),
    }
}

pub fn read_bytes(path: &str) -> Option<Vec<u8>> {
    let guard = source_lock().read().unwrap();
    match guard.as_ref()? {
        ResourceSource::FileSystem => fs::read(path).ok(),
        ResourceSource::Memory(m) => m.files.get(path).cloned(),
    }
}

pub fn exists(path: &str) -> bool {
    let guard = source_lock().read().unwrap();
    match guard.as_ref() {
        Some(ResourceSource::FileSystem) => Path::new(path).exists(),
        Some(ResourceSource::Memory(m)) => m.files.contains_key(path),
        None => Path::new(path).exists(),
    }
}

/// 递归列出目录下所有文件（相对路径，含子目录）
/// 例如 assets/portraits 下有 `我/hi.png` 和 `bg.png`，返回 ["bg.png", "我/hi.png"]
pub fn list_dir_recursive(dir: &str) -> Vec<String> {
    let mut result = Vec::new();
    let prefix = format!("{}/", dir.trim_end_matches('/'));

    {
        let guard = source_lock().read().unwrap();
        match guard.as_ref() {
            Some(ResourceSource::FileSystem) => {
                let base = Path::new(dir);
                if base.is_dir() {
                    walk_fs(base, base, &mut result);
                }
            }
            Some(ResourceSource::Memory(m)) => {
                for key in m.files.keys() {
                    if let Some(rest) = key.strip_prefix(&prefix) {
                        if !rest.is_empty() {
                            result.push(rest.to_string());
                        }
                    }
                }
            }
            None => {}
        }
    }

    result.sort();
    result
}

/// 非递归版本（兼容旧调用）
pub fn list_dir(dir: &str) -> Vec<String> {
    let mut result = Vec::new();
    let prefix = format!("{}/", dir.trim_end_matches('/'));

    {
        let guard = source_lock().read().unwrap();
        match guard.as_ref() {
            Some(ResourceSource::FileSystem) => {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                result.push(name.to_string());
                            }
                        }
                    }
                }
            }
            Some(ResourceSource::Memory(m)) => {
                for key in m.files.keys() {
                    if let Some(rest) = key.strip_prefix(&prefix) {
                        if !rest.is_empty() && !rest.contains('/') {
                            result.push(rest.to_string());
                        }
                    }
                }
            }
            None => {}
        }
    }

    result.sort();
    result
}

fn walk_fs(base: &Path, current: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(current) {
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
            walk_fs(base, &path, out);
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// 将内存中的资源落盘为临时文件（保持子目录结构）
pub fn materialize(path: &str) -> Option<PathBuf> {
    let guard = source_lock().read().unwrap();
    match guard.as_ref()? {
        ResourceSource::FileSystem => {
            let p = Path::new(path);
            if p.exists() {
                Some(p.to_path_buf())
            } else {
                None
            }
        }
        ResourceSource::Memory(m) => {
            let data = m.files.get(path)?;
            let temp_root = std::env::temp_dir().join("ngal_cache");
            let _ = fs::create_dir_all(&temp_root);

            // 保留子目录结构，避免同名冲突
            let sub_path = path.replace('/', "_");
            let temp_path = temp_root.join(&sub_path);

            if !temp_path.exists() {
                let _ = fs::write(&temp_path, data);
            }
            Some(temp_path)
        }
    }
}

pub fn cleanup_cache() {
    let should_clean = {
        let guard = source_lock().read().unwrap();
        matches!(guard.as_ref(), Some(ResourceSource::Memory(_)))
    };

    if should_clean {
        let temp_root = std::env::temp_dir().join("ngal_cache");
        let _ = fs::remove_dir_all(&temp_root);
    }
}