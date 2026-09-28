// src/audio.rs
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};

static BGM_PROC: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static VOICE_PROC: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static BGM_VOLUME: Mutex<u8> = Mutex::new(70);
static VOICE_VOLUME: Mutex<u8> = Mutex::new(80);

/// 初始化音频系统
pub fn init() -> Result<()> {
    BGM_PROC
        .set(Mutex::new(None))
        .map_err(|_| anyhow!("BGM 已初始化"))?;
    VOICE_PROC
        .set(Mutex::new(None))
        .map_err(|_| anyhow!("语音已初始化"))?;
    Ok(())
}

/// 查找 mpv 可执行文件
fn find_mpv() -> Option<PathBuf> {
    // 1. 程序同目录下的 mpv（绿色版部署）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let local = if cfg!(windows) {
                dir.join("mpv.exe")
            } else {
                dir.join("mpv")
            };
            if local.exists() {
                return Some(local);
            }
        }
    }
    // 2. 系统 PATH
    let name = if cfg!(windows) { "mpv.exe" } else { "mpv" };
    if Command::new(name).arg("--version").output().is_ok() {
        return Some(PathBuf::from(name));
    }
    None
}

fn spawn_mpv(path: &str, background: bool, volume: u8) -> Result<Child> {
    let mpv = find_mpv().ok_or_else(|| {
        anyhow!(
            "找不到 mpv 播放器，请安装：\n\
             Termux:  pkg install mpv\n\
             Linux:   apt install mpv  /  dnf install mpv  /  pacman -S mpv\n\
             macOS:   brew install mpv\n\
             Windows: 下载 mpv.exe 放到 PATH 或程序同目录"
        )
    })?;

    let real_path = crate::assets::materialize(path)
        .ok_or_else(|| anyhow!("音频不存在: {}", path))?;

    let mut cmd = Command::new(mpv);
    cmd.arg("--no-video")
        .arg("--really-quiet")
        .arg("--vo=null")
        .arg("--no-window-dragging")
        .arg("--no-input-default-bindings")
        .arg("--no-input-cursor")
        .arg(format!("--volume={}", volume))
        .arg(&real_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Windows 隐藏控制台窗口
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if background {
        cmd.arg("--loop=inf");
    }

    Ok(cmd.spawn()?)
}

fn kill_child(slot: &OnceLock<Mutex<Option<Child>>>) {
    if let Some(m) = slot.get() {
        if let Some(mut child) = m.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// 播放背景音乐（循环）
pub fn play_bgm(path: &str, volume: u8) -> Result<()> {
    *BGM_VOLUME.lock().unwrap() = volume;
    kill_child(&BGM_PROC);
    let child = spawn_mpv(path, true, volume)?;
    if let Some(m) = BGM_PROC.get() {
        *m.lock().unwrap() = Some(child);
    }
    Ok(())
}

/// 播放语音（单次）
pub fn play_voice(path: &str, volume: u8) -> Result<()> {
    *VOICE_VOLUME.lock().unwrap() = volume;
    kill_child(&VOICE_PROC);
    let child = spawn_mpv(path, false, volume)?;
    if let Some(m) = VOICE_PROC.get() {
        *m.lock().unwrap() = Some(child);
    }
    Ok(())
}

/// 停止背景音乐
pub fn stop_bgm() {
    kill_child(&BGM_PROC);
}

/// 停止语音
pub fn stop_voice() {
    kill_child(&VOICE_PROC);
}

/// 设置 BGM 音量（下次播放生效）
///
/// mpv 的 `--volume` 是启动参数，运行中无法修改。
/// 这里记录用户设置，下次调用 `play_bgm` 时使用新值。
pub fn set_bgm_volume(volume: u8) {
    *BGM_VOLUME.lock().unwrap() = volume;
}

/// 设置语音音量（下次播放生效）
pub fn set_voice_volume(volume: u8) {
    *VOICE_VOLUME.lock().unwrap() = volume;
}