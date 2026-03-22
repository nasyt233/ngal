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
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

// ---------- 游戏数据结构 ----------

/// 一个对话场景：包含多句台词
struct Scene {
    lines: Vec<String>,      // 对话内容列表
    current_line: usize,     // 当前显示到第几句
}

impl Scene {
    fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            current_line: 0,
        }
    }

    /// 获取当前台词，如果已经结束则返回 None
    fn current_text(&self) -> Option<&String> {
        if self.current_line < self.lines.len() {
            Some(&self.lines[self.current_line])
        } else {
            None
        }
    }

    /// 推进到下一句，返回是否还有更多
    fn next_line(&mut self) -> bool {
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;
            true
        } else {
            false
        }
    }

    /// 重置场景
    fn reset(&mut self) {
        self.current_line = 0;
    }
}

/// 游戏全局状态
enum AppState {
    Menu,               // 主菜单
    InDialogue(Scene),  // 对话中，携带当前场景
}

struct App {
    state: AppState,
    menu_options: Vec<String>,   // 主菜单选项
    selected: usize,             // 当前选中的菜单项
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Menu,
            menu_options: vec![
                "钟离".to_string(),
                "温迪".to_string(),
                "雷电将军".to_string(),
                "退出".to_string(),
            ],
            selected: 0,
        }
    }

    /// 根据选中的菜单项进入对应场景
    fn enter_scene(&mut self) {
        match self.selected {
            0 => { // 钟离
                let scene = Scene::new(vec![
                    "钟离：欲买桂花同载酒，只可惜故人...".to_string(),
                    "钟离：此情此景，竟让我想起了若陀。".to_string(),
                    "钟离：旅行者，要一起喝杯茶吗？".to_string(),
                ]);
                self.state = AppState::InDialogue(scene);
            }
            1 => { // 温迪
                let scene = Scene::new(vec![
                    "温迪：呀吼！旅行者，要听我唱首歌吗？".to_string(),
                    "温迪：咳咳... 在风起的地方...".to_string(),
                    "温迪：啊，好像跑调了，嘿嘿～".to_string(),
                ]);
                self.state = AppState::InDialogue(scene);
            }
            2 => { // 雷电将军
                let scene = Scene::new(vec![
                    "雷电将军：永恒... 近在眼前。".to_string(),
                    "雷电将军：你的愿望，我感受到了。".to_string(),
                    "雷电将军：但此刻，我只想与你共赏这稻妻的樱花。".to_string(),
                ]);
                self.state = AppState::InDialogue(scene);
            }
            3 => { // 退出
                std::process::exit(0);
            }
            _ => {}
        }
    }

    /// 处理按键事件
    fn handle_event(&mut self, key: KeyCode) {
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
                    self.enter_scene();
                }
                _ => {}
            },
            AppState::InDialogue(scene) => {
                match key {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        // 空格或回车：推进对话
                        if !scene.next_line() {
                            // 对话结束，返回菜单
                            self.state = AppState::Menu;
                        }
                    }
                    KeyCode::Esc => {
                        // 按ESC强制返回菜单
                        self.state = AppState::Menu;
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------- UI 渲染 ----------

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.size();

    // ---------- 主边框（粉紫色双边框） ----------
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double)
        .title("原神牛逼")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(212, 112, 212)));
    frame.render_widget(main_block, area);

    // 内部绘制区域（去掉边框占用的边距）
    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    match &app.state {
        AppState::Menu => render_menu(frame, inner_area, app),
        AppState::InDialogue(scene) => render_dialogue(frame, inner_area, scene),
    }
}

/// 渲染主菜单
fn render_menu(frame: &mut Frame, area: Rect, app: &App) {
    // 准备列表项
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
        .block(Block::default().title("请选择你的角色：").borders(Borders::NONE))
        .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

    // 计算列表居中显示
    let list_height = app.menu_options.len() as u16 * 2; // 每项占2行
    let list_area = Rect {
        x: area.x + (area.width.saturating_sub(30)) / 2,
        y: area.y + (area.height.saturating_sub(list_height)) / 2,
        width: 30,
        height: list_height,
    };
    frame.render_widget(list, list_area);
}

/// 渲染对话场景
fn render_dialogue(frame: &mut Frame, area: Rect, scene: &Scene) {
    if let Some(text) = scene.current_text() {
        // 对话内容（可换行）
        let paragraph = Paragraph::new(text.as_str())
            .style(Style::default().fg(Color::Rgb(255, 255, 255)))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::NONE));

        // 对话区域（上方留白）
        let text_area = Rect {
            x: area.x + 4,
            y: area.y + 4,
            width: area.width - 8,
            height: area.height - 8,
        };
        frame.render_widget(paragraph, text_area);

        // 右下角提示
        let hint = if scene.current_line + 1 < scene.lines.len() {
            "(按空格继续)"
        } else {
            "(对话结束，按空格返回菜单)"
        };
        let hint_paragraph = Paragraph::new(hint)
            .style(Style::default().fg(Color::Rgb(100, 100, 100)))
            .alignment(Alignment::Right);
        let hint_area = Rect {
            x: area.x + 4,
            y: area.y + area.height - 3,
            width: area.width - 8,
            height: 1,
        };
        frame.render_widget(hint_paragraph, hint_area);
    }
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
                KeyCode::Esc => break,          // 按ESC完全退出程序
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