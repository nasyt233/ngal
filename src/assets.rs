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

/// 用 RwLock 包裹，允许运行期间重新设置资源源
static SOURCE: OnceLock<RwLock<Option<ResourceSource>>> = OnceLock::new();

fn source_lock() -> &'static RwLock<Option<ResourceSource>> {
    SOURCE.get_or_init(|| RwLock::new(None))
}

/// 使用文件系统作为资源源（源码模式）
pub fn set_filesystem() {
    *source_lock().write().unwrap() = Some(ResourceSource::FileSystem);
}

/// 使用内存作为资源源（打包模式）
pub fn set_memory(files: HashMap<String, Vec<u8>>) {
    *source_lock().write().unwrap() =
        Some(ResourceSource::Memory(Arc::new(MemoryAssets { files })));
}

/// 当前是否使用内存资源源（打包模式）
pub fn is_memory() -> bool {
    matches!(
        source_lock().read().unwrap().as_ref(),
        Some(ResourceSource::Memory(_))
    )
}

/// 读取文本文件
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

/// 读取二进制文件
pub fn read_bytes(path: &str) -> Option<Vec<u8>> {
    let guard = source_lock().read().unwrap();
    match guard.as_ref()? {
        ResourceSource::FileSystem => fs::read(path).ok(),
        ResourceSource::Memory(m) => m.files.get(path).cloned(),
    }
}

/// 检查文件是否存在
pub fn exists(path: &str) -> bool {
    let guard = source_lock().read().unwrap();
    match guard.as_ref() {
        Some(ResourceSource::FileSystem) => Path::new(path).exists(),
        Some(ResourceSource::Memory(m)) => m.files.contains_key(path),
        None => Path::new(path).exists(),
    }
}

/// 列出目录下的直接文件名（不含路径，不含子目录）
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

/// 将内存中的资源落盘为临时文件（用于需要文件路径的场景，如 mpv 播放、图片解码）
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

            // 保持子目录结构，避免重名（用下划线替换斜杠）
            let sub_path = path.replace('/', "_");
            let temp_path = temp_root.join(&sub_path);

            if !temp_path.exists() {
                let _ = fs::write(&temp_path, data);
            }
            Some(temp_path)
        }
    }
}

/// 清理临时缓存（退出时调用）
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