# 🎮 ngal - 终端视觉小说引擎

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![Ratatui](https://img.shields.io/badge/ratatui-0.26-blue)](https://github.com/ratatui-org/ratatui)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

**ngal** 是一个用 Rust 编写的终端 Galgame（视觉小说）引擎，让你在命令行中体验分支对话的乐趣。

它拥有精美的双边框 UI、**图片立绘**、**背景音乐**、**角色语音**、**选项分支**、**存档/读档**功能，并会自动创建项目所需的目录和默认剧情文件。

![演示](image/ys.png)  
*（请自行添加实际运行截图）*

---

## ✨ 功能特色

- 🎨 **华丽彩色标题** – 主菜单显示渐变色艺术字，带星形装饰
- 🖼️ **图片立绘** – 自动加载 `assets/portraits/` 目录下的角色图片，在立绘区域居中显示，无图片时回退到 ASCII 占位
- 🎵 **背景音乐** – 支持场景内动态切换 BGM，音量可独立调节
- 🎤 **角色语音** – 每句台词可指定独立语音文件，支持自定义音量
- 📜 **简化分支对话** – 每条对话独立指定说话人，支持任意角色切换，无需为换人创建新场景
- 💾 **存档/读档** – 随时按 `S` 保存，按 `L` 加载（存档保存在 `save/save.json`）
- ⚙️ **设置菜单** – 独立调节 BGM 和语音音量，配置保存至 `assets/config.json`
- 📝 **自定义底部提示** – 主菜单底部可显示自定义文本（如操作说明）
- 🛠️ **自动初始化** – 首次运行自动创建 `assets/`、`assets/portraits/`、`assets/music/`、`assets/voices/` 和 `save/` 文件夹，并生成默认剧情文件
- ⌨️ **简单操作** – 方向键选择，回车/空格确认，ESC 返回菜单，`q` 键随时安全退出
- 🔒 **安全退出** – 无论正常退出、按 `q` 键、选择“退出”菜单还是意外 panic，终端均能恢复，不留控制字符
- 📦 **纯 Rust 实现** – 基于 `ratatui`、`crossterm` 和 `image` 库，轻量且跨平台

---

## 🚀 快速开始

### 安装

#### 从 crates.io 安装

```bash
cargo install ngal
```

#### 从源码编译（推荐）

确保已安装 Rust（1.70+）：

```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

编译后的可执行文件位于 target/release/ngal，可将其添加到 PATH。

### 运行

直接执行：

```bash
ngal
```

首次运行会在当前目录下创建以下结构：

```
.
├── assets/
│   ├── config.json           # 配置文件（自动生成）
│   ├── dialogue.json         # 默认剧情文件（自动生成）
│   ├── portraits/            # 角色立绘目录（自动创建，需手动添加图片）
│   │   └── 角色名.png        # 图片文件名需与 speaker 一致
│   ├── music/                # 背景音乐目录
│   │   └── bgm.mp3           # 默认背景音乐（可选）
│   └── voices/               # 角色语音目录
│       └── 角色名.mp3        # 默认语音文件（可选）
├── save/
│   └── save.json             # 存档文件（首次存档后生成）
└── ngal                      # 项目执行程序
```

## 按键说明

- 空格/Enter 推进到下一句
- ↑/↓ 移动光标
- Enter 选择选项
- S/L 存档/读档
-ESC/q 退出程序

---

## 📖 自定义剧情

所有剧情数据存储在 assets/dialogue.json 中。你可以按以下格式编写自己的故事：

```json
{
  "title": "你的游戏标题",
  "footer": "底部提示文字 | q 退出",
  "scenes": {
    "scene_id_1": {
      "dialogue": [
        { "music": "bgm_scene1.mp3" },
        { "speaker": "角色A", "text": "第一句话", "voice": "a_line1.mp3" },
        { "speaker": "角色B", "text": "第二句话" },
        { "speaker": "角色A", "text": "第三句话" }
      ],
      "options": [
        { "text": "选项文字1", "next_scene": "target_scene_1" },
        { "text": "选项文字2", "next_scene": "target_scene_2" }
      ]
    },
    "target_scene_1": {
      "dialogue": [
        { "speaker": "角色C", "text": "你选择了选项1" }
      ],
      "options": []
    }
  },
  "initial_scene": "scene_id_1"
}
```

## 字段说明

- title：主菜单顶部显示的标题（支持 emoji 和普通文字）
- footer：主菜单底部文本框的提示文字（可加入操作说明）
- scenes：场景字典，键为场景 ID，值为场景数据
- dialogue：对话数组，按顺序显示。每条对话可以包含：
  - speaker：说话人（必须与 text 同时出现）
  - text：台词内容（必须与 speaker 同时出现）
  - voice：可选，指定语音文件名（如 "voice": "a_line1.mp3"），若不指定则默认使用 {speaker}.mp3
  - music：可选，单独一行用于切换背景音乐（如 { "music": "bgm_scene1.mp3" }）
- options：选项列表（可为空）。当该场景所有对话结束后会显示选项，选择后跳转到 next_scene
- initial_scene：游戏开始时的第一个场景 ID

### 提示

- 说话人可以是任意字符串，程序会尝试在 assets/portraits/ 中加载同名图片（如 角色A.png），若不存在则显示 ASCII 占位。
- 如果某场景的 options 为空，对话结束后会自动返回主菜单。
- 场景中可以任意切换说话人，无需为每个角色单独创建场景。
- 音乐指令会立即切换背景音乐，并可以放在对话数组任意位置（通常放在开头）。

---

## 🎵 音频支持

### 背景音乐

- 将音乐文件（MP3、WAV、OGG 等）放入 assets/music/ 目录
- 在 JSON 中使用 { "music": "filename.mp3" } 切换背景音乐
- 音量可在设置菜单中调节（默认 70%）

### 角色语音

- 将语音文件放入 assets/voices/ 目录
- 在 JSON 中使用 "voice": "filename.mp3" 指定语音（可选）
- 若不指定 voice 字段，则默认使用 {speaker}.mp3
- 音量可在设置菜单中独立调节（默认 80%）

### 音频要求

- 需要安装 mpv 播放器（Termux: pkg install mpv，Linux: sudo apt install mpv，Windows: 安装 mpv 并添加到 PATH）
- 支持 mpv 能播放的所有音频格式

---

## ⚙️ 配置文件

配置文件保存在 assets/config.json，首次运行自动生成：

```json
{
  "bgm_volume": 70,
  "voice_volume": 80,
  "version": "0.3.0"
}
```

- bgm_volume：背景音乐音量（0-100）
- voice_volume：角色语音音量（0-100）
- version：版本号（自动从 Cargo.toml 读取）

---

## 💾 存档机制

- 存档保存在 save/save.json（JSON 格式）
- 包含当前场景 ID、对话索引、菜单选中项等信息
- 可在任何状态下（主菜单、对话、选项界面）按 S 存档，按 L 读档
- 读档后自动恢复当前对话的语音播放

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！如果你想添加新功能，请先开 Issue 讨论。

开发环境准备：

```bash
cargo build
cargo run
```

代码风格使用 rustfmt，提交前请运行：

```bash
cargo fmt
cargo clippy
```

---

## 📄 许可证

本项目采用 MIT 许可证。详情参见 LICENSE 文件。

---

## 🙏 致谢

- Ratatui – 强大的终端 UI 库
- Crossterm – 跨平台终端处理
- image – 图片解码和缩放
- mpv – 强大的媒体播放器（用于音频播放）
- 所有贡献者和测试者

---

现在就开始你的终端冒险吧！ 🎉
