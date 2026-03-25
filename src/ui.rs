use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::image;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

    // 全局背景
    let bg_color = Color::Rgb(30, 20, 40);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(bg_color)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(8)),
            Constraint::Length(8),
        ])
        .split(area);

    render_top(frame, chunks[0], app);
    render_bottom(frame, chunks[1], app);
}

fn render_top(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double)
        .style(Style::default().bg(Color::Rgb(40, 30, 50)));
    frame.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width - 2,
        height: area.height - 2,
    };

    match &app.state {
        crate::app::AppState::Menu => {
            let mut y_offset = 0;
            if let Some(logo) = &app.logo {
                let logo_area = Rect {
                    x: inner.x,
                    y: inner.y,
                    width: inner.width,
                    height: 6.min(inner.height),
                };
                image::draw_portrait(frame, logo_area, logo);
                y_offset = 6;
            }

            let title = &app.db.title;
            let title_para = Paragraph::new(vec![
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
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::Rgb(40, 30, 50)));
            frame.render_widget(title_para, Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: 4,
            });

            let items: Vec<ListItem> = app
                .menu_options
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    let style = if i == app.selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                            .bg(Color::Rgb(60, 50, 70))
                    } else {
                        Style::default()
                            .fg(Color::Rgb(200, 200, 200))
                            .bg(Color::Rgb(40, 30, 50))
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = app.menu_options.len() as u16 * 2;
            let start_y = inner.y + y_offset + 4 + (inner.height.saturating_sub(y_offset + 4 + list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(30)) / 2,
                y: start_y,
                width: 30.min(inner.width),
                height: list_height.min(inner.height - y_offset - 4),
            };
            frame.render_widget(list, list_area);

            // 版本号
            let version = format!("v{}", app.config.version);
            let version_para = Paragraph::new(version)
                .style(Style::default().fg(Color::Rgb(150, 150, 150)).bg(Color::Rgb(40, 30, 50)))
                .alignment(Alignment::Right);
            let version_area = Rect {
                x: inner.x + inner.width - 15,
                y: inner.y + inner.height - 1,
                width: 15,
                height: 1,
            };
            frame.render_widget(version_para, version_area);
        }
        crate::app::AppState::Settings => {
            let text = vec![
                Line::from(vec![Span::styled("⚙️ 音量设置", Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD))]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("BGM 音量: "),
                    Span::styled(format!("{}%", app.config.bgm_volume), Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  (+/- 调节)"),
                ]),
                Line::from(vec![
                    Span::raw("语音音量: "),
                    Span::styled(format!("{}%", app.config.voice_volume), Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  ([ ] 调节)"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("自动播放: "),
                    Span::styled(if app.config.auto_play { "开启" } else { "关闭" }, Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  (A 切换)"),
                ]),
                Line::from(vec![
                    Span::raw("自动播放速度: "),
                    Span::styled(format!("{:.1}秒", app.config.auto_play_speed), Style::default().fg(Color::Rgb(100, 255, 100))),
                    Span::raw("  (1 减慢 / 2 加快)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled("按 S 保存配置 | ESC 返回", Style::default().fg(Color::Rgb(150, 150, 150)))]),
            ];
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(40, 30, 50)))
                .alignment(Alignment::Center);
            let area = Rect {
                x: inner.x,
                y: inner.y + (inner.height.saturating_sub(8)) / 2,
                width: inner.width,
                height: 8,
            };
            frame.render_widget(para, area);

            if let Some(msg) = &app.status_message {
                let msg_para = Paragraph::new(msg.as_str())
                    .style(Style::default().fg(Color::Rgb(255, 255, 0)).bg(Color::Rgb(40, 30, 50)))
                    .alignment(Alignment::Center);
                let msg_area = Rect {
                    x: inner.x,
                    y: inner.y + inner.height - 3,
                    width: inner.width,
                    height: 1,
                };
                frame.render_widget(msg_para, msg_area);
            }
        }
        crate::app::AppState::About => {
            let text = vec![
                Line::from(vec![Span::styled("🎮 ngal - 终端视觉小说引擎", Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD))]),
                Line::from(""),
                Line::from("作者: 🤓NAS油条🤓"),
                Line::from(format!("版本: v{}", app.config.version)),
                Line::from(""),
                Line::from("项目地址:"),
                Line::from("https://github.com/nasyt233/ngal"),
                Line::from(""),
                Line::from("项目依赖:"),
                Line::from("  - Ratatui"),
                Line::from("  - Crossterm"),
                Line::from("  - image-rs"),
                Line::from("  - mpv(需自装)"),
                Line::from(""),
                Line::from("按 ESC 返回"),
            ];
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(40, 30, 50)))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("关于我们").border_style(Style::default().fg(Color::Rgb(212, 112, 212))));
            let area = Rect {
                x: inner.x + (inner.width.saturating_sub(50)) / 2,
                y: inner.y + (inner.height.saturating_sub(15)) / 2,
                width: 50.min(inner.width),
                height: 15.min(inner.height),
            };
            frame.render_widget(para, area);
        }
        crate::app::AppState::History => {
            let items: Vec<ListItem> = app.history.iter().map(|(speaker, text)| {
                let prefix = if let Some(s) = speaker {
                    format!("[{}] ", s)
                } else {
                    "".to_string()
                };
                let display_text = format!("{}{}", prefix, text);
                ListItem::new(display_text)
            }).collect();
            
            let mut list_state = ratatui::widgets::ListState::default();
            // 默认选中最后一项，实现底部显示
            if let Some(last) = items.len().checked_sub(1) {
                list_state.select(Some(last));
            }
            
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("历史记录").border_style(Style::default().fg(Color::Rgb(212, 112, 212))))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(20, 20, 30)));
            
            let history_area = Rect {
                x: inner.x + (inner.width.saturating_sub(60)) / 2,
                y: inner.y + (inner.height.saturating_sub(20)) / 2,
                width: 60.min(inner.width),
                height: 20.min(inner.height),
            };
            
            frame.render_widget(Clear, history_area);
            frame.render_stateful_widget(list, history_area, &mut list_state);
        }
        crate::app::AppState::InDialogue { .. } => {
            if let Some(line) = app.current_dialogue_line() {
                if let (Some(speaker), Some(_text)) = (&line.speaker, &line.text) {
                    if let Some(img) = app.portraits.get(speaker) {
                        image::draw_portrait(frame, inner, img);
                    } else {
                        let art = match speaker.as_str() {
                            "NAS油条" => "   🍳  NAS油条  🍳",
                            "鸣朝"    => "   ⚔️  鸣朝  ⚔️",
                            "原神"    => "   ✨  原神  ✨",
                            _ => "   （暂无立绘）",
                        };
                        let text = format!("{}\n\n{}", art, speaker);
                        let para = Paragraph::new(text)
                            .style(Style::default().fg(Color::Rgb(212, 112, 212)).bg(Color::Rgb(40, 30, 50)))
                            .alignment(Alignment::Center)
                            .wrap(Wrap { trim: true });
                        let para_area = Rect {
                            x: inner.x,
                            y: inner.y + (inner.height.saturating_sub(5)) / 2,
                            width: inner.width,
                            height: 5.min(inner.height),
                        };
                        frame.render_widget(para, para_area);
                    }
                }
            }
        }
        crate::app::AppState::InChoice { options, selected, .. } => {
            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, (text, _))| {
                    let style = if i == *selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                            .bg(Color::Rgb(60, 50, 70))
                    } else {
                        Style::default()
                            .fg(Color::Rgb(200, 200, 200))
                            .bg(Color::Rgb(40, 30, 50))
                    };
                    ListItem::new(Line::from(Span::styled(text, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().title("请选择：").borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = options.len() as u16 * 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: inner.y + (inner.height.saturating_sub(list_height)) / 2,
                width: 40.min(inner.width),
                height: list_height.min(inner.height),
            };
            frame.render_widget(list, list_area);
        }
    }
}

fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    let bg_color = Color::Rgb(40, 30, 50);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let name_area = chunks[0];
    let text_area = chunks[1];

    let (speaker, content, status) = match &app.state {
        crate::app::AppState::Menu => (
            "系统".to_string(),
            format!("{} | q 退出", app.db.footer),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::Settings => (
            "设置".to_string(),
            "按 +/- 调节BGM音量，[ ] 调节语音音量，A 切换自动播放，1/2 调节速度，S 保存，ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::About => (
            "关于".to_string(),
            "按 ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::History => (
            "历史记录".to_string(),
            "按 ESC 或 H 关闭".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::InDialogue { .. } => (
            app.current_speaker().unwrap_or_else(|| "".to_string()),
            app.current_text().unwrap_or_else(|| "".to_string()),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::InChoice { .. } => (
            app.current_speaker().unwrap_or_else(|| "系统".to_string()),
            "请选择一项：".to_string(),
            app.status_message.as_deref(),
        ),
    };

    // 名字显示
    let name_style = Style::default()
        .fg(Color::Rgb(255, 255, 255))
        .add_modifier(Modifier::BOLD)
        .bg(bg_color);
    if !speaker.is_empty() {
        let name_para = Paragraph::new(Line::from(Span::styled(speaker, name_style)))
            .alignment(Alignment::Left);
        frame.render_widget(name_para, name_area);
    } else {
        frame.render_widget(Paragraph::new(""), name_area);
    }

    // 文本框内容
    let display_text = if let Some(msg) = status { msg } else { content.as_str() };
    let text_style = if status.is_some() {
        Style::default().fg(Color::Rgb(255, 255, 0))
    } else {
        Style::default().fg(Color::Rgb(255, 255, 255))
    };

    let text_para = Paragraph::new(display_text)
        .style(text_style.bg(bg_color))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                .border_type(ratatui::widgets::BorderType::Double)
                .style(Style::default().bg(bg_color)),
        );
    frame.render_widget(text_para, text_area);
}