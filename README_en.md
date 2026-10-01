# ngal - Terminal Visual Novel Engine

# 🎮 ngal - Terminal Visual Novel Engine


[中文说明](README.md)

> A visual novel engine written in Rust, allowing you to enjoy visual novels directly inside your terminal.
> 
> 

## ✨ Features

- 🤓 Automatic game scanning
- 🎨 Colorful UI with double-line border layout
- 📝 Built-in easy-to-use script editor
- 🖼️ Character portraits & background images (PNG / JPEG supported)
- 🎵 Background music & character voice playback (requires mpv)
- 📜 Branching choices & multiple endings
- 💾 Unlimited save slots
- ⌨️ Auto-play, typewriter text animation, dialogue history
- 🎨 Customizable background color
- 🧮 Variable arithmetic (`+ - * /`) and `if` conditional branching
- 📝 Inline comments (`#`) and escape sequences (`:` `"` `'` `\n` `\t`)
- 🕹️ System command execution `$(uptime)` to fetch system information such as announcements

## 🚀 Quick Start

### Installation

#### One‑click install script for Linux / Termux

```bash
bash -c "$(curl -L https://raw.gitcode.com/nasyt/ngal/raw/main/install.sh)"
```

#### Build from source

```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

#### Install via [crates.io](crates.io)

```bash
cargo install ngal
```

### Run

```bash
ngal              # Launch game in current directory
ngal mygame       # Launch game from specified folder
ngal --version    # Show version
ngal build        # Package your game
ngal edit         # Open built-in script editor
```

### Directory Structure

These folders will be auto‑created on first launch:

```Plain Text
assets/
├── game.json       # Game configuration
├── dialog/
│   ├── dialogue.ng # Main script file
│   └── xxx.ng      # Additional script files
├── portraits/      # Character portrait images
├── music/          # Background music files
└── voices/         # Character voice audio
save/               # Savegame directory
```

## 📖 Script Writing

The main script file is `assets/dialog/dialogue.ng`. Both `.ng` and `.txt` file extensions are supported.

### Basic Syntax Example

```ng
# ngal v1.0.4 demo script    # # marks a comment line

[welcome]               # [welcome] = entry point label
Chapter One             # Plain text without speaker name is narration
load:index              # Jump to another label inside current script
# External script: load:day1.ng:welcome

[index]                 # New label, do NOT leave blank lines between label and content
name = John             # Assign variable
bg:bg.png               # Load background image
System: Waiting for 2 seconds
sleep:2                 # Pause execution for 2 seconds, auto continue afterwards
System: 2 second wait finished.
music:bg.mp3            # Play background music
img:logo.png:2:50%      # Show portrait, position:1=left,2=center,3=right; scale:50%
System: Welcome to ngal engine!
System: Hello player:hello.mp3    # Dialogue with voice audio
System: Current time $(date)
System: User: $(whoami)\nDirectory: $(pwd)    # \n = line break
img:logo2.png           # Switch portrait
img:                    # Empty value hides portrait (same rule for bg / music)
System: Default name: {name}   # Insert variable into text
input:Enter your name:name     # Prompt user input and store result into variable
{name}: My name is {name}!

# Variable calculation
a = 13
System: a = {a}
b = 78
System: b = {b}
c = a + b
System: Sum result {c}

System: Make your choice
score = 10
System: Current score {score}
# Branch selection, separate options with |
choose:Accept adventure (+8 score):accept|Refuse adventure (-5 score):refuse

[accept]
System: You accepted the adventure!
score = score + 8
System: Score {score}
load:jx

[refuse]
System: You refused the adventure!
score = score - 5
System: Score {score}
load:jx

[jx]
System: Demonstrating if condition
if score >= 10: good_end # Jump to label if condition matches
load:bad_end # Runs if condition fails

[good_end]
end
System: Your score is >=10
System: 🤓 True End! Score {score}
load:exit

[bad_end]
System: Score below 10
System: 😭 Bad End. Score {score}
load:exit

[exit]
System: Game finished
bg:        # Clear background
music:     # Stop music playback
end        # Return to main menu
```

### Command Reference Table

|Command|Format|Description|
|---|---|---|
|Dialogue|`Speaker:Text`|Show character dialogue|
|Voice Dialogue|`Speaker:Text:voice.mp3`|Voice file stored in `assets/voices/`|
|Narration|`Plain text`|Narrative text without speaker name|
|Variable assign|`var = value`|Supports string and numeric values|
|Variable math|`var = expression`|Supports `+ - * /` and parentheses|
|User input|`input:prompt:var`|Read keyboard input into variable|
|Variable insert|`{var}`|Embed variable value inside text content|
|Portrait|`img:image.png:pos:scale%`|Position:1‑Left,2‑Center,3‑Right|
|Clear portrait|`img:`|Empty argument removes portrait|
|Background|`bg:image.png`|Image stretched to fill terminal|
|Clear background|`bg:`|Remove background|
|Sleep / wait|`sleep:0.5`|Pause script execution for seconds|
|BGM|`music:audio.mp3`|Audio file stored in `assets/music/`|
|Stop BGM|`music:`|Stop background music|
|Choice branch|`choose:opt1:label1|opt2:label2`|Separate multiple options with `|`|
|Conditional jump|`if condition:label`|Operators: `> < >= <= == !=`|
|Jump label|`load:label`|Jump to label inside current script|
|External jump|`load:file.ng:label`|Load another script file and jump|
|Exit game|`end`|Return to main menu|

### ⌨️ Key Bindings

|Key|Action|
|---|---|
|Space / Enter|Advance dialogue / Confirm selection|
|↑ / ↓|Navigate options / Scroll history|
|ESC|Back / Open menu|
|S|Save progress|
|L|Load save|
|H|Show dialogue history|
|A|Toggle auto‑play mode|
|T|Toggle typewriter animation|
|3 / 4|Adjust text display speed|
|B|Cycle through background colors|
|q|Return to menu / Quit|

Editor keys:
Type `h` inside editor to view help

## 📜 Dependencies

- **mpv** — Required for audio playback
- Rust 1.70+

## 📄 License

MIT