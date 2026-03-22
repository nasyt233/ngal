use std::collections::HashMap;
use std::io::{self, stdout};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage}; // ✅ 修正导入
use serde::{Deserialize, Serialize};

// ---------- 游戏数据结构 ----------

/// 一个对话场景
#[derive(Serialize, Deserialize, Clone)]
struct Scene {
    lines: Vec<String>,
    current_line: usize,
    speaker: String,
}

impl Scene {
    fn new(speaker: &str, lines: Vec<String>) -> Self {
        Self {
            lines,
            current_line: 0,
            speaker: speaker.to_string(),
        }
    }

    fn current_text(&self) -> Option<&String> {
        if self.current_line < self.lines.len() {
            Some(&self.lines[self.current_line])
        } else {
            None
        }
    }

    fn next_line(&mut self) -> bool {
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;
            true
        } else {
            false
        }
    }
}

/// 游戏全局状态
#[derive(Serialize, Deserialize, Clone)]
enum AppState {
    Menu,
    InDialogue(Scene),
}

/// 存档数据结构
#[derive(Serialize, Deserialize)]
struct SaveData {
    state: AppState,
    menu_selected: usize,
}

/// 主应用
struct App {
    state: AppState,
    menu_options: Vec<String>,
    selected: usize,
    status_message: Option<String>,

    // 图片相关
    picker: Picker,
    image_protocol: Option<Box<dyn StatefulProtocol>>, // ✅ 现在正确
    character_images: HashMap<String, Box<dyn StatefulProtocol>>,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 创建图片探测器（自动探测终端支持的图形协议）
        let picker = Picker::from_query_stdio()?;

        // 加载角色图片（如果文件存在）
        let mut character_images = HashMap::new();
        let assets = [
            ("钟离", "./assets/zhongli.png"),
            ("温迪", "./assets/venti.png"),
            ("雷电将军", "./assets/raiden.png"),
        ];

        for (name, path) in assets {
            if let Ok(img) = image::ImageReader::open(path)?.decode() {
                let protocol = picker.new_resize_protocol(img);
                character_images.insert(name.to_string(), protocol);
            }
        }

        Ok(Self {
            state: AppState::Menu,
            menu_options: vec![
                "开始游戏".to_string(),
                "存档".to_string(),
                "读档".to_string(),
                "退出".to_string(),
            ],
            selected: 0,
            status_message: None,
            picker,
            image_protocol: None,
            character_images,
        })
    }

    /// 执行菜单操作
    fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => self.save_game(),
            2 => self.load_game(),
            3 => std::process::exit(0),
            _ => {}
        }
    }

    /// 开始游戏：进入钟离的对话
    fn start_game(&mut self) {
        let scene = Scene::new(
            "钟离",
            vec![
                "钟离：欲买桂花同载酒，只可惜故人...".to_string(),
                "钟离：此情此景，竟让我想起了若陀。".to_string(),
                "钟离：旅行者，要一起喝杯茶吗？".to_string(),
            ],
        );
        // 设置对应立绘
        self.image_protocol = self.character_images.get("钟离").cloned();
        self.state = AppState::InDialogue(scene);
        self.status_message = None;
    }

    /// 存档
    fn save_game(&mut self) {
        let data = SaveData {
            state: self.state.clone(),
            menu_selected: self.selected,
        };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = std::fs::write("save.json", json) {
                    self.status_message = Some(format!("存档失败：{}", e));
                } else {
                    self.status_message = Some("存档成功".to_string());
                }
            }
            Err(e) => {
                self.status_message = Some(format!("序列化失败：{}", e));
            }
        }
    }

    /// 读档
    fn load_game(&mut self) {
        match std::fs::read_to_string("save.json") {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(data) => {
                    self.state = data.state;
                    self.selected = data.menu_selected;
                    // 根据当前对话角色更新立绘
                    self.update_image_from_state();
                    self.status_message = Some("读档成功".to_string());
                }
                Err(e) => {
                    self.status_message = Some(format!("解析存档失败：{}", e));
                }
            },
            Err(e) => {
                self.status_message = Some(format!("读取存档失败：{}", e));
            }
        }
    }

    /// 根据当前状态更新显示的立绘
    fn update_image_from_state(&mut self) {
        match &self.state {
            AppState::InDialogue(scene) => {
                self.image_protocol = self.character_images.get(&scene.speaker).cloned();
            }
            _ => {
                self.image_protocol = None;
            }
        }
    }

    /// 处理按键事件
    fn handle_event(&mut self, key: KeyCode) {
        // 清除状态消息（除非马上又设置）
        self.status_message = None;

        match &mut self.state {
            AppState::Menu => match key {
                KeyCode::Up => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected < self.menu_options.len() - 1 {
                        self.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    self.execute_menu();
                }
                _ => {}
            },
            AppState::InDialogue(scene) => match key {
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if !scene.next_line() {
                        // 对话结束，返回菜单
                        self.state = AppState::Menu;
                        self.image_protocol = None;
                    }
                }
                KeyCode::Esc => {
                    self.state = AppState::Menu;
                    self.image_protocol = None;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.save_game();
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.load_game();
                }
                _ => {}
            },
        }
    }
}

// ---------- UI 布局 ----------

const BOTTOM_HEIGHT: u16 = 8; // 底部文本框固定高度

fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

    // 垂直分割：上部分（立绘/菜单）和下部分（文本框）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(BOTTOM_HEIGHT)),
            Constraint::Length(BOTTOM_HEIGHT),
        ])
        .split(area);

    let top_area = chunks[0];
    let bottom_area = chunks[1];

    render_top(frame, top_area, app);
    render_bottom(frame, bottom_area, app);
}

/// 渲染顶部区域（立绘或菜单）
fn render_top(frame: &mut Frame, area: Rect, app: &mut App) {
    // 顶部区域边框（表示立绘区域）
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .title("立绘区域")
        .title_alignment(Alignment::Center);
    frame.render_widget(block, area);

    // 内部绘制区域（去掉边框）
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        AppState::Menu => {
            // 菜单选项列表
            let items: Vec<ListItem> = app
                .menu_options
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    let style = if i == app.selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(200, 200, 200))
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = app.menu_options.len() as u16 * 2;
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(30)) / 2,
                y: inner_area.y + (inner_area.height.saturating_sub(list_height)) / 2,
                width: 30,
                height: list_height,
            };
            frame.render_widget(list, list_area);
        }
        AppState::InDialogue(scene) => {
            if let Some(protocol) = &mut app.image_protocol {
                // 有立绘：显示图片，占满整个内部区域
                let image_widget = StatefulImage::default();
                // protocol 是 &mut Box<dyn StatefulProtocol>，可以直接传给需要 &mut dyn StatefulProtocol 的参数
                frame.render_stateful_widget(image_widget, inner_area, protocol);
            } else {
                // 无立绘：显示占位文字
                let text = format!("正在与 {} 对话中...", scene.speaker);
                let para = Paragraph::new(text)
                    .style(Style::default().fg(Color::Rgb(180, 180, 180)))
                    .alignment(Alignment::Center);
                let para_area = Rect {
                    x: inner_area.x,
                    y: inner_area.y + inner_area.height / 2 - 1,
                    width: inner_area.width,
                    height: 1,
                };
                frame.render_widget(para, para_area);
            }
        }
    }
}

/// 渲染底部区域（说话人名字 + 文本框）
fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    // 分割底部：名字区（1行）和文本框区（剩余）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let name_area = chunks[0];
    let text_area = chunks[1];

    // 根据状态确定说话人和显示内容
    let (speaker, content, status) = match &app.state {
        AppState::Menu => (
            "系统",
            match app.selected {
                0 => "开始新的旅程",
                1 => "保存当前进度",
                2 => "加载存档",
                3 => "退出游戏",
                _ => "",
            },
            app.status_message.as_deref(),
        ),
        AppState::InDialogue(scene) => (
            scene.speaker.as_str(), // 转为 &str 保证类型一致
            scene.current_text().map(|s| s.as_str()).unwrap_or(""),
            app.status_message.as_deref(),
        ),
    };

    // 绘制名字（白色加粗）
    let name_style = Style::default()
        .fg(Color::Rgb(255, 255, 255))
        .add_modifier(Modifier::BOLD);
    let name_paragraph = Paragraph::new(Line::from(Span::styled(speaker, name_style)))
        .alignment(Alignment::Left);
    frame.render_widget(name_paragraph, name_area);

    // 文本框内容：优先显示状态消息，否则显示对话/提示
    let display_text = status.unwrap_or(content);
    let text_style = if status.is_some() {
        Style::default().fg(Color::Rgb(255, 255, 0)) // 状态消息用黄色
    } else {
        Style::default().fg(Color::Rgb(255, 255, 255))
    };

    let text_paragraph = Paragraph::new(display_text)
        .style(text_style)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                .border_type(ratatui::widgets::BorderType::Double),
        );
    frame.render_widget(text_paragraph, text_area);
}

// ---------- 主函数 ----------

fn main() -> io::Result<()> {
    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用（如果图片加载失败会 panic，也可改为 unwrap_or_else 处理）
    let mut app = App::new().expect("初始化应用失败，请检查 assets 目录");

    // 主循环
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break, // 按 ESC 完全退出程序
                _ => app.handle_event(key.code),
            }
        }
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}