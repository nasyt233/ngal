use std::path::Path;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::image;
use crate::save::SaveData;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

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
                image::draw_portrait(frame, logo_area, logo, 2, 100);  // 居中，100% 大小
                y_offset = 6;
            }

            let title = &app.title;
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
            let available_height = inner.height.saturating_sub(y_offset + 4);
            let start_y = inner.y + y_offset + 4 + (available_height.saturating_sub(list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(30)) / 2,
                y: start_y,
                width: 30.min(inner.width),
                height: list_height.min(available_height),
            };
            frame.render_widget(list, list_area);

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
                Line::from(vec![
                    Span::styled("⚙️ 音量设置", Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD))
                ]),
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
                Line::from(vec![
                    Span::styled("按 S 保存配置 | ESC 返回", Style::default().fg(Color::Rgb(150, 150, 150)))
                ]),
            ];
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(40, 30, 50)))
                .alignment(Alignment::Center);
            let para_area = Rect {
                x: inner.x,
                y: inner.y + (inner.height.saturating_sub(8)) / 2,
                width: inner.width,
                height: 8,
            };
            frame.render_widget(para, para_area);

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
                Line::from(vec![
                    Span::styled("🎮 ngal - 终端视觉小说引擎", Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD))
                ]),
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
            let para_area = Rect {
                x: inner.x + (inner.width.saturating_sub(50)) / 2,
                y: inner.y + (inner.height.saturating_sub(15)) / 2,
                width: 50.min(inner.width),
                height: 15.min(inner.height),
            };
            frame.render_widget(para, para_area);
        }
        crate::app::AppState::History => {
            let items: Vec<ListItem> = app.history.iter().rev().map(|(speaker, text)| {
                let prefix = if let Some(s) = speaker {
                    format!("[{}] ", s)
                } else {
                    "".to_string()
                };
                let display_text = format!("{}{}", prefix, text);
                ListItem::new(display_text)
            }).collect();

            let mut list_state = ratatui::widgets::ListState::default();
            if let Some(last) = items.len().checked_sub(1) {
                list_state.select(Some(last));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("历史记录").border_style(Style::default().fg(Color::Rgb(212, 112, 212))))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(20, 20, 30)));

            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(60)) / 2,
                y: inner.y + (inner.height.saturating_sub(20)) / 2,
                width: 60.min(inner.width),
                height: 20.min(inner.height),
            };
            frame.render_widget(Clear, list_area);
            frame.render_stateful_widget(list, list_area, &mut list_state);
        }
        crate::app::AppState::SaveSlot => {
            let items: Vec<ListItem> = (1..=10).map(|i| {
                let exists = SaveData::exists(i);
                let info = if exists {
                    // 读取存档显示时间戳（简单显示）
                    if let Ok(data) = SaveData::load(i) {
                        format!("存档槽 {} - {}", i, data.timestamp)
                    } else {
                        format!("存档槽 {} (有存档)", i)
                    }
                } else {
                    format!("存档槽 {} (空)", i)
                };
                let style = if i - 1 == app.selected {
                    Style::default()
                        .fg(Color::Rgb(255, 255, 0))
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Rgb(60, 50, 70))
                } else if exists {
                    Style::default()
                        .fg(Color::Rgb(200, 200, 200))
                        .bg(Color::Rgb(40, 30, 50))
                } else {
                    Style::default()
                        .fg(Color::Rgb(100, 100, 100))
                        .bg(Color::Rgb(40, 30, 50))
                };
                ListItem::new(Line::from(Span::styled(info, style)))
            }).collect();
        
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("选择存档槽位").border_style(Style::default().fg(Color::Rgb(212, 112, 212))))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                .highlight_symbol("> ");
        
            let list_height = 10;
            let start_y = inner.y + (inner.height.saturating_sub(list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: start_y,
                width: 40.min(inner.width),
                height: list_height.min(inner.height),
            };
            frame.render_widget(list, list_area);
        }
        
        crate::app::AppState::LoadSlot => {
            let items: Vec<ListItem> = (1..=10).map(|i| {
                let exists = SaveData::exists(i);
                let info = if exists {
                    if let Ok(data) = SaveData::load(i) {
                        format!("存档槽 {} - {}", i, data.timestamp)
                    } else {
                        format!("存档槽 {} (有存档)", i)
                    }
                } else {
                    format!("存档槽 {} (空)", i)
                };
                let style = if i - 1 == app.selected && exists {
                    Style::default()
                        .fg(Color::Rgb(255, 255, 0))
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Rgb(60, 50, 70))
                } else if exists {
                    Style::default()
                        .fg(Color::Rgb(200, 200, 200))
                        .bg(Color::Rgb(40, 30, 50))
                } else {
                    Style::default()
                        .fg(Color::Rgb(100, 100, 100))
                        .bg(Color::Rgb(40, 30, 50))
                };
                ListItem::new(Line::from(Span::styled(info, style)))
            }).collect();
        
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("选择读档槽位").border_style(Style::default().fg(Color::Rgb(212, 112, 212))))
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                .highlight_symbol("> ");
        
            let list_height = 10;
            let start_y = inner.y + (inner.height.saturating_sub(list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: start_y,
                width: 40.min(inner.width),
                height: list_height.min(inner.height),
            };
            frame.render_widget(list, list_area);
        }
        crate::app::AppState::Input { prompt, .. } => {
            let input_display = format!("{}: {}", prompt, app.input_buffer);
            let para = Paragraph::new(vec![
                Line::from(Span::styled(input_display, Style::default().fg(Color::White))),
                Line::from(Span::styled("(输入文本，按回车确认，ESC取消)", Style::default().fg(Color::Gray))),
            ])
            .style(Style::default().bg(Color::Rgb(40, 30, 50)))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("输入").border_style(Style::default().fg(Color::Rgb(212, 112, 212))));
            let para_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: inner.y + (inner.height.saturating_sub(6)) / 2,
                width: 40.min(inner.width),
                height: 6.min(inner.height),
            };
            frame.render_widget(para, para_area);
        }
        crate::app::AppState::InDialogue { .. } => {
            // 1. 绘制背景图片（如果有）
            if let Some(bg_filename) = &app.current_background {
                let bg_path = Path::new("assets/portraits").join(bg_filename);
                if let Ok(bg_img) = image::load_image(&bg_path) {
                    image::draw_background(frame, inner, &bg_img);
                }
            }
        
            // 2. 绘制前景立绘（如果有）
            if let Some(params) = &app.current_image_params {
                if let Some(filename) = &params.filename {
                    let img = if let Some(cached) = app.image_cache.get(filename) {
                        cached.clone()
                    } else {
                        let img_path = Path::new("assets/portraits").join(filename);
                        match image::load_image(&img_path) {
                            Ok(img) => {
                                app.image_cache.insert(filename.clone(), img.clone());
                                img
                            }
                            Err(_) => {
                                // 返回一个 1x1 透明占位图片
                                let placeholder = ::image::ImageBuffer::from_pixel(1, 1, ::image::Rgba([0, 0, 0, 0]));
                                app.image_cache.insert(filename.clone(), placeholder.clone());
                                placeholder
                            }
                        }
                    };
                    image::draw_portrait(frame, inner, &img, params.position, params.scale);
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
            format!("{} | q 退出", app.footer),
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
        crate::app::AppState::LoadSlot => (
            "读档".to_string(),
            if SaveData::exists(1) { "↑↓ 移动，Enter 确认，ESC 返回".to_string() } else { "暂无存档，ESC 返回".to_string() },
            app.status_message.as_deref(),
        ),
        crate::app::AppState::SaveSlot => (
            "存档".to_string(),
            "↑↓ 移动，Enter 确认，ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::Input { .. } => (
            "输入".to_string(),
            "请输入内容".to_string(),
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