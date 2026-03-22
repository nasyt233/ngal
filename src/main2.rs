use std::io::{self, stdout, Stdout};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

// 游戏状态
enum AppState {
    Menu,      // 选项菜单
    Dialogue,  // 对话显示
}

struct App {
    state: AppState,
    options: Vec<String>,
    selected: usize,
    dialogue: String,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Menu,
            options: vec![
                "前往蒙德".to_string(),
                "前往璃月".to_string(),
                "前往稻妻".to_string(),
            ],
            selected: 0,
            dialogue: "你好，旅行者...".to_string(),
        }
    }

    fn handle_event(&mut self, key: KeyCode) {
        match self.state {
            AppState::Menu => match key {
                KeyCode::Up => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected < self.options.len() - 1 {
                        self.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    self.dialogue = format!("你选择了：{}", self.options[self.selected]);
                    self.state = AppState::Dialogue;
                }
                _ => {}
            },
            AppState::Dialogue => {
                if let KeyCode::Char(' ') = key {
                    // 回到菜单（实际游戏中应进入下一段对话）
                    self.state = AppState::Menu;
                }
            }
        }
    }
}

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
            if key.code == KeyCode::Esc {
                break;
            }
            app.handle_event(key.code);
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

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.size();

    // 创建一个带双边框的块（粉紫色）
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double)
        .title("原神牛逼")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(212, 112, 212)));

    // 内部绘制区域（去掉边框占用的空间）
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    match app.state {
        AppState::Menu => {
            // 准备列表项
            let items: Vec<ListItem> = app
                .options
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
                .block(Block::default().title("请选择你的旅程：").borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            // 计算列表居中显示的位置
            let list_height = app.options.len() as u16 * 2; // 每项占2行（含间距）
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(30)) / 2,
                y: inner_area.y + (inner_area.height.saturating_sub(list_height)) / 2,
                width: 30,
                height: list_height,
            };
            frame.render_widget(list, list_area);
        }
        AppState::Dialogue => {
            let paragraph = Paragraph::new(app.dialogue.as_str())
                .style(Style::default().fg(Color::Rgb(255, 255, 255)))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::NONE));

            // 对话居中显示
            let para_area = Rect {
                x: inner_area.x + 4,
                y: inner_area.y + (inner_area.height / 2) - 1,
                width: inner_area.width - 8,
                height: 3,
            };
            frame.render_widget(paragraph, para_area);

            // 提示继续
            let hint = Paragraph::new("(按空格继续)")
                .style(Style::default().fg(Color::Rgb(100, 100, 100)))
                .alignment(Alignment::Center);
            let hint_area = Rect {
                x: inner_area.x + 4,
                y: inner_area.y + inner_area.height - 4,
                width: inner_area.width - 8,
                height: 1,
            };
            frame.render_widget(hint, hint_area);
        }
    }
}