use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::{Child, Command};

/// 播放音频文件（使用 mpv）
/// background: true 循环播放，false 单次播放
pub fn play_audio(path: &Path, background: bool, volume: u8) -> Result<Child> {
    if !path.exists() {
        return Err(anyhow!("音频文件不存在: {}", path.display()));
    }
    let volume_str = format!("{}", volume);
    let mut cmd = Command::new("mpv");
    cmd.arg("--no-video")
        .arg("--really-quiet")
        .arg("--vo=null")
        .arg("--no-window-dragging")
        .arg("--no-input-default-bindings")
        .arg("--no-input-cursor")
        .arg(format!("--volume={}", volume_str))
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if background {
        cmd.arg("--loop=inf");
    }
    Ok(cmd.spawn()?)
}