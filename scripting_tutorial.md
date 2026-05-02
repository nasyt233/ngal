# ngal Scenario Writing Tutorial

## File Location

`assets/dialog/dialogue.txt`

## Basic Structure

```txt
[scene_name]
command1
command2
...
```

The game starts from the `[welcome]` scene.

## Complete Example

```txt
[welcome]
music:title.mp3
img:NASyoutiao.png:3:50%
NASyoutiao:This project is developed with Rust language\nPress Enter to continue:nas_intro.mp3
img:
input:Please enter your name:name
NASyoutiao:Alright, {name}, which game is the best?:gamenb.mp3
choose:Genshin is the best:ysnb|Wuthering Waves is the best:mcnb|Endless Starlight is the best:zmd

[ysnb]
bg:91.png
img:Wuthering Waves.png:3:35%
Wuthering Waves:Wuthering Waves is the best 😡:mcnb.mp3
Wuthering Waves:Genshin is not the best 🤓:ys_no_nb.mp3
Genshin:Genshin is the best 🤓👍:ysnb.mp3
Genshin:Wuthering Waves is not the best 😡:mc_no_nb.mp3
load:ytnb

[mcnb]
img:Genshin.png
Genshin:Genshin is the best 🤓👍:ysnb.mp3
Genshin:Wuthering Waves is not the best 😡:mc_no_nb.mp3
Wuthering Waves:Wuthering Waves is the best 😡:mcnb.mp3
Wuthering Waves:Genshin is not the best 🤓:ys_no_nb.mp3
load:ytnb

[zmd]
img:Endless Starlight.png
Endless Starlight:I am the best
end

[ytnb]
img:Me.png
{name}:😋Their productions are the best 😋:ysmcnb.mp3
bg:
NASyoutiao:Game over
end
```

## Command Reference

### Dialogue and Narration

| Command | Format | Description |
|---------|--------|-------------|
| Dialogue | `speaker:text` | Display text only |
| Dialogue | `speaker:text:voice.mp3` | Voice file stored in assets/voices/ directory |
| Narration | `text content` | Displayed as narration without speaker |

### Variables

| Command | Format | Description |
|---------|--------|-------------|
| Variable Input | `input:prompt:variable_name` | Store user input as a variable |
| Variable Usage | `{variable_name}` | Replace with variable value in text |

### Images and Background

| Command | Format | Description |
|---------|--------|-------------|
| Portrait | `img:filename.png` | Stored in assets/portraits/ directory |
| Portrait Position/Scale | `img:file:1\|2\|3:percentage%` | 1=left 2=center 3=right |
| Clear Portrait | `img:` | Empty clears current portrait |
| Background | `bg:filename.png` | Stretched to fill entire background |
| Clear Background | `bg:` | Empty clears background image |

### Audio

| Command | Format | Description |
|---------|--------|-------------|
| Background Music | `music:filename.mp3` | Stored in assets/music/ directory |
| Stop Music | `music:` | Empty stops currently playing music |

### Flow Control

| Command | Format | Description |
|---------|--------|-------------|
| Choice Branch | `choose:option1:scene1\|option2:scene2` | Use vertical bar to separate multiple options |
| Jump | `load:scene_name` | Jump to specified scene |
| End Game | `end` | Return to main menu |

## Path Notes

- Image, voice, and music files are all placed in corresponding subdirectories under `assets/`
- Filenames support Chinese characters and spaces

## Important Notes

- `\n` in text represents a line break
- Commands and parameters are case-sensitive
- Scene names can only contain letters, numbers, and underscores
- Ensure file encoding is UTF-8 (without BOM)