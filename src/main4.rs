use std::io::{self, stdout, };
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
use serde::{Deserialize, Serialize};

// ---------- 游戏数据结构 ----------

/// 一个对话场景
#[derive(Serialize, Deserialize, Clone)]
struct Scene {
    lines: Vec<String>,
    current_line: usize,
    speaker: String, // 说话人名字
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
    Menu,               // 主菜单
    InDialogue(Scene),  // 对话中
}

/// 存档数据结构
#[derive(Serialize, Deserialize)]
struct SaveData {
    state: AppState,
    menu_selected: usize, // 菜单选中项（仅在菜单状态有用）
}

/// 主应用
struct App {
    state: AppState,
    menu_options: Vec<String>,
    selected: usize,
    status_message: Option<String>, // 临时状态消息（如存档成功）
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Menu,
            menu_options: vec![
                "开始游戏".to_string(),
                "存档".to_string(),
                "读档".to_string(),
                "退出".to_string(),
            ],
            selected: 0,
            status_message: None,
        }
    }

    /// 根据选中的菜单项执行操作
    fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),       // 开始游戏 -> 进入角色选择
            1 => self.save_game(),        // 存档
            2 => self.load_game(),        // 读档
            3 => std::process::exit(0),   // 退出
            _ => {}
        }
    }

    /// 开始游戏：显示角色选择（这里简化，直接进入GTI的对话）
    fn start_game(&mut self) {
        let scene = Scene::new(
            "GTI",
            vec![
                "钟离：欲买桂花同载酒，只可惜故人...".to_string(),
                "钟离：此情此景，竟让我想起了若陀。".to_string(),
                "钟离：旅行者，要一起喝杯茶吗？".to_string(),
            ],
        );
        self.state = AppState::InDialogue(scene);
        self.status_message = None;
    }

    /// 存档：将当前状态保存到文件
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

    /// 读档：从文件加载状态
    fn load_game(&mut self) {
        match std::fs::read_to_string("save.json") {
            Ok(json) => match serde_json::from_str::<SaveData>(&json) {
                Ok(data) => {
                    self.state = data.state;
                    self.selected = data.menu_selected;
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

    /// 处理按键事件
    fn handle_event(&mut self, key: KeyCode) {
        // 每次按键清除状态消息（除非是新设置的消息）
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
                    }
                }
                KeyCode::Esc => {
                    self.state = AppState::Menu; // 强制返回菜单
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // 对话中按 S 键快速存档
                    self.save_game();
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    // 对话中按 L 键快速读档
                    self.load_game();
                }
                _ => {}
            },
        }
    }
}

// ---------- UI 布局 ----------

/// 底部固定高度（文本框区域）
const BOTTOM_HEIGHT: u16 = 8;

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.size();

    // 垂直分割：上部分（立绘/菜单区域）和下部分（文本框区域）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(BOTTOM_HEIGHT)),
            Constraint::Length(BOTTOM_HEIGHT),
        ])
        .split(area);

    let top_area = chunks[0];
    let bottom_area = chunks[1];

    // 绘制顶部区域（立绘/菜单）
    render_top(frame, top_area, app);

    // 绘制底部区域（名字 + 文本框）
    render_bottom(frame, bottom_area, app);
}

/// 渲染顶部区域
fn render_top(frame: &mut Frame, area: Rect, app: &App) {
    // 整个顶部区域用一个半透明边框包起来，表示这是立绘区域
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .title("三角恋")
        .title_alignment(Alignment::Center);
    frame.render_widget(block, area);

    // 内部绘制区域（去掉边框边距）
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        AppState::Menu => {
            // 菜单选项居中显示
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
            // 这里可以放置角色立绘，暂时留空
            // 可以在左上角显示角色名字（但底部已有名字）
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

/// 渲染底部区域（名字 + 文本框）
fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    // 将底部区域分成上下两部分：名字区（1行）和文本框区（剩余）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let name_area = chunks[0];
    let text_area = chunks[1];

    // 根据状态确定说话人和显示内容
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
            scene.speaker.as_str(),   // 关键修复：将 &String 转为 &str
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

    // 文本框内容：如果有状态消息则优先显示，否则显示正常内容
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
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用
    let mut app = App::new();

    // 主循环
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break, // 按ESC退出程序
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