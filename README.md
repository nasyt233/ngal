# 🎮 ngal - 终端视觉小说引擎

[English REDME]('REDME_en.md')

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

**ngal** 是一个用 Rust 编写的终端 Galgame 引擎，让你在命令行中体验视觉小说的乐趣。

## ✨ 功能

- 🎨 彩色界面 + 双边框设计
- 🖼️ 图片立绘 / 背景（支持 PNG/JPEG）
- 🎵 背景音乐 + 角色语音（需 mpv）
- 📜 分支选项 + 多结局
- 💾 10 个存档槽位
- ⌨️ 自动播放 / 文字动画 / 历史记录
- 🎨 可调背景色（深紫/深蓝/深绿/深红/灰色/无色）

## 🚀 快速开始

### 安装

**从源码编译**
```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

**从 crates.io 安装**

```bash
cargo install ngal
```

### 运行

```bash
ngal              # 当前目录运行
ngal mygame     # 指定游戏目录运行
ngal --version    # 显示版本
```

### 目录结构

首次运行自动创建以下目录：

```
assets/
├── game.json       # 游戏配置
├── dialog/
│   └── dialogue.txt # 剧情文件
├── portraits/       # 图片立绘
├── music/           # 背景音乐
└── voices/          # 角色语音
save/                # 存档目录
```

## ⌨️ 按键说明

| 按键 | 功能 |
|------|------|
| 空格/Enter | 推进对话 / 确认选项 |
| ↑/↓ | 移动选项 / 滚动列表 |
| ESC | 返回上一级 / 退出菜单 |
| S | 存档 |
| L | 读档 |
| H | 历史记录 |
| A | 自动播放开关 |
| T | 文字动画开关 |
| 3/4 | 文字速度调节 |
| B | 切换背景色 |
| q | 退出程序 |

## 📜 依赖

- **mpv** - 音频播放（必需）
- **Rust** 1.70+

## 📄 许可证

MIT
