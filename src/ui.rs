// src/ui.rs
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::save::SaveData;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.size();

    let chunks = Layout::vertical([
        Constraint::Length(area.height.saturating_sub(8)),
        Constraint::Length(8),
    ])
    .split(area);

    render_top(frame, chunks[0], app);
    render_bottom(frame, chunks[1], app);
}

fn get_bg_color(app: &App) -> Color {
    match app.config.background_color.as_str() {
        "default" => Color::Reset,
        "#2A2A3E" => Color::Rgb(42, 42, 62),
        "#1E1E2E" => Color::Rgb(30, 30, 46),
        "#222436" => Color::Rgb(34, 36, 54),
        "#2C2C3C" => Color::Rgb(44, 44, 60),
        "#3A2C3C" => Color::Rgb(58, 44, 60),
        _ => {
            if let Some(rgb) = parse_hex_color(&app.config.background_color) {
                Color::Rgb(rgb.0, rgb.1, rgb.2)
            } else {
                Color::Rgb(40, 30, 50)
            }
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn render_top(frame: &mut Frame, area: Rect, app: &mut App) {
    let bg_color = get_bg_color(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .border_type(ratatui::widgets::BorderType::Double)
        .style(Style::default().bg(bg_color));
    frame.render_widget(block, area);

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
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    match &app.state {
        crate::app::AppState::Menu => {
            render_menu(frame, inner, app, bg_color);
        }
        crate::app::AppState::Settings => {
            let text = vec![
                Line::from(vec![Span::styled(
                    "⚙️ 音量设置",
                    Style::default()
                        .fg(Color::Rgb(255, 215, 0))
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("BGM 音量: "),
                    Span::styled(
                        format!("{}%", app.config.bgm_volume),
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (+/- 调节)"),
                ]),
                Line::from(vec![
                    Span::raw("语音音量: "),
                    Span::styled(
                        format!("{}%", app.config.voice_volume),
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  ([ ] 调节)"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("自动播放: "),
                    Span::styled(
                        if app.config.auto_play { "开启" } else { "关闭" },
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (A 切换)"),
                ]),
                Line::from(vec![
                    Span::raw("自动播放速度: "),
                    Span::styled(
                        format!("{:.1}秒", app.config.auto_play_speed),
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (1 减慢 / 2 加快)"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("文字动画: "),
                    Span::styled(
                        if app.config.text_animation { "开启" } else { "关闭" },
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (T 切换)"),
                ]),
                Line::from(vec![
                    Span::raw("文字速度: "),
                    Span::styled(
                        format!("{}ms", app.config.text_speed),
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (3 减慢 / 4 加快)"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("背景颜色: "),
                    Span::styled(
                        match app.config.background_color.as_str() {
                            "default" => "无色",
                            "#2A2A3E" => "深灰紫",
                            "#1E1E2E" => "猫鼬暗色",
                            "#222436" => "深藏青",
                            "#2C2C3C" => "暖灰",
                            "#3A2C3C" => "紫罗兰灰",
                            _ => &app.config.background_color,
                        },
                        Style::default().fg(Color::Rgb(100, 255, 100)),
                    ),
                    Span::raw("  (B 切换)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "按 S 保存配置 | ESC 返回",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                )]),
            ];
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(bg_color))
                .alignment(Alignment::Center);
            let para_area = Rect {
                x: inner.x,
                y: inner.y + (inner.height.saturating_sub(12)) / 2,
                width: inner.width,
                height: 12,
            };
            frame.render_widget(para, para_area);

            if let Some(msg) = &app.status_message {
                let msg_para = Paragraph::new(msg.as_str())
                    .style(Style::default().fg(Color::Rgb(255, 255, 0)).bg(bg_color))
                    .alignment(Alignment::Center);
                let msg_area = Rect {
                    x: inner.x,
                    y: inner.y + inner.height.saturating_sub(3),
                    width: inner.width,
                    height: 1,
                };
                frame.render_widget(msg_para, msg_area);
            }
        }
        crate::app::AppState::About => {
            let text = vec![
                Line::from(vec![Span::styled(
                    "🎮 ngal - 终端视觉小说引擎",
                    Style::default()
                        .fg(Color::Rgb(255, 215, 0))
                        .add_modifier(Modifier::BOLD),
                )]),
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
                Line::from(""),
                Line::from("按 ESC 返回"),
            ];
            let para = Paragraph::new(text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(bg_color))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("关于我们")
                        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                        .style(Style::default().bg(bg_color)),
                );
            let para_area = Rect {
                x: inner.x + (inner.width.saturating_sub(50)) / 2,
                y: inner.y + (inner.height.saturating_sub(15)) / 2,
                width: 50.min(inner.width),
                height: 15.min(inner.height),
            };
            frame.render_widget(para, para_area);
        }
        crate::app::AppState::History => {
            let items: Vec<ListItem> = app
                .history
                .iter()
                .map(|(speaker, text)| {
                    let prefix = if let Some(s) = speaker {
                        format!("[{}] ", s)
                    } else {
                        "".to_string()
                    };
                    let display_text = format!("{}{}", prefix, text);
                    ListItem::new(display_text)
                })
                .collect();

            let mut list_state = ratatui::widgets::ListState::default();
            if !items.is_empty() {
                let sel = app.history_selected.min(items.len() - 1);
                list_state.select(Some(sel));
            }

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("历史记录 (↑↓/PgUp/PgDn 滚动 | ESC 返回)")
                        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                        .style(Style::default().bg(bg_color)),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Rgb(255, 255, 0))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ")
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).bg(bg_color));

            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(60)) / 2,
                y: inner.y + (inner.height.saturating_sub(20)) / 2,
                width: 60.min(inner.width),
                height: 20.min(inner.height),
            };
            frame.render_stateful_widget(list, list_area, &mut list_state);
        }
        crate::app::AppState::GameMenu => {
            let items = vec!["1. 返回游戏", "2. 存档", "3. 读档", "q. 返回主界面"];
            let list_items: Vec<ListItem> = items
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
                    ListItem::new(Line::from(Span::styled(*text, style.bg(bg_color))))
                })
                .collect();

            let list = List::new(list_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("游戏菜单")
                        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                        .style(Style::default().bg(bg_color)),
                )
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)));

            let list_height = items.len() as u16 * 2;
            let start_y = inner.y + (inner.height.saturating_sub(list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(30)) / 2,
                y: start_y,
                width: 30.min(inner.width),
                height: list_height.min(inner.height),
            };
            frame.render_widget(list, list_area);
        }
        crate::app::AppState::SaveSlot => {
            let total = SaveData::next_empty_slot();
            let items: Vec<ListItem> = (1..=total)
                .map(|i| {
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
                    let style = if i - 1 == app.selected {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 0))
                            .add_modifier(Modifier::BOLD)
                    } else if exists {
                        Style::default().fg(Color::Rgb(200, 200, 200))
                    } else {
                        Style::default().fg(Color::Rgb(100, 100, 100))
                    };
                    ListItem::new(Line::from(Span::styled(info, style.bg(bg_color))))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("选择存档槽位 (↑↓ 滚动)")
                        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                        .style(Style::default().bg(bg_color)),
                )
                .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                .highlight_symbol("> ");

            let mut list_state = ratatui::widgets::ListState::default();
            list_state.select(Some(app.selected));

            let list_height = 10.min(inner.height);
            let start_y = inner.y + (inner.height.saturating_sub(list_height)) / 2;
            let list_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: start_y,
                width: 40.min(inner.width),
                height: list_height,
            };
            frame.render_stateful_widget(list, list_area, &mut list_state);
        }
        crate::app::AppState::LoadSlot => {
            let valid_slots: Vec<usize> = SaveData::list_slots();

            if valid_slots.is_empty() {
                let para = Paragraph::new("暂无存档，请先进行游戏并保存\n\n按 ESC 返回")
                    .style(Style::default().fg(Color::Rgb(255, 200, 100)).bg(bg_color))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("选择读档槽位")
                            .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                            .style(Style::default().bg(bg_color)),
                    );
                let para_area = Rect {
                    x: inner.x + (inner.width.saturating_sub(40)) / 2,
                    y: inner.y + (inner.height.saturating_sub(6)) / 2,
                    width: 40.min(inner.width),
                    height: 6.min(inner.height),
                };
                frame.render_widget(para, para_area);
            } else {
                let items: Vec<ListItem> = valid_slots
                    .iter()
                    .map(|&i| {
                        let info = if let Ok(data) = SaveData::load(i) {
                            format!("存档槽 {} - {}", i, data.timestamp)
                        } else {
                            format!("存档槽 {}", i)
                        };
                        let style =
                            if valid_slots.iter().position(|&x| x == i) == Some(app.selected) {
                                Style::default()
                                    .fg(Color::Rgb(255, 255, 0))
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Rgb(200, 200, 200))
                            };
                        ListItem::new(Line::from(Span::styled(info, style.bg(bg_color))))
                    })
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("选择读档槽位 (↑↓ 滚动)")
                            .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                            .style(Style::default().bg(bg_color)),
                    )
                    .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
                    .highlight_symbol("> ");

                let mut list_state = ratatui::widgets::ListState::default();
                list_state.select(Some(app.selected));

                let list_height = 10.min(inner.height);
                let start_y = inner.y + (inner.height.saturating_sub(list_height)) / 2;
                let list_area = Rect {
                    x: inner.x + (inner.width.saturating_sub(40)) / 2,
                    y: start_y,
                    width: 40.min(inner.width),
                    height: list_height,
                };
                frame.render_stateful_widget(list, list_area, &mut list_state);
            }
        }
        crate::app::AppState::Input { ref prompt, .. } => {
            let input_display = format!("{}: {}", prompt, app.input_buffer);
            let para = Paragraph::new(vec![
                Line::from(Span::styled(input_display, Style::default().fg(Color::White))),
                Line::from(Span::styled(
                    "(按回车确认，ESC取消)",
                    Style::default().fg(Color::Gray),
                )),
            ])
            .style(Style::default().bg(bg_color))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("输入")
                    .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                    .style(Style::default().bg(bg_color)),
            );
            let para_area = Rect {
                x: inner.x + (inner.width.saturating_sub(40)) / 2,
                y: inner.y + (inner.height.saturating_sub(6)) / 2,
                width: 40.min(inner.width),
                height: 6.min(inner.height),
            };
            frame.render_widget(para, para_area);
        }
        crate::app::AppState::InDialogue { .. } => {
            if let Some(bg_filename) = &app.current_background {
                let bg_path = format!("assets/portraits/{}", bg_filename);
                if let Ok(bg_img) = crate::image::load_image_rgba(&bg_path) {
                    crate::image::draw_background(frame, inner, &bg_img);
                }
            }

            if let Some(params) = &app.current_image_params {
                if let Some(filename) = &params.filename {
                    let path = format!("assets/portraits/{}", filename);
                    match crate::image::load_image_rgba(&path) {
                        Ok(img) => {
                            crate::image::draw_portrait(
                                frame,
                                inner,
                                &img,
                                params.position,
                                params.scale,
                            );
                        }
                        Err(_) => {
                            let text = format!("图片加载失败: {}", filename);
                            let para = Paragraph::new(text)
                                .style(
                                    Style::default()
                                        .fg(Color::Rgb(212, 112, 212))
                                        .bg(bg_color),
                                )
                                .alignment(Alignment::Center);
                            frame.render_widget(para, inner);
                        }
                    }
                }
            }
        }
        crate::app::AppState::InChoice {
            options, selected, ..
        } => {
            if let Some(bg_filename) = &app.current_background {
                let bg_path = format!("assets/portraits/{}", bg_filename);
                if let Ok(bg_img) = crate::image::load_image_rgba(&bg_path) {
                    crate::image::draw_background(frame, inner, &bg_img);
                }
            }

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
                    ListItem::new(Line::from(Span::styled(text, style.bg(bg_color))))
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
        crate::app::AppState::EndOfFile => {
            let para = Paragraph::new(vec![
                Line::from(Span::styled(
                    "剧情结束",
                    Style::default()
                        .fg(Color::Rgb(255, 215, 0))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "按任意键返回主菜单",
                    Style::default().fg(Color::Rgb(200, 200, 200)),
                )),
            ])
            .style(Style::default().bg(bg_color))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("结束")
                    .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
                    .style(Style::default().bg(bg_color)),
            );
            let para_area = Rect {
                x: inner.x + (inner.width.saturating_sub(30)) / 2,
                y: inner.y + (inner.height.saturating_sub(5)) / 2,
                width: 30.min(inner.width),
                height: 5.min(inner.height),
            };
            frame.render_widget(para, para_area);
        }
    }
}

// ============================================================
// 菜单渲染：分发到 5 种布局
// ============================================================

fn render_menu(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    match app.menu_layout {
        2 => render_menu_layout2(frame, inner, app, bg_color),
        3 => render_menu_layout3(frame, inner, app, bg_color),
        4 => render_menu_layout4(frame, inner, app, bg_color),
        5 => render_menu_layout5(frame, inner, app, bg_color),
        _ => render_menu_layout1(frame, inner, app, bg_color),
    }

    // 版本号统一右下角
    let version = format!("v{}", app.config.version);
    let version_para = Paragraph::new(version)
        .style(Style::default().fg(Color::Rgb(150, 150, 150)).bg(bg_color))
        .alignment(Alignment::Right);
    let version_area = Rect {
        x: inner.x + inner.width.saturating_sub(15),
        y: inner.y + inner.height.saturating_sub(1),
        width: 15,
        height: 1,
    };
    frame.render_widget(version_para, version_area);
}

/// 构建菜单列表（供所有布局复用）
fn build_menu_list<'a>(app: &App, bg_color: Color, _width: u16) -> List<'a> {
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
            ListItem::new(Line::from(Span::styled(text.clone(), style.bg(bg_color))))
        })
        .collect();

    List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().fg(Color::Rgb(255, 255, 0)))
        .style(Style::default().bg(bg_color))
}

/// 构建标题段落
fn build_title_lines(app: &App) -> Paragraph<'static> {
    let title = app.title.clone();
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled("✨", Style::default().fg(Color::Rgb(255, 255, 0))),
            Span::raw(" "),
            Span::styled(
                title,
                Style::default()
                    .fg(Color::Rgb(212, 112, 212))
                    .add_modifier(Modifier::BOLD),
            ),
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
}

// ==================== 布局 1：默认（logo 上方 + 左菜单卡片右图片） ====================
fn render_menu_layout1(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    let mut y_offset = 0u16;

    // ---------- 顶部 Logo ----------
    if let Some(img) = &app.logo {
        let logo_height = 6u16.min(inner.height / 4);
        let logo_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: logo_height,
        };
        crate::image::draw_portrait(frame, logo_area, img, 2, 100);
        y_offset += logo_height + 1;
    }

    // ---------- 标题 ----------
    let title_height = 3u16;
    frame.render_widget(
        build_title_lines(app).style(Style::default().bg(bg_color)),
        Rect {
            x: inner.x,
            y: inner.y + y_offset,
            width: inner.width,
            height: title_height,
        },
    );
    y_offset += title_height;

    // ---------- 装饰分隔线（短）----------
    let divider_text = "❖ ───────────── ❖";
    let divider_w = (divider_text.chars().count() as u16).min(inner.width);
    let divider = Paragraph::new(divider_text)
        .style(
            Style::default()
                .fg(Color::Rgb(140, 90, 180))
                .bg(bg_color),
        )
        .alignment(Alignment::Center);
    frame.render_widget(
        divider,
        Rect {
            x: inner.x + (inner.width.saturating_sub(divider_w)) / 2,
            y: inner.y + y_offset,
            width: divider_w,
            height: 1,
        },
    );
    y_offset += 2;

    // ---------- 菜单 + 图片 ----------
    let remaining_height = inner.height.saturating_sub(y_offset);

    let (menu_area, image_area) = if app.menu_image.is_some() {
        let split = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(Rect {
            x: inner.x,
            y: inner.y + y_offset,
            width: inner.width,
            height: remaining_height,
        });
        (split[0], Some(split[1]))
    } else {
        (
            Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: remaining_height,
            },
            None,
        )
    };

    // ---------- 紧凑菜单卡片 ----------
    let item_h = 1u16;                                     // ← 每项 1 行
    let menu_w = 22u16.min(menu_area.width);               // ← 宽度 22
    let content_h = app.menu_options.len() as u16 * item_h;
    let menu_h = (content_h + 2).min(menu_area.height);    // ← 只留上下边框

    let menu_card = Rect {
        x: menu_area.x + (menu_area.width.saturating_sub(menu_w)) / 2,
        y: menu_area.y + (menu_area.height.saturating_sub(menu_h)) / 2,
        width: menu_w,
        height: menu_h,
    };

    // 卡片外框（圆角）
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(160, 110, 200)))
        .style(Style::default().bg(bg_color));
    frame.render_widget(card_block, menu_card);

    // 卡片内菜单
    let inner_menu = Rect {
        x: menu_card.x + 1,
        y: menu_card.y + 1,
        width: menu_card.width.saturating_sub(2),
        height: menu_card.height.saturating_sub(2),
    };

    for (i, text) in app.menu_options.iter().enumerate() {
        let y = inner_menu.y + i as u16 * item_h;
        if y >= inner_menu.y + inner_menu.height {
            break;
        }

        let is_selected = i == app.selected;

        // 选中项：金色背景条
        if is_selected {
            let bar = Rect {
                x: inner_menu.x,
                y,
                width: inner_menu.width,
                height: 1,
            };
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Rgb(255, 210, 100))),
                bar,
            );
        }

        // 文本
        let (label, style) = if is_selected {
            (
                format!(" ▶ {} ", text),
                Style::default()
                    .fg(Color::Rgb(30, 20, 40))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                format!("   {} ", text),
                Style::default().fg(Color::Rgb(200, 200, 220)),
            )
        };

        let para = Paragraph::new(Line::from(Span::styled(label, style)))
            .alignment(Alignment::Center);
        frame.render_widget(
            para,
            Rect {
                x: inner_menu.x,
                y,
                width: inner_menu.width,
                height: 1,
            },
        );
    }

    // ---------- 右侧图片 ----------
    if let Some(img) = &app.menu_image {
        if let Some(img_area) = image_area {
            let padded = Rect {
                x: img_area.x + 1,
                y: img_area.y + 1,
                width: img_area.width.saturating_sub(2),
                height: img_area.height.saturating_sub(2),
            };
            crate::image::draw_portrait(frame, padded, img, 2, 100);
        }
    }
}

// ==================== 布局 2：全屏背景 + 居中卡片菜单 ====================
fn render_menu_layout2(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    // 全屏背景图
    if let Some(img) = &app.menu_image {
        crate::image::draw_background(frame, inner, img);
    } else if let Some(img) = &app.logo {
        crate::image::draw_background(frame, inner, img);
    }

    // 居中卡片
    let card_w = 36u16.min(inner.width);
    let card_h = (app.menu_options.len() as u16 * 2 + 8).min(inner.height);
    let card = Rect {
        x: inner.x + (inner.width.saturating_sub(card_w)) / 2,
        y: inner.y + (inner.height.saturating_sub(card_h)) / 2,
        width: card_w,
        height: card_h,
    };

    frame.render_widget(ratatui::widgets::Clear, card);

    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Double)
        .border_style(Style::default().fg(Color::Rgb(212, 112, 212)))
        .style(Style::default().bg(bg_color));
    frame.render_widget(card_block, card);

    let inner_card = Rect {
        x: card.x + 1,
        y: card.y + 1,
        width: card.width.saturating_sub(2),
        height: card.height.saturating_sub(2),
    };

    // 标题在卡片内顶部
    let title = build_title_lines(app).style(Style::default().bg(bg_color));
    frame.render_widget(
        title,
        Rect {
            x: inner_card.x,
            y: inner_card.y,
            width: inner_card.width,
            height: 3,
        },
    );

    // 菜单
    let list = build_menu_list(app, bg_color, inner_card.width);
    let list_height = app.menu_options.len() as u16 * 2;
    let list_area = Rect {
        x: inner_card.x + (inner_card.width.saturating_sub(28)) / 2,
        y: inner_card.y + 3 + (inner_card.height.saturating_sub(3 + list_height)) / 2,
        width: 28.min(inner_card.width),
        height: list_height.min(inner_card.height.saturating_sub(3)),
    };
    frame.render_widget(list, list_area);
}

// ==================== 布局 3：左图右菜单 ====================
fn render_menu_layout3(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    let split = Layout::horizontal([
        Constraint::Percentage(45),
        Constraint::Percentage(55),
    ])
    .split(inner);

    let left = split[0];
    let right = split[1];

    // 左侧图片（优先 menu_image，其次 logo）
    if let Some(img) = &app.menu_image {
        crate::image::draw_portrait(frame, left, img, 2, 100);
    } else if let Some(img) = &app.logo {
        crate::image::draw_portrait(frame, left, img, 2, 100);
    }

    // 右侧标题 + 菜单
    let title = build_title_lines(app).style(Style::default().bg(bg_color));
    frame.render_widget(
        title,
        Rect {
            x: right.x,
            y: right.y + 2,
            width: right.width,
            height: 4,
        },
    );

    let list = build_menu_list(app, bg_color, right.width);
    let list_height = app.menu_options.len() as u16 * 2;
    let list_area = Rect {
        x: right.x + (right.width.saturating_sub(30)) / 2,
        y: right.y + 6 + (right.height.saturating_sub(6 + list_height)) / 2,
        width: 30.min(right.width),
        height: list_height.min(right.height.saturating_sub(6)),
    };
    frame.render_widget(list, list_area);
}

// ==================== 布局 4：顶部大图 + 底部横排菜单 ====================
fn render_menu_layout4(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    let split = Layout::vertical([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(inner);

    let top = split[0];
    let bottom = split[1];

    // 顶部图片（全屏拉伸）
    if let Some(img) = &app.menu_image {
        crate::image::draw_background(frame, top, img);
    } else if let Some(img) = &app.logo {
        crate::image::draw_background(frame, top, img);
    }

    // 底部：标题在上，菜单横向排列
    let title = build_title_lines(app).style(Style::default().bg(bg_color));
    frame.render_widget(
        title,
        Rect {
            x: bottom.x,
            y: bottom.y,
            width: bottom.width,
            height: 4.min(bottom.height),
        },
    );

    // 横排菜单
    let menu_line = app
        .menu_options
        .iter()
        .enumerate()
        .fold(String::new(), |acc, (i, txt)| {
            let marker = if i == app.selected { "▶ " } else { "  " };
            if acc.is_empty() {
                format!("{}{}", marker, txt)
            } else {
                format!("{}   {}{}", acc, marker, txt)
            }
        });

    let menu_para = Paragraph::new(menu_line)
        .style(Style::default().bg(bg_color))
        .alignment(Alignment::Center);
    frame.render_widget(
        menu_para,
        Rect {
            x: bottom.x,
            y: bottom.y + 4,
            width: bottom.width,
            height: 2,
        },
    );

    // 底部操作提示
    let footer = Paragraph::new(app.footer.as_str())
        .style(Style::default().fg(Color::Rgb(150, 150, 150)).bg(bg_color))
        .alignment(Alignment::Center);
    frame.render_widget(
        footer,
        Rect {
            x: bottom.x,
            y: bottom.y + bottom.height.saturating_sub(2),
            width: bottom.width,
            height: 1,
        },
    );
}

// ==================== 布局 5：居中大字 + 竖排菜单 ====================
fn render_menu_layout5(frame: &mut Frame, inner: Rect, app: &mut App, bg_color: Color) {
    // 大标题
    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "✨  N G A L  ✨",
            Style::default()
                .fg(Color::Rgb(212, 112, 212))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            app.title.clone(),
            Style::default().fg(Color::Rgb(200, 100, 255)),
        )),
        Line::from(Span::styled(
            "Terminal Visual Novel Engine",
            Style::default().fg(Color::Rgb(150, 150, 180)),
        )),
        Line::from(""),
    ])
    .alignment(Alignment::Center)
    .style(Style::default().bg(bg_color));

    frame.render_widget(
        title,
        Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 6.min(inner.height),
        },
    );

    // 竖排大字菜单
    let item_height = 2u16;
    let total_height = app.menu_options.len() as u16 * item_height;
    let start_y = inner.y + 8 + (inner.height.saturating_sub(8 + total_height)) / 2;

    for (i, text) in app.menu_options.iter().enumerate() {
        let marker = if i == app.selected { "▶  " } else { "    " };
        let style = if i == app.selected {
            Style::default()
                .fg(Color::Rgb(255, 255, 0))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(180, 180, 200))
        };
        let line = Line::from(vec![
            Span::styled(marker, style),
            Span::styled(text.clone(), style),
        ]);
        let para = Paragraph::new(line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg_color));
        let item_area = Rect {
            x: inner.x,
            y: start_y + i as u16 * item_height,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(para, item_area);
    }
}

// ============================================================
// 底部对话框
// ============================================================

fn render_bottom(frame: &mut Frame, area: Rect, app: &App) {
    let bg_color = get_bg_color(app);
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

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
            "按 +/- 调节BGM音量，[ ] 调节语音音量，A 切换自动播放，1/2 调节速度，T 切换文字动画，3/4 调节速度，B 切换背景，S 保存，ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::About => (
            "关于".to_string(),
            "按 ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::History => (
            "历史记录".to_string(),
            "↑↓/jk 滚动 | PgUp/PgDn 翻页 | Home/End 首尾 | ESC 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::GameMenu => (
            "游戏菜单".to_string(),
            "1返回游戏 2存档 3读档 q返回主界面 | ↑/↓选择 Enter确认".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::SaveSlot => (
            "存档".to_string(),
            "↑↓ 滚动 | Enter 保存 | ESC/q 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::LoadSlot => (
            "读档".to_string(),
            "↑↓ 滚动 | Enter 读取 | ESC/q 返回".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::Input { .. } => (
            "输入".to_string(),
            "请输入内容".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::InDialogue { .. } => (
            app.current_speaker().unwrap_or_else(|| "".to_string()),
            app.display_text.clone(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::InChoice { .. } => (
            app.current_speaker().unwrap_or_else(|| "系统".to_string()),
            "请选择一项：".to_string(),
            app.status_message.as_deref(),
        ),
        crate::app::AppState::EndOfFile => (
            "结束".to_string(),
            "剧情已完结，按任意键返回".to_string(),
            app.status_message.as_deref(),
        ),
    };

    let name_style = Style::default()
        .fg(Color::Rgb(255, 255, 255))
        .add_modifier(Modifier::BOLD)
        .bg(bg_color);
    if !speaker.is_empty() {
        let name_para =
            Paragraph::new(Line::from(Span::styled(speaker, name_style))).alignment(Alignment::Left);
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