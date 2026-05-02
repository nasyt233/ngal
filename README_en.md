# 🎮 ngal - Terminal Visual Novel Engine

[简体中文 README](README.md)

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

**ngal** is a terminal Galgame engine written in Rust, bringing visual novel experiences to your command line.

## ✨ Features

- 🎨 Colorful interface with dual border design
- 🖼️ Character portraits / backgrounds (PNG/JPEG support)
- 🎵 Background music + character voice (requires mpv)
- 📜 Branching choices + multiple endings
- 💾 10 save slots
- ⌨️ Auto-play / text animation / history
- 🎨 Adjustable background color (deep purple/blue/green/red/gray/none)

## 🚀 Quick Start

### Installation

**Build from source**
```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

**Install from crates.io**

```bash
cargo install ngal
```

### Usage

```bash
ngal              # Run in current directory
ngal mygame       # Run with specified game directory
ngal --version    # Show version
```

### Directory Structure

First run automatically creates the following directories:

```
assets/
├── game.json       # Game configuration
├── dialog/
│   └── dialogue.txt # Dialogue file
├── portraits/       # Character portraits
├── music/           # Background music
└── voices/          # Character voices
save/                # Save directory
```

## ⌨️ Controls

| Key | Function |
|------|----------|
| Space/Enter | Advance dialogue / Confirm selection |
| ↑/↓ | Move selection / Scroll list |
| ESC | Return to previous / Exit menu |
| S | Save game |
| L | Load game |
| H | History |
| A | Toggle auto-play |
| T | Toggle text animation |
| 3/4 | Adjust text speed |
| B | Switch background color |
| q | Quit program |

## 📜 Dependencies

- **mpv** - Audio playback (required)
- **Rust** 1.70+

## 📄 License

MIT