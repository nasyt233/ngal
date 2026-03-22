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
use serde::{Deserialize, Serialize};

// ---------- 数据结构 ----------

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

#[derive(Serialize, Deserialize, Clone)]
enum AppState {
    Menu,
    InDialogue(Scene),
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    state: AppState,
    menu_selected: usize,
}

struct App {
    state: AppState,
    menu_options: Vec<String>,
    selected: usize,
    status_message: Option<String>,
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

    fn execute_menu(&mut self) {
        match self.selected {
            0 => self.start_game(),
            1 => self.save_game(),
            2 => self.load_game(),
            3 => std::process::exit(0),
            _ => {}
        }
    }

    fn start_game(&mut self) {
        // 这里可以扩展为角色选择菜单，现在简单进入钟离
        let scene = Scene::new(
            "钟离",
            vec![
                "钟离：欲买桂花同载酒，只可惜故人...".to_string(),
                "钟离：此情此景，竟让我想起了若陀。".to_string(),
                "钟离：旅行者，要一起喝杯茶吗？".to_string(),
            ],
        );
        self.state = AppState::InDialogue(scene);
        self.status_message = None;
    }

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

    fn handle_event(&mut self, key: KeyCode) {
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
                        self.state = AppState::Menu;
                    }
                }
                KeyCode::Esc => {
                    self.state = AppState::Menu;
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

// ---------- UI ----------

/// 底部文本框固定高度（可根据终端高度动态调整，但保持固定较好）
const BOTTOM_HEIGHT: u16 = 8;

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

    render_top(frame, chunks[0], app);
    render_bottom(frame, chunks[1], app);
}

/// 渲染顶部区域（立绘或菜单）
fn render_top(frame: &mut Frame, area: Rect, app: &mut App) {
    // 顶部区域边框
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .title("立绘区域")
        .title_alignment(Alignment::Center);
    frame.render_widget(block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        AppState::Menu => {
            // 菜单列表居中
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
                width: 30.min(inner_area.width),
                height: list_height.min(inner_area.height),
            };
            frame.render_widget(list, list_area);
        }
        AppState::InDialogue(scene) => {
            // 显示角色名作为立绘占位，可替换为 ASCII 艺术
            let art = match scene.speaker.as_str() {
                "钟离" => "   ⠀⠀⢀⣠⣤⣶⣶⣶⣤⣄⡀⠀⠀",
                "温迪" => "   ⠀⣠⣾⣿⣿⣿⣿⣿⣿⣿⣷⣄⠀",
                "雷电将军" => "   ⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦",
                _ => "   （暂无立绘）",
            };
            let text = format!("{}\n\n{}", art, scene.speaker);
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(212, 112, 212)))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            // 居中显示
            let para_area = Rect {
                x: inner_area.x,
                y: inner_area.y + (inner_area.height.saturating_sub(5)) / 2,
                width: inner_area.width,
                height: 5.min(inner_area.height),
            };
            frame.render_widget(para, para_area);
        }
    }
}

/// 渲染底部区域（说话人名字 + 文本框）
fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let name_area = chunks[0];
    let text_area = chunks[1];

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
            scene.speaker.as_str(),
            scene.current_text().map(|s| s.as_str()).unwrap_or(""),
            app.status_message.as_deref(),
        ),
    };

    // 名字
    let name_style = Style::default()
        .fg(Color::Rgb(255, 255, 255))
        .add_modifier(Modifier::BOLD);
    let name_paragraph = Paragraph::new(Line::from(Span::styled(speaker, name_style)))
        .alignment(Alignment::Left);
    frame.render_widget(name_paragraph, name_area);

    // 文本框内容
    let display_text = status.unwrap_or(content);
    let text_style = if status.is_some() {
        Style::default().fg(Color::Rgb(255, 255, 0))
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

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break, // 退出程序
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