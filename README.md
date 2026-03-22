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

从源码编译（推荐）

确保已安装 Rust（1.70+）：


### 从crates.io社区获取资源

```bash
# 下载源码构建
cargo install ngal

# 添加环境变量
PATH=$HOME/.cargo/bin:$PATH

```

### 从github获取资源
```bash
git clone https://github.com/nasyt233/ngal.git

cd ngal

cargo build --release

# 编译后的可执行文件位于 target/release/ngal
# 可将其添加到 PATH环境
```


### 运行

直接执行：

```bash
ngal
```

首次运行会在当前目录下创建以下结构：

```
.
├── assets/
│   ├── dialogue.json   # 默认剧情文件（自动生成）
│   └── portraits/      # 角色立绘目录（自动创建，需手动添加图片）
│        └──我.png      # 图片的名字 (自行导入) 对应角色的名字，并且自动显示
├── save/
│   └── save.json       # 存档文件（首次存档后生成）
└── ngal                # 项目执行程序
```

## 按键说明
 
- 空格/Enter 推进到下一句
- ↑/↓ 移动光标
- Enter 选择选项
- S/L  存档/读档
- ESC/q 退出程序

---

## 📖 自定义剧情

所有剧情数据存储在 assets/dialogue.json 中

你可以按以下格式编写自己的故事：

```json
{
  "title": "你的游戏标题",
  "footer": "底部提示文字 | q 退出",
  "scenes": {
    "scene_id_1": {
      "dialogue": [
        { "speaker": "角色A", "text": "第一句话" },
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

- title：主菜单顶部显示的标题（支持 emoji 和普通文字）
- footer：主菜单底部文本框的提示文字（可加入操作说明）
- scenes：场景字典，键为场景 ID，值为场景数据
- dialogue：对话数组，按顺序显示。每条对话包含 speaker（说话人）和 text（台词）
- options：选项列表（可为空）。当该场景所有对话结束后会显示选项，选择后跳转到 next_scene
- initial_scene：游戏开始时的第一个场景 ID

## ⚠ 提示：

- 说话人可以是任意字符串，程序会尝试在 assets/portraits/ 中加载同名图片（如 角色A.png），若不存在则显示 ASCII 占位。
- 如果某场景的 options 为空，对话结束后会自动返回主菜单。
- 场景中可以任意切换说话人，无需为每个角色单独创建场景。

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
- image – 图片解码和缩放
- 所有贡献者和测试者

---

现在就开始你的终端冒险吧！ 🎉
