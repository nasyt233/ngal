use std::collections::HashMap;
use std::fs;
use std::io::{self, stdout};
use std::path::Path;
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

// ---------- 默认对话与配置 ----------
const DEFAULT_DIALOGUE: &str = r#"{
  "title": "原神 VS 鸣朝",
  "footer": "按回车继续",
  "scenes": {
    "start": {
      "speaker": "NAS油条",
      "lines": [
        "本项目由Rust语言开发，按回车键继续。",
        "哪个游戏牛逼?"
      ],
      "options": [
        { "text": "原神牛逼👍", "next_scene": "ysnb" },
        { "text": "鸣朝牛逼👍", "next_scene": "mcnb" }
      ]
    },
    "ysnb": {
      "speaker": "鸣朝",
      "lines": [
        "鸣朝才牛逼😡",
        "原神不牛逼🤓"
      ],
      "options": [
        { "text": "鸣朝牛逼", "next_scene": "hnb" }
      ]
    },
    "mcnb": {
      "speaker": "原神",
      "lines": [
        "原神才牛逼🤓👍",
        "鸣朝不牛逼😡"
      ],
      "options": [
        { "text": "原神牛逼", "next_scene": "hnb" }
      ]
    },
    "hnb": {
      "speaker": "NAS油条",
      "lines": [
        "😋他们产的片才牛逼😋",
        "游戏结束"
      ],
      "options": []
    }
  },
  "initial_scene": "start"
}"#;

// ---------- 数据模型 ----------

#[derive(Debug, Clone, Deserialize)]
struct SceneData {
    speaker: String,
    lines: Vec<String>,
    options: Vec<OptionData>,
}

#[derive(Debug, Clone, Deserialize)]
struct OptionData {
    text: String,
    next_scene: String,
}

#[derive(Debug, Deserialize)]
struct DialogueDB {
    title: String,
    footer: String,
    scenes: HashMap<String, SceneData>,
    initial_scene: String,
}

// ---------- 游戏状态 ----------

#[derive(Serialize, Deserialize, Clone)]
enum AppState {
    Menu,
    InDialogue {
        scene_id: String,
        line_index: usize,
    },
    InChoice {
        scene_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    state: AppState,
    menu_selected: usize,
}

// ---------- 辅助枚举 ----------
enum ChoiceAction {
    Select,
    Exit,
    Save,
    Load,
}

// ---------- 主应用 ----------

struct App {
    state: AppState,
    menu_options: Vec<String>,
    selected: usize,
    status_message: Option<String>,
    db: DialogueDB,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::ensure_directories()?;
        let db_content = Self::ensure_dialogue_file()?;
        let db: DialogueDB = serde_json::from_str(&db_content)?;
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
            db,
        })
    }

    fn ensure_directories() -> io::Result<()> {
        if !Path::new("assets").exists() {
            fs::create_dir("assets")?;
        }
        if !Path::new("save").exists() {
            fs::create_dir("save")?;
        }
        Ok(())
    }

    fn ensure_dialogue_file() -> io::Result<String> {
        let path = Path::new("assets/dialogue.json");
        if !path.exists() {
            fs::write(path, DEFAULT_DIALOGUE)?;
            Ok(DEFAULT_DIALOGUE.to_string())
        } else {
            fs::read_to_string(path)
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
        self.state = AppState::InDialogue {
            scene_id: self.db.initial_scene.clone(),
            line_index: 0,
        };
        self.status_message = None;
    }

    fn save_game(&mut self) {
        let data = SaveData {
            state: self.state.clone(),
            menu_selected: self.selected,
        };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                let save_path = Path::new("save/save.json");
                if let Err(e) = fs::write(save_path, json) {
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
        let save_path = Path::new("save/save.json");
        match fs::read_to_string(save_path) {
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

    fn current_dialogue_line(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, line_index } => {
                if let Some(scene) = self.db.scenes.get(scene_id) {
                    if *line_index < scene.lines.len() {
                        return Some(scene.lines[*line_index].clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn current_speaker(&self) -> Option<String> {
        match &self.state {
            AppState::InDialogue { scene_id, .. } => {
                self.db.scenes.get(scene_id).map(|s| s.speaker.clone())
            }
            AppState::InChoice { scene_id, .. } => {
                self.db.scenes.get(scene_id).map(|s| s.speaker.clone())
            }
            _ => None,
        }
    }

    fn advance_dialogue(&mut self) {
        match &self.state {
            AppState::InDialogue { scene_id, line_index } => {
                if let Some(scene) = self.db.scenes.get(scene_id) {
                    let next_line = line_index + 1;
                    if next_line < scene.lines.len() {
                        self.state = AppState::InDialogue {
                            scene_id: scene_id.clone(),
                            line_index: next_line,
                        };
                    } else if !scene.options.is_empty() {
                        let options: Vec<(String, String)> = scene
                            .options
                            .iter()
                            .map(|opt| (opt.text.clone(), opt.next_scene.clone()))
                            .collect();
                        self.state = AppState::InChoice {
                            scene_id: scene_id.clone(),
                            options,
                            selected: 0,
                        };
                    } else {
                        self.state = AppState::Menu;
                    }
                } else {
                    self.state = AppState::Menu;
                }
            }
            _ => {}
        }
    }

    fn select_option(&mut self) {
        match &self.state {
            AppState::InChoice { options, selected, .. } => {
                if let Some((_, next_scene)) = options.get(*selected) {
                    self.state = AppState::InDialogue {
                        scene_id: next_scene.clone(),
                        line_index: 0,
                    };
                }
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, key: KeyCode) {
        self.status_message = None;

        match &mut self.state {
            AppState::Menu => {
                match key {
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
                }
                return;
            }
            AppState::InDialogue { .. } => {
                match key {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        self.advance_dialogue();
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
                }
                return;
            }
            AppState::InChoice { .. } => {}
        }

        let action = if let AppState::InChoice { options, selected, .. } = &mut self.state {
            let options_count = options.len();
            match key {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    None
                }
                KeyCode::Down => {
                    if *selected < options_count - 1 {
                        *selected += 1;
                    }
                    None
                }
                KeyCode::Enter => Some(ChoiceAction::Select),
                KeyCode::Esc => Some(ChoiceAction::Exit),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(ChoiceAction::Save),
                KeyCode::Char('l') | KeyCode::Char('L') => Some(ChoiceAction::Load),
                _ => None,
            }
        } else {
            None
        };

        if let Some(action) = action {
            match action {
                ChoiceAction::Select => self.select_option(),
                ChoiceAction::Exit => self.state = AppState::Menu,
                ChoiceAction::Save => self.save_game(),
                ChoiceAction::Load => self.load_game(),
            }
        }
    }
}

// ---------- UI ----------

const BOTTOM_HEIGHT: u16 = 8;

fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

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
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double);
    frame.render_widget(block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        AppState::Menu => {
            // 自定义标题
            let title = &app.db.title;
            let title_paragraph = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("✨", Style::default().fg(Color::Rgb(255, 255, 0))),
                    Span::raw(" "),
                    Span::styled(title, Style::default().fg(Color::Rgb(212, 112, 212)).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled("✨", Style::default().fg(Color::Rgb(255, 255, 0))),
                ]),
                Line::from(vec![
                    Span::styled("✦", Style::default().fg(Color::Rgb(200, 100, 255))),
                    Span::raw("   Genshin Impact   "),
                    Span::styled("✦", Style::default().fg(Color::Rgb(200, 100, 255))),
                ]),
                Line::from(vec![
                    Span::styled("★", Style::default().fg(Color::Rgb(255, 215, 0))),
                    Span::raw(" Terminal Edition "),
                    Span::styled("★", Style::default().fg(Color::Rgb(255, 215, 0))),
                ]),
            ])
            .alignment(Alignment::Center);

            let title_area = Rect {
                x: inner_area.x,
                y: inner_area.y,
                width: inner_area.width,
                height: 4,
            };
            frame.render_widget(title_paragraph, title_area);

            // 菜单列表
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
            let start_y = inner_area.y + (inner_area.height.saturating_sub(list_height + 4)) / 2 + 4;
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(30)) / 2,
                y: start_y,
                width: 30.min(inner_area.width),
                height: list_height.min(inner_area.height.saturating_sub(4)),
            };
            frame.render_widget(list, list_area);
        }
        AppState::InDialogue { scene_id, .. } => {
            if let Some(scene) = app.db.scenes.get(scene_id) {
                // ASCII 立绘占位（可替换为图片）
                let art = match scene.speaker.as_str() {
                    "NAS油条" => "   🍳  NAS油条  🍳",
                    "鸣朝"    => "   ⚔️  鸣朝  ⚔️",
                    "原神"    => "   ✨  原神  ✨",
                    _ => "   （暂无立绘）",
                };
                let text = format!("{}\n\n{}", art, scene.speaker);
                let para = Paragraph::new(text)
                    .style(Style::default().fg(Color::Rgb(212, 112, 212)))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                let para_area = Rect {
                    x: inner_area.x,
                    y: inner_area.y + (inner_area.height.saturating_sub(5)) / 2,
                    width: inner_area.width,
                    height: 5.min(inner_area.height),
                };
                frame.render_widget(para, para_area);
            }
        }
        AppState::InChoice { options, selected, .. } => {
            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, (text, _))| {
                    let style = if i == *selected {
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
                .block(Block::default().title("请选择：").borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = options.len() as u16 * 2;
            let list_area = Rect {
                x: inner_area.x + (inner_area.width.saturating_sub(40)) / 2,
                y: inner_area.y + (inner_area.height.saturating_sub(list_height)) / 2,
                width: 40.min(inner_area.width),
                height: list_height.min(inner_area.height),
            };
            frame.render_widget(list, list_area);
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
            "系统".to_string(),
            app.db.footer.clone(),
            app.status_message.as_deref(),
        ),
        AppState::InDialogue { .. } => (
            app.current_speaker().unwrap_or_else(|| "?".to_string()),
            app.current_dialogue_line().unwrap_or_else(|| "".to_string()),
            app.status_message.as_deref(),
        ),
        AppState::InChoice { .. } => (
            app.current_speaker().unwrap_or_else(|| "系统".to_string()),
            "请选择一项：".to_string(),
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
    let display_text = if let Some(status_msg) = status {
        status_msg
    } else {
        content.as_str()
    };
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
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().expect("初始化失败，请检查 assets/dialogue.json");

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break,
                _ => app.handle_event(key.code),
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}