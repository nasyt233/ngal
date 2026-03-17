# 🎮 ngal 终端视觉小说引擎

ngal 是一个用 Rust 编写的终端 Galgame（视觉小说）引擎

让你在命令行中体验分支对话的乐趣

它拥有精美的双边框 UI、角色立绘占位、选项分支、存档/读档功能

并会自动创建项目所需的目录和默认剧情文件。

![演示](image/ys.png)


---

✨ 功能特色

- 🎨 华丽彩色标题 – 主菜单显示渐变色艺术字，带星形装饰
- 📜 分支对话 – 支持多场景、多选项的剧情树
- 💾 存档/读档 – 随时按 S 保存，按 L 加载（存档保存在 save/save.json）
- 🖼️ 立绘区域 – 预留顶部区域显示角色名或 ASCII 艺术占位
- 🛠 自动初始化 – 首次运行自动创建 assets/ 和 save/ 文件夹，并生成默认剧情文件
- ⌨️ 简单操作 – 方向键选择，回车/空格确认，ESC 返回菜单
- 📦 纯 Rust 实现 – 基于 ratatui 和 crossterm，轻量且跨平台

---

## 🚀 快速开始

安装

从源码编译（推荐）

确保已安装 Rust（1.70+）：

从crates.io社区获取资源

```bash
cargo install ngal
```

从github获取资源
```bash
git clone https://github.com/nasyt233/ngal.git
cd ngal
cargo build --release
```

编译后的可执行文件位于 target/release/ngal，可将其添加到 PATH。

使用 Cargo 安装

```bash
cargo install --git https://github.com/nasyt233/ngal.git
```

运行

直接执行：

```bash
ngal
```

首次运行会在当前目录下创建以下结构：

```
.
├── assets/
│   └── dialogue.json   # 默认剧情文件（自动生成）
└── save/
    └── save.json       # 存档文件（首次存档后生成）
```

## 按键说明

状态 按键 功能

主菜单 ↑/↓ 移动选项

 Enter 确认选择
 
对话中 空格 / Enter 推进到下一句

 ESC 返回主菜单
 
 S 快速存档
 
 L 快速读档
 
选项界面 ↑/↓ 移动光标

 Enter 选择选项
 
 ESC 返回主菜单
 
 S / L 存档 / 读档
 
全局 ESC 从主菜单退出程序

---

## 📖 自定义剧情

所有剧情数据存储在 assets/dialogue.json 中。你可以按以下格式编写自己的故事：

```json
{
  "scenes": {
    "scene_id_1": {
      "speaker": "角色名",
      "lines": ["第一句台词", "第二句台词", "..."],
      "options": [
        { "text": "选项显示文字", "next_scene": "目标场景ID" },
        { "text": "另一个选项", "next_scene": "another_scene" }
      ]
    },
    "another_scene": {
      "speaker": "另一个角色",
      "lines": ["..."],
      "options": []
    }
  },
  "initial_scene": "scene_id_1"
}
```

- speaker：当前说话人，显示在底部文本框左上角
- lines：台词数组，按空格逐句推进
- options：选项列表（可为空）。当台词结束后会显示选项，选择后跳转到 next_scene
- initial_scene：游戏开始时的第一个场景 ID

提示：如果某场景的 options 为空，台词结束后会自动返回主菜单。

---

## 💾 存档机制

- 存档保存在 save/save.json（JSON 格式）
- 包含当前场景 ID、台词索引、菜单选中项等信息
- 可在任何状态下（主菜单、对话、选项界面）按 S 存档，按 L 读档

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

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
- 所有贡献者和测试者

---

现在就开始你的终端冒险吧！ 🎉
