# 🎮 ngal - Terminal Visual Novel Engine

[简体中文 README](README.md)
![web introduce](index.html)

> A Rust-powered galgame engine that lets you enjoy visual novels right in your command-line interface.

## ✨ Features
- 🎨 Colorful UI with double-border layout
- 🖼️ Character sprites & background images (PNG/JPEG supported)
- 🎵 Background music & character voice lines (requires mpv)
- 📜 Branching choices & multiple endings
- 💾 10 save slots
- ⌨️ Auto-play, text animation, history log
- 🎨 Adjustable background colors (dark purple / dark blue / dark green / dark red / gray / transparent)
- 🧮 Variable arithmetic (`+ - * /`) and conditional `if` statements
- 📝 In-line comments (`#`) and escape characters (`:` `"` `'` `\n` `\t`)

## 🚀 Quick Start

### Installation

#### One-click install script
```bash
bash -c "$(curl -L https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh)"
```

#### Build from source
```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

#### Install from crates.io
```bash
cargo install ngal
```

### Run
```bash
ngal              # Run game in current directory
ngal mygame       # Run game from specified directory
ngal --version    # Show version info
```

### Directory Structure
The following directories are created automatically on first launch:
```
assets/
├── game.json       # Game configuration
├── dialog/
│   ├── dialogue.ng # Script file (.ng / .txt supported)
│   └── xxx.ng      # Additional script files
├── portraits/      # Character sprites
├── music/          # Background music
└── voices/         # Character voice files
save/               # Save data directory
```

## 📖 Script Writing

The main script file is `assets/dialog/dialogue.ng`. Both `.ng` and `.txt` file extensions are supported.

### Basic Syntax
```ng
# ngal example tutorial script    # # denotes comment

[welcome]               # [welcome] is entry point
Chapter 1               # Plain text without speaker name
load:index              # Jump to another scene; supports external file: load:day1.ng:welcome

[index]                 # Sub-scene
name = Jiahao           # Variable assignment
bg:bg.png               # Load background image
music:bgm.mp3           # Play background music
img:logo.png:2:50%      # Load sprite (1=left,2=center,3=right; 50% = scale)
System: Welcome to ngal engine!   # Dialogue with speaker name
img:                     # Empty to clear sprite (works for bg/music too)
System: Default name: {name}       # {var} interpolate variable; braces not needed in arithmetic
input:Please enter your name:name  # Read user input into variable
{name}: My name is {name}!         # Variable can also be used as speaker name

# Variable arithmetic
a = 13
System: a = {a}
b = 78
System: b = {b}
c = a + b               # Supports + - * /
System: Result of addition: {c}

System: Time for choices
score = 10
System: Current score: {score}
choose:Accept adventure(+8 score):accept|Refuse adventure(-5 score):refuse

[accept]
System: You accepted the adventure!
score = score + 8
System: Current score {score}
load:jx

[refuse]
System: You refused the adventure!
score = score - 5
System: Current score {score}
load:jx

[jx]
System: If condition demo
if score >= 10: good_end # Jump if condition holds
load:bad_end # Fall-through if condition fails

[good_end]
System: Score is greater or equal to 10
System: 🤓 Perfect ending! Score {score}
load:exit

[bad_end]
System: Score is less than 10
System: 😭 Bad ending. Score {score}
load:exit

[exit]
System: Game over
bg:    # Clear background
music: # Stop music
end    # Exit game
```

### Command Reference

| Command | Format | Description |
|---|---|---|
| Dialogue | `Speaker:text` | Show character dialogue |
| Dialogue with voice | `Speaker:text:voice.mp3` | Voice files go to `assets/voices/` |
| Narration | `Text content` | Text without speaker |
| Variable assignment | `var = value` | Supports strings and numbers |
| Variable calculation | `var = expression` | Supports `+ - * /` and parentheses |
| User input | `input:prompt:var` | Read user input into variable |
| Variable interpolation | `{var}` | Insert variable value into text |
| Sprite | `img:file.png:position:scale%` | Position: 1-left, 2-center, 3-right |
| Clear sprite | `img:` | Leave empty to remove sprite |
| Background image | `bg:file.png` | Stretch to fill screen |
| Clear background | `bg:` | Leave empty to remove background |
| Background music | `music:file.mp3` | Place in `assets/music/` |
| Stop music | `music:` | Leave empty to stop playback |
| Branch choices | `choose:opt1:scene1\|opt2:scene2` | Separate options with vertical bar |
| Conditional jump | `if condition:scene` | Supports `> < >= <= == !=` |
| Scene jump | `load:scene` | Jump to named scene |
| External scene jump | `load:file.ng:scene` | Load external script and jump |
| Quit game | `end` | Return to main menu |

### ⌨️ Key Bindings

| Key | Function |
|---|---|
| Space / Enter | Advance dialogue / confirm choice |
| ↑ / ↓ | Navigate choices / scroll lists |
| ESC | Go back / exit menu |
| S | Save game |
| L | Load game |
| H | Show history log |
| A | Toggle auto-play |
| T | Toggle text animation |
| 3 / 4 | Adjust text speed |
| B | Cycle background color |
| q | Return to menu / quit |

## 📜 Dependencies
- **mpv** — Required for audio playback
- Rust 1.70+

## 📄 License
MIT
