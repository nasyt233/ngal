// src/scanner.rs
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const MAX_DEPTH: usize = 4;
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    ".cache",
    ".npm",
    ".cargo",
];

#[derive(Clone, Copy, PartialEq)]
pub enum GameKind {
    Directory,
    Packed,
}

#[derive(Clone)]
pub struct GameEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: GameKind,
    pub rel_display: String,
}

enum ScanMsg {
    Found(GameEntry),
    Done,
}

// ============================================================
// 扫描
// ============================================================

fn spawn_scan(root: PathBuf) -> Receiver<ScanMsg> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let root_canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        scan_recursive(&root_canonical, &root_canonical, 0, &tx);
        let _ = tx.send(ScanMsg::Done);
    });
    rx
}

fn scan_recursive(dir: &Path, root: &Path, depth: usize, tx: &mpsc::Sender<ScanMsg>) {
    if depth > MAX_DEPTH {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut items: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        items.push((name, path));
    }
    items.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    for (name, path) in items {
        if path.is_dir() {
            if path.join("assets/game.json").exists() {
                let display_name = read_game_title(&path).unwrap_or(name.clone());
                let entry = GameEntry {
                    path: path.clone(),
                    name: display_name,
                    kind: GameKind::Directory,
                    rel_display: display_rel_path(&path, root),
                };
                if tx.send(ScanMsg::Found(entry)).is_err() {
                    return;
                }
                continue;
            }
            subdirs.push(path);
        } else if path.is_file() {
            if path.extension().and_then(|s| s.to_str()) == Some("ngal") {
                let display_name = name.trim_end_matches(".ngal").to_string();
                let entry = GameEntry {
                    path: path.clone(),
                    name: display_name,
                    kind: GameKind::Packed,
                    rel_display: display_rel_path(&path, root),
                };
                if tx.send(ScanMsg::Found(entry)).is_err() {
                    return;
                }
            }
        }
    }

    for subdir in subdirs {
        scan_recursive(&subdir, root, depth + 1, tx);
    }
}

fn read_game_title(dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("assets/game.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("title").and_then(|t| t.as_str()).map(String::from)
}

fn display_rel_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| {
            let s = p.to_string_lossy().to_string();
            if s.is_empty() {
                "(当前目录)".to_string()
            } else {
                format!("./{}", s)
            }
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn sort_games(games: &mut [GameEntry]) {
    games.sort_by(|a, b| {
        let ka: u8 = match a.kind {
            GameKind::Directory => 0,
            GameKind::Packed => 1,
        };
        let kb: u8 = match b.kind {
            GameKind::Directory => 0,
            GameKind::Packed => 1,
        };
        ka.cmp(&kb)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

// ============================================================
// 扫描界面
// ============================================================

pub fn run_scanner(root: &Path) -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let root_buf = root.to_path_buf();
    let mut rx = spawn_scan(root_buf.clone());

    let mut games: Vec<GameEntry> = Vec::new();
    let mut scanning = true;
    let mut selected = 0usize;
    let mut list_area = Rect::default();
    let mut last_click: Option<(Instant, usize)> = None;
    let mut chosen: Option<GameEntry> = None;

    // 新建游戏输入状态
    let mut creating = false;
    let mut new_name = String::new();
    let mut create_error: Option<String> = None;

    // 首帧
    terminal.draw(|f| {
        let area = f.size();
        list_area = draw_scanner(f, area, &games, selected, &root_buf, scanning);
        if creating {
            draw_new_game_dialog(f, area, &new_name, create_error.as_deref());
        }
    })?;

    loop {
        // ---------- 消费扫描消息 ----------
        let mut new_found = false;
        loop {
            match rx.try_recv() {
                Ok(ScanMsg::Found(entry)) => {
                    games.push(entry);
                    new_found = true;
                }
                Ok(ScanMsg::Done) => {
                    scanning = false;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    scanning = false;
                    break;
                }
            }
        }

        if new_found {
            let keep_path = games.get(selected).map(|g| g.path.clone());
            sort_games(&mut games);
            if let Some(p) = keep_path {
                if let Some(idx) = games.iter().position(|g| g.path == p) {
                    selected = idx;
                }
            }
        }

        // ---------- 绘制 ----------
        terminal.draw(|f| {
            let area = f.size();
            list_area = draw_scanner(f, area, &games, selected, &root_buf, scanning);
            if creating {
                draw_new_game_dialog(f, area, &new_name, create_error.as_deref());
            }
        })?;

        // ---------- 事件 ----------
        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) => {
                    // ---------- 新建输入模式 ----------
                    if creating {
                        match key.code {
                            KeyCode::Enter => {
                                let name = new_name.trim().to_string();
                                if name.is_empty() {
                                    create_error = Some("名称不能为空".to_string());
                                } else {
                                    let target = root_buf.join(&name);
                                    if target.exists() {
                                        create_error =
                                            Some(format!("{} 已存在", name));
                                    } else {
                                        // 退出 TUI 执行创建
                                        disable_raw_mode()?;
                                        execute!(
                                            terminal.backend_mut(),
                                            LeaveAlternateScreen,
                                            DisableMouseCapture
                                        )?;
                                        terminal.show_cursor()?;

                                        println!(
                                            "正在创建游戏: {}\n",
                                            target.display()
                                        );
                                        let create_result =
                                            crate::commands::new_project(Some(
                                                target.clone(),
                                            ));
                                        if let Err(e) = &create_result {
                                            println!("\n❌ 创建失败: {}", e);
                                        }

                                        println!("\n按回车返回扫描界面...");
                                        let _ =
                                            std::io::stdin().read_line(&mut String::new());

                                        enable_raw_mode()?;
                                        execute!(
                                            terminal.backend_mut(),
                                            EnterAlternateScreen,
                                            EnableMouseCapture
                                        )?;
                                        terminal.clear()?;

                                        // 重置扫描
                                        games.clear();
                                        selected = 0;
                                        scanning = true;
                                        rx = spawn_scan(root_buf.clone());

                                        creating = false;
                                        new_name.clear();
                                        create_error = None;
                                        continue;
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                creating = false;
                                new_name.clear();
                                create_error = None;
                            }
                            KeyCode::Backspace => {
                                new_name.pop();
                                create_error = None;
                            }
                            KeyCode::Char(c) => {
                                new_name.push(c);
                                create_error = None;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // ---------- 普通模式 ----------
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected > 0 {
                                selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !games.is_empty() && selected < games.len() - 1 {
                                selected += 1;
                            }
                        }
                        KeyCode::Home => selected = 0,
                        KeyCode::End => {
                            if !games.is_empty() {
                                selected = games.len() - 1;
                            }
                        }
                        KeyCode::PageUp => {
                            selected = selected.saturating_sub(5);
                        }
                        KeyCode::PageDown => {
                            if !games.is_empty() {
                                selected = (selected + 5).min(games.len() - 1);
                            }
                        }
                        KeyCode::Enter => {
                            if !games.is_empty() {
                                chosen = Some(games[selected].clone());
                                break;
                            }
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            // 只有文件夹游戏支持编辑
                            if let Some(entry) = games.get(selected) {
                                if entry.kind == GameKind::Directory {
                                    // 退出 TUI 进入编辑器
                                    disable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        LeaveAlternateScreen,
                                        DisableMouseCapture
                                    )?;
                                    terminal.show_cursor()?;

                                    // 找 dialogue.ng
                                    let dialog_path =
                                        entry.path.join("assets/dialog/dialogue.ng");
                                    let target = if dialog_path.exists() {
                                        dialog_path
                                    } else {
                                        // 回退：找 assets/dialog 下第一个 .ng/.txt
                                        find_first_script(&entry.path)
                                            .unwrap_or(dialog_path)
                                    };

                                    println!(
                                        "正在打开编辑器: {}\n",
                                        target.display()
                                    );

                                    let edit_result = (|| -> Result<()> {
                                        let mut ed =
                                            crate::edit::Editor::new(target)?;
                                        ed.run()
                                    })();

                                    if let Err(e) = edit_result {
                                        println!("\n❌ 编辑器错误: {}", e);
                                        println!("\n按回车返回扫描界面...");
                                        let _ =
                                            std::io::stdin().read_line(&mut String::new());
                                    }

                                    enable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        EnterAlternateScreen,
                                        EnableMouseCapture
                                    )?;
                                    terminal.clear()?;
                                    continue;
                                } else {
                                    // 打包游戏不支持编辑
                                }
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            creating = true;
                            new_name.clear();
                            create_error = None;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if !games.is_empty() && selected < games.len() - 1 {
                            selected += 1;
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let col = mouse.column;
                        let row = mouse.row;
                        if col >= list_area.x
                            && col < list_area.x + list_area.width
                            && row >= list_area.y + 1
                            && row < list_area.y + list_area.height.saturating_sub(1)
                        {
                            let idx = (row - list_area.y - 1) as usize;
                            if idx < games.len() {
                                let now = Instant::now();
                                let is_double = last_click
                                    .map(|(t, i)| {
                                        i == idx
                                            && now.duration_since(t)
                                                < Duration::from_millis(500)
                                    })
                                    .unwrap_or(false);
                                if is_double {
                                    chosen = Some(games[idx].clone());
                                    break;
                                }
                                selected = idx;
                                last_click = Some((now, idx));
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
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

    if let Some(entry) = chosen {
        return launch_game(&entry);
    }

    Ok(())
}

/// 找游戏目录下第一个剧本文件
fn find_first_script(game_dir: &Path) -> Option<PathBuf> {
    let dialog_dir = game_dir.join("assets/dialog");
    let entries = std::fs::read_dir(&dialog_dir).ok()?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "ng" || ext == "txt" {
                    candidates.push(path);
                }
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next()
}

/// 启动选中的游戏
fn launch_game(entry: &GameEntry) -> Result<()> {
    match entry.kind {
        GameKind::Directory => {
            std::env::set_current_dir(&entry.path)?;
            crate::assets::set_filesystem();
            let result = crate::runner::run_game();
            crate::assets::cleanup_cache();
            result
        }
        GameKind::Packed => crate::commands::run_packed(&entry.path, false),
    }
}

// ============================================================
// 渲染
// ============================================================

const ASCII_TITLE: &[&str] = &[
    r"  ███╗   ██╗ ██████╗  █████╗ ██╗     ",
    r"  ████╗  ██║██╔════╝ ██╔══██╗██║     ",
    r"  ██╔██╗ ██║██║  ███╗███████║██║     ",
    r"  ██║╚██╗██║██║   ██║██╔══██║██║     ",
    r"  ██║ ╚████║╚██████╔╝██║  ██║███████╗",
    r"  ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝",
];

fn draw_scanner(
    frame: &mut ratatui::Frame,
    area: Rect,
    games: &[GameEntry],
    selected: usize,
    root: &Path,
    scanning: bool,
) -> Rect {
    let bg = Color::Rgb(24, 24, 34);
    frame.render_widget(Block::default().style(Style::default().bg(bg)), area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .style(Style::default().bg(bg));
    frame.render_widget(outer, area);

    if area.width >= 2 && area.height >= 2 {
        let corners: [(u16, u16, char); 4] = [
            (area.x, area.y, '✦'),
            (area.x + area.width - 1, area.y, '✦'),
            (area.x, area.y + area.height - 1, '✦'),
            (area.x + area.width - 1, area.y + area.height - 1, '✦'),
        ];
        let buf = frame.buffer_mut();
        for (x, y, ch) in corners {
            let cell = buf.get_mut(x, y);
            cell.set_char(ch)
                .set_fg(Color::Rgb(255, 215, 0))
                .set_style(Style::default().add_modifier(Modifier::BOLD));
        }
    }

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(4),
    };

    let title_h = 6u16.min(inner.height / 3);

    // 布局：标题 / 副标题 / 状态 / 空行 / 列表 / 路径 / 底部
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_h),   // [0] 标题
            Constraint::Length(1),          // [1] 副标题
            Constraint::Length(1),          // [2] 状态
            Constraint::Length(1),          // [3] 空行
            Constraint::Min(6),             // [4] 列表
            Constraint::Length(1),          // [5] 完整路径
            Constraint::Length(2),          // [6] 底部
        ])
        .split(inner);

    // ---------- 大标题 ----------
    let title_lines: Vec<Line> = ASCII_TITLE
        .iter()
        .map(|s| {
            Line::from(Span::styled(
                *s,
                Style::default()
                    .fg(Color::Rgb(230, 130, 220))
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let title_para = Paragraph::new(title_lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(bg));
    frame.render_widget(title_para, chunks[0]);

    // ---------- 副标题 ----------
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("❖ ", Style::default().fg(Color::Rgb(140, 90, 180))),
        Span::styled(
            "终端视觉小说引擎",
            Style::default().fg(Color::Rgb(200, 180, 230)),
        ),
        Span::styled(" ❖", Style::default().fg(Color::Rgb(140, 90, 180))),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().bg(bg));
    frame.render_widget(subtitle, chunks[1]);

    // ---------- 状态行 ----------
    let status_text = if scanning {
        Line::from(vec![
            Span::styled("⟳ ", Style::default().fg(Color::Rgb(255, 215, 0))),
            Span::styled(
                format!("正在扫描… 已找到 {} 个", games.len()),
                Style::default()
                    .fg(Color::Rgb(255, 215, 0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   ({})", root.display()),
                Style::default().fg(Color::Rgb(120, 120, 140)),
            ),
        ])
    } else if games.is_empty() {
        Line::from(vec![Span::styled(
            "⚠ 未在此目录下找到游戏   (按 N 新建)",
            Style::default()
                .fg(Color::Rgb(255, 180, 100))
                .add_modifier(Modifier::BOLD),
        )])
    } else {
        Line::from(vec![
            Span::styled("◆ 找到 ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled(
                format!("{}", games.len()),
                Style::default()
                    .fg(Color::Rgb(255, 215, 0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" 个游戏   ({})", root.display()),
                Style::default().fg(Color::Rgb(150, 150, 180)),
            ),
        ])
    };
    let status = Paragraph::new(status_text)
        .alignment(Alignment::Center)
        .style(Style::default().bg(bg));
    frame.render_widget(status, chunks[2]);

    // ---------- 列表 ----------
    let list_width = 60u16.min(chunks[4].width);
    let list_x = chunks[4].x + (chunks[4].width.saturating_sub(list_width)) / 2;
    let list_area = Rect {
        x: list_x,
        y: chunks[4].y,
        width: list_width,
        height: chunks[4].height,
    };

    if games.is_empty() {
        let tip_lines = if scanning {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "正在搜索当前目录…",
                    Style::default().fg(Color::Rgb(180, 180, 200)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "找到游戏后会立即显示",
                    Style::default().fg(Color::Rgb(120, 120, 140)),
                )),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "当前目录下没有找到游戏",
                    Style::default().fg(Color::Rgb(180, 180, 200)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "按 N 键新建一个游戏",
                    Style::default()
                        .fg(Color::Rgb(120, 255, 150))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "游戏文件夹需要包含 assets/game.json",
                    Style::default().fg(Color::Rgb(120, 120, 140)),
                )),
            ]
        };
        let tip = Paragraph::new(tip_lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg));
        frame.render_widget(tip, list_area);
    } else {
        let items: Vec<ListItem> = games
            .iter()
            .map(|g| {
                let (icon, icon_color) = match g.kind {
                    GameKind::Directory => ("📁", Color::Rgb(150, 200, 255)),
                    GameKind::Packed => ("📦", Color::Rgb(255, 200, 130)),
                };
                let editable_hint = if g.kind == GameKind::Directory {
                    Span::styled("  [E 编辑]", Style::default().fg(Color::Rgb(180, 140, 255)))
                } else {
                    Span::raw("")
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", icon), Style::default().fg(icon_color)),
                    Span::styled(
                        format!("{:<20}", truncate(&g.name, 20)),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("  {}", g.rel_display),
                        Style::default().fg(Color::Rgb(120, 120, 140)),
                    ),
                    editable_hint,
                ]))
            })
            .collect();

        let title = if scanning {
            format!(" ◆ 选择游戏 (扫描中… {}) ", games.len())
        } else {
            " ◆ 选择游戏 (Enter 启动 | E 编辑) ".to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Rgb(255, 130, 200))
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::Rgb(160, 110, 200)))
                    .style(Style::default().bg(Color::Rgb(30, 30, 42))),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(255, 210, 100))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = ListState::default();
        state.select(Some(selected));
        frame.render_stateful_widget(list, list_area, &mut state);
    }

    // ---------- 完整路径 ----------
    if let Some(entry) = games.get(selected) {
        let full_path = entry.path.display().to_string();
        // 路径过长时截断中间，保留首尾
        let available = chunks[5].width.saturating_sub(4) as usize;
        let display_path = truncate_path(&full_path, available);

        let path_line = Line::from(vec![
            Span::styled(
                " ● ",
                Style::default().fg(Color::Rgb(255, 130, 200)),
            ),
            Span::styled(
                display_path,
                Style::default()
                    .fg(Color::Rgb(200, 200, 220))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let path_para = Paragraph::new(path_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg));
        frame.render_widget(path_para, chunks[5]);
    }

    // ---------- 底部提示 + 版本号 ----------
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[6]);

    let version_text = Line::from(vec![
        Span::styled(
            format!(" ngal v{} ", env!("CARGO_PKG_VERSION")),
            Style::default()
                .fg(Color::Rgb(24, 24, 34))
                .bg(Color::Rgb(180, 140, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);
    let version_para = Paragraph::new(version_text)
        .alignment(Alignment::Left)
        .style(Style::default().bg(bg));
    frame.render_widget(version_para, bottom_chunks[0]);

    let hint = if !games.is_empty() {
        Line::from(vec![
            Span::styled("↑↓ ", Style::default().fg(Color::Rgb(255, 215, 0))),
            Span::styled("选择  ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled("Enter ", Style::default().fg(Color::Rgb(120, 255, 150))),
            Span::styled("启动  ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled("E ", Style::default().fg(Color::Rgb(180, 140, 255))),
            Span::styled("编辑  ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled("N ", Style::default().fg(Color::Rgb(120, 255, 150))),
            Span::styled("新建  ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled("Q ", Style::default().fg(Color::Rgb(255, 120, 120))),
            Span::styled("退出", Style::default().fg(Color::Rgb(150, 150, 180))),
        ])
    } else {
        Line::from(vec![
            Span::styled("N ", Style::default().fg(Color::Rgb(120, 255, 150))),
            Span::styled("新建  ", Style::default().fg(Color::Rgb(150, 150, 180))),
            Span::styled("Q / Esc ", Style::default().fg(Color::Rgb(255, 120, 120))),
            Span::styled("退出", Style::default().fg(Color::Rgb(150, 150, 180))),
        ])
    };
    let hint_para = Paragraph::new(hint)
        .alignment(Alignment::Right)
        .style(Style::default().bg(bg));
    frame.render_widget(hint_para, bottom_chunks[1]);

    list_area
}

/// 路径过长时中间截断，保留首尾
fn truncate_path(path: &str, max_chars: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_chars {
        return path.to_string();
    }
    if max_chars < 8 {
        return chars[..max_chars].iter().collect();
    }
    let keep_head = max_chars / 2 - 1;
    let keep_tail = max_chars - keep_head - 3;
    let head: String = chars[..keep_head].iter().collect();
    let tail: String = chars[chars.len() - keep_tail..].iter().collect();
    format!("{}...{}", head, tail)
}

/// 新建游戏输入弹窗
fn draw_new_game_dialog(
    frame: &mut ratatui::Frame,
    area: Rect,
    new_name: &str,
    error: Option<&str>,
) {
    let popup_w = 50u16.min(area.width);
    let popup_h = 9u16.min(area.height);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };

    frame.render_widget(Clear, popup);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "输入新游戏的目录名：",
            Style::default().fg(Color::Rgb(200, 200, 220)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(Color::Rgb(255, 130, 200))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                new_name.to_string(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█",
                Style::default()
                    .fg(Color::Rgb(120, 255, 150))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(e) = error {
        lines.push(Line::from(Span::styled(
            format!("⚠ {}", e),
            Style::default().fg(Color::Rgb(255, 120, 120)),
        )));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(120, 255, 150)),
        ),
        Span::styled(" 确认   ", Style::default().fg(Color::Rgb(180, 180, 200))),
        Span::styled(
            " Esc ",
            Style::default().fg(Color::Black).bg(Color::Rgb(255, 120, 120)),
        ),
        Span::styled(" 取消", Style::default().fg(Color::Rgb(180, 180, 200))),
    ]));

    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(Span::styled(
                    " ◆ 新建游戏 ",
                    Style::default()
                        .fg(Color::Rgb(255, 215, 0))
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::Rgb(255, 215, 0)))
                .style(Style::default().bg(Color::Rgb(30, 30, 42))),
        );

    frame.render_widget(para, popup);
}

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        out.push('…');
        out
    }
}