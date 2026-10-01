// src/edit/render.rs
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::state::{
    CompletionKind, EditorMode, FileNameAction, C_ACCENT, C_BG, C_BG_ALT, C_BLUE, C_BORDER_DIM,
    C_BORDER_FOCUS, C_GRAY, C_GREEN, C_PINK, C_RED, C_YELLOW, MENU_ITEMS,
};
use super::syntax::{highlight_line, highlight_line_with_cursor};
use super::Editor;

impl Editor {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let area = frame.size();

        frame.render_widget(Block::default().style(Style::default().bg(C_BG)), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        let main_area = chunks[0];
        let status_area = chunks[1];

        let body_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(main_area);

        let header_area = body_main[0];
        let below_header = body_main[1];

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(26), Constraint::Min(0)])
            .split(below_header);

        let sidebar_area = cols[0];
        let content_area = cols[1];

        self.content_area_width = content_area.width;

        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(sidebar_area);

        let menu_area = sidebar_chunks[0];
        let file_list_area = sidebar_chunks[1];

        self.menu_area = menu_area;
        self.file_list_area = file_list_area;
        self.content_area = content_area;

        // 顶部标题
        let mode_label = match self.mode {
            EditorMode::Normal => "菜单",
            EditorMode::FileListFocus => "文件",
            EditorMode::DirectEdit => "编辑",
            EditorMode::Input => "输入",
            EditorMode::FilePicker => "选择文件",
            EditorMode::ScenePicker => "选择场景",
            EditorMode::CharacterPicker => "选择角色",
            EditorMode::ConfirmDelete => "确认删除",
            EditorMode::FileNameInput { .. } => "文件名",
            EditorMode::Shell { .. } => "终端",
            EditorMode::ShowStats => "统计",
        };
        let title_line = Line::from(vec![
            Span::styled(
                "  ngal editor ",
                Style::default()
                    .fg(C_BG)
                    .bg(C_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}  ", self.file_path.display()),
                Style::default().fg(C_GRAY),
            ),
            Span::styled(
                format!("[{}] ", mode_label),
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(title_line).style(Style::default().bg(C_BG_ALT)),
            header_area,
        );

        let is_editing = matches!(self.mode, EditorMode::DirectEdit);
        let is_menu_focus = matches!(self.mode, EditorMode::Normal);
        let is_filelist_focus = matches!(self.mode, EditorMode::FileListFocus);

        // 功能菜单
        let items: Vec<ListItem> = MENU_ITEMS
            .iter()
            .map(|label| {
                if *label == "────────────────" {
                    ListItem::new(Line::from(Span::styled(
                        *label,
                        Style::default().fg(C_BORDER_DIM),
                    )))
                } else {
                    let color = match *label {
                        "运行测试 (F5)" => C_GREEN,
                        "检查统计" => C_BLUE,
                        "打包游戏" => C_BLUE,
                        "保存文件" => C_YELLOW,
                        "命令执行" => C_ACCENT,
                        "插入输入" => C_GREEN,
                        "插入等待" => C_ACCENT,
                        "退出" => C_RED,
                        "撤销 (Ctrl+Z)" => C_PINK,
                        _ => Color::White,
                    };
                    ListItem::new(Line::from(Span::styled(*label, Style::default().fg(color))))
                }
            })
            .collect();

        let (menu_border, menu_title) = if is_menu_focus {
            (C_BORDER_FOCUS, " ◆ 功能 ")
        } else {
            (C_BORDER_DIM, "   功能 ")
        };

        let sidebar_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(
                        menu_title,
                        Style::default()
                            .fg(if is_menu_focus { C_PINK } else { C_GRAY })
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(menu_border))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_PINK)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(sidebar_list, menu_area, &mut self.sidebar_state);

        // 剧情文件列表
        let file_items: Vec<ListItem> = if self.story_files.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (无文件，按 n 新建)",
                Style::default().fg(C_GRAY),
            )))]
        } else {
            let current_name = self
                .file_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            self.story_files
                .iter()
                .map(|f| {
                    let is_current = f == &current_name;
                    let size_str = std::fs::metadata(self.dialog_dir.join(f))
                        .ok()
                        .map(|m| {
                            let kb = m.len() / 1024;
                            if kb < 1 {
                                format!("{}B", m.len())
                            } else {
                                format!("{}K", kb)
                            }
                        })
                        .unwrap_or_default();
                    if is_current {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                "● ",
                                Style::default()
                                    .fg(C_GREEN)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                f.clone(),
                                Style::default()
                                    .fg(C_GREEN)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("  {}", size_str), Style::default().fg(C_GRAY)),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                f.clone(),
                                Style::default().fg(Color::Rgb(210, 210, 220)),
                            ),
                            Span::styled(format!("  {}", size_str), Style::default().fg(C_GRAY)),
                        ]))
                    }
                })
                .collect()
        };

        let (file_border, file_title) = if is_filelist_focus {
            (C_BLUE, " ◆ 剧情文件 (n新建 d删 m改) ")
        } else {
            (C_BORDER_DIM, "   剧情文件 ")
        };

        let file_list = List::new(file_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(
                        file_title,
                        Style::default()
                            .fg(if is_filelist_focus { C_BLUE } else { C_GRAY })
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(file_border))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_BLUE)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(file_list, file_list_area, &mut self.story_state);

        // 右侧内容
        let total_lines = self.content.len();
        let line_num_width = if total_lines < 10 {
            1
        } else if total_lines < 100 {
            2
        } else if total_lines < 1000 {
            3
        } else {
            4
        };

        let content_lines: Vec<Line> = self
            .content
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let line_num = format!(" {:>width$} │ ", i + 1, width = line_num_width);
                let line_num_style = if i == self.content_cursor {
                    Style::default()
                        .fg(C_ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(C_GRAY)
                };

                let mut spans = vec![Span::styled(line_num, line_num_style)];

                if i == self.content_cursor && is_editing {
                    spans.extend(highlight_line_with_cursor(s, self.content_col));
                } else if i == self.content_cursor {
                    let line_spans = highlight_line(s);
                    for sp in line_spans {
                        spans.push(Span::styled(
                            sp.content.to_string(),
                            sp.style.add_modifier(Modifier::BOLD),
                        ));
                    }
                } else {
                    spans.extend(highlight_line(s));
                }

                Line::from(spans)
            })
            .collect();

        let undo_hint = if self.undo_stack.is_empty() {
            String::new()
        } else {
            format!(" ↶{}", self.undo_stack.len())
        };
        let clipboard_hint = if self.clipboard.is_empty() {
            String::new()
        } else {
            format!(" 📋{}", self.clipboard.len())
        };
        let hscroll_hint = if self.content_hscroll > 0 {
            format!(" →{}", self.content_hscroll)
        } else {
            String::new()
        };
        let line_info = format!(" {} / {} ", self.content_cursor + 1, self.content.len());

        let content_title = Line::from(vec![
            Span::styled(" ✎ ", Style::default().fg(C_BLUE)),
            Span::styled(
                self.file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled"),
                Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD),
            ),
            Span::styled(undo_hint, Style::default().fg(C_ACCENT)),
            Span::styled(clipboard_hint, Style::default().fg(C_YELLOW)),
            Span::styled(hscroll_hint, Style::default().fg(C_PINK)),
            Span::styled(format!("  行{}", line_info), Style::default().fg(C_GRAY)),
        ]);

        let (content_border, content_border_type) = if is_editing {
            (C_GREEN, BorderType::Double)
        } else {
            (C_BORDER_FOCUS, BorderType::Rounded)
        };

        let content_para = Paragraph::new(content_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(content_border_type)
                    .title(content_title)
                    .border_style(Style::default().fg(content_border))
                    .style(Style::default().bg(C_BG)),
            )
            .scroll((self.content_scroll, self.content_hscroll));

        frame.render_widget(content_para, content_area);

        // 底部状态栏
        let (status_text, status_style, status_border) = match self.mode {
            EditorMode::Normal => {
                let text = self.status_message.clone().unwrap_or_else(|| {
                    " hjkl/↑↓菜单 │ ←→内容 │ Enter确认 │ 空格换行 │ i编辑 │ Tab文件 │ x/c/v │ h帮助 │ q退出"
                        .to_string()
                });
                (format!(" {}", text), Style::default().fg(C_GRAY), C_BORDER_DIM)
            }
            EditorMode::FileListFocus => {
                let text = self.status_message.clone().unwrap_or_else(|| {
                    " ↑↓/jk选择 │ n新建 │ d删除 │ m重命名 │ r刷新 │ Enter打开 │ Tab编辑 │ ESC返回"
                        .to_string()
                });
                (format!(" {}", text), Style::default().fg(C_BLUE), C_BLUE)
            }
            EditorMode::Input => (
                format!(" ✎ {}: {}", self.input_prompt, self.input_buffer),
                Style::default().fg(C_GREEN),
                C_GREEN,
            ),
            EditorMode::FilePicker => (
                " 输入搜索 │ ↑↓进入列表 │ Enter确认 │ ESC取消".to_string(),
                Style::default().fg(C_BLUE),
                C_BLUE,
            ),
            EditorMode::ScenePicker => (
                " 输入搜索 │ ↑↓进入列表 │ Enter确认 │ ESC取消".to_string(),
                Style::default().fg(C_ACCENT),
                C_ACCENT,
            ),
            EditorMode::CharacterPicker => (
                " ↑↓/jk选择角色 │ Enter确认 │ ESC取消".to_string(),
                Style::default().fg(C_PINK),
                C_PINK,
            ),
            EditorMode::DirectEdit => {
                let text = self.status_message.clone().unwrap_or_else(|| {
                    " 编辑中 │ 打字触发补全 │ Tab/Enter 选择 │ ESC 关闭补全 │ Ctrl+S 保存 │ F5 测试"
                        .to_string()
                });
                (format!(" {}", text), Style::default().fg(C_GREEN), C_GREEN)
            }
            EditorMode::ConfirmDelete => (
                " ⚠ 确认删除？ (y确认 / n取消)".to_string(),
                Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
                C_RED,
            ),
            EditorMode::FileNameInput { .. } => (String::new(), Style::default(), C_PINK),
            EditorMode::Shell { .. } => (
                " 终端模式 │ 输入命令回车执行 │ ESC退出".to_string(),
                Style::default().fg(C_ACCENT),
                C_ACCENT,
            ),
            EditorMode::ShowStats => (
                " 统计信息 │ 按任意键关闭".to_string(),
                Style::default().fg(C_BLUE),
                C_BLUE,
            ),
        };

        let status_para = Paragraph::new(status_text)
            .style(status_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(status_border))
                    .style(Style::default().bg(C_BG_ALT)),
            );

        frame.render_widget(status_para, status_area);

        // ============ 弹窗 ============

        self.render_file_picker(frame, area);
        self.render_scene_picker(frame, area);
        self.render_character_picker(frame, area);
        self.render_confirm_delete(frame, area);
        self.render_file_name_input(frame, area);
        self.render_shell(frame, area);
        self.render_stats(frame, area);
        self.render_help(frame, area);
        self.render_completion(frame, area);
    }

    // ============================================================
    // 命令补全弹窗
    // ============================================================
    fn render_completion(&self, frame: &mut Frame, area: Rect) {
        if !self.completion_active || self.completion_items.is_empty() {
            return;
        }

        let inner = self.content_area;
        let popup_w = 46u16.min(inner.width.max(20));
        let popup_h = (self.completion_items.len() as u16 + 2).min(12);

        // 定位：内容区右上角向内偏移
        let popup_x = inner
            .x
            .saturating_add(inner.width.saturating_sub(popup_w + 2));
        let popup_y = inner.y + 1;

        let popup_area = Rect {
            x: popup_x.min(area.width.saturating_sub(popup_w)),
            y: popup_y.min(area.height.saturating_sub(popup_h)),
            width: popup_w,
            height: popup_h,
        };

        frame.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = self
            .completion_items
            .iter()
            .map(|item| {
                let kind_hint = match item.kind {
                    CompletionKind::Insert => "",
                    CompletionKind::FilePicker(_) => " [文件]",
                    CompletionKind::ScenePicker => " [场景]",
                    CompletionKind::CharacterPicker => " [角色]",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<10}", item.label),
                        Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}{}", item.desc, kind_hint),
                        Style::default().fg(C_GRAY),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title(format!(" ◆ 补全: {} ", self.completion_query))
                    .border_style(Style::default().fg(C_GREEN))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_GREEN)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶");

        let mut state = ListState::default();
        state.select(Some(self.completion_selected));
        frame.render_stateful_widget(list, popup_area, &mut state);
    }

    // ============================================================
    // 文件选择器（带搜索）
    // ============================================================
    fn render_file_picker(&self, frame: &mut Frame, area: Rect) {
        if !matches!(self.mode, EditorMode::FilePicker) {
            return;
        }
        let popup_area = centered_rect(60, 60, area);
        frame.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // 搜索栏
        let cursor = if self.picker_focused { "█" } else { "" };
        let search_text = format!(" 🔍 {}{}", self.picker_query, cursor);
        let search_style = if self.picker_focused {
            Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_GRAY)
        };
        let search_para = Paragraph::new(search_text).style(search_style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 搜索 (↑↓ 进入列表) ")
                .border_style(if self.picker_focused {
                    Style::default().fg(C_GREEN)
                } else {
                    Style::default().fg(C_BORDER_DIM)
                })
                .style(Style::default().bg(C_BG_ALT)),
        );
        frame.render_widget(search_para, chunks[0]);

        // 列表
        let items: Vec<ListItem> = if self.picker_files.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (无匹配文件)",
                Style::default().fg(C_GRAY),
            )))]
        } else {
            self.picker_files
                .iter()
                .map(|f| ListItem::new(f.as_str()))
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title(format!(" ◆ {} ", self.picker_title))
                    .border_style(Style::default().fg(C_BLUE))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_BLUE)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        if !self.picker_files.is_empty() {
            list_state.select(Some(self.picker_selected));
        }
        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // ============================================================
    // 场景选择器（带搜索）
    // ============================================================
    fn render_scene_picker(&self, frame: &mut Frame, area: Rect) {
        if !matches!(self.mode, EditorMode::ScenePicker) {
            return;
        }
        let popup_area = centered_rect(70, 70, area);
        frame.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // 搜索栏
        let cursor = if self.scene_focused { "█" } else { "" };
        let search_text = format!(" 🔍 {}{}", self.scene_query, cursor);
        let search_style = if self.scene_focused {
            Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(C_GRAY)
        };
        let search_para = Paragraph::new(search_text).style(search_style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 搜索 (↑↓ 进入列表) ")
                .border_style(if self.scene_focused {
                    Style::default().fg(C_GREEN)
                } else {
                    Style::default().fg(C_BORDER_DIM)
                })
                .style(Style::default().bg(C_BG_ALT)),
        );
        frame.render_widget(search_para, chunks[0]);

        // 列表
        let items: Vec<ListItem> = if self.scene_list.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  (没有找到场景)",
                Style::default().fg(C_GRAY),
            )))]
        } else {
            self.scene_list
                .iter()
                .map(|(scene, file)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:<24}", scene),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(format!("  [{}]", file), Style::default().fg(C_GRAY)),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title(format!(" ◆ {} ", self.scene_title))
                    .border_style(Style::default().fg(C_ACCENT))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        if !self.scene_list.is_empty() {
            list_state.select(Some(self.scene_selected));
        }
        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // ============================================================
    // 角色选择器
    // ============================================================
    fn render_character_picker(&self, frame: &mut Frame, area: Rect) {
        if !matches!(self.mode, EditorMode::CharacterPicker) {
            return;
        }
        let popup_area = centered_rect(60, 60, area);
        frame.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = self
            .character_list
            .iter()
            .map(|c| match c {
                None => ListItem::new(Line::from(Span::styled(
                    " ✎ 自定义（手动输入）",
                    Style::default().fg(C_ACCENT),
                ))),
                Some(name) => ListItem::new(Line::from(Span::styled(
                    format!(" {}", name),
                    Style::default().fg(Color::White),
                ))),
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title(format!(" ◆ {} ", self.character_title))
                    .border_style(Style::default().fg(C_PINK))
                    .style(Style::default().bg(C_BG_ALT)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(C_PINK)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        if !self.character_list.is_empty() {
            list_state.select(Some(self.character_selected));
        }
        frame.render_stateful_widget(list, popup_area, &mut list_state);
    }

    // ============================================================
    // 删除确认
    // ============================================================
    fn render_confirm_delete(&self, frame: &mut Frame, area: Rect) {
        if !matches!(self.mode, EditorMode::ConfirmDelete) {
            return;
        }
        let popup_area = centered_rect(50, 25, area);
        frame.render_widget(Clear, popup_area);

        let filename = self
            .story_files
            .get(self.story_selected)
            .cloned()
            .unwrap_or_default();

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " ⚠ 确认删除文件 ",
                    Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    filename,
                    Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " ？",
                    Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(" 此操作不可恢复！", Style::default().fg(C_GRAY))),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [y] ", Style::default().fg(Color::Black).bg(C_RED)),
                Span::raw(" 确认   "),
                Span::styled(" [n] ", Style::default().fg(Color::Black).bg(C_GREEN)),
                Span::raw(" 取消"),
            ]),
        ];

        let para = Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(C_RED))
                .style(Style::default().bg(C_BG_ALT)),
        );

        frame.render_widget(para, popup_area);
    }

    // ============================================================
    // 文件名输入
    // ============================================================
    fn render_file_name_input(&self, frame: &mut Frame, area: Rect) {
        let (action, buffer) = match &self.mode {
            EditorMode::FileNameInput { action, buffer } => (action, buffer),
            _ => return,
        };
        let popup_area = centered_rect(60, 25, area);
        frame.render_widget(Clear, popup_area);

        let (title, hint) = match action {
            FileNameAction::Create => (" 新建文件 ", " 输入文件名 (自动添加 .ng 后缀) "),
            FileNameAction::Rename(_) => (" 重命名文件 ", " 输入新文件名 (自动添加 .ng 后缀) "),
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(hint, Style::default().fg(C_GRAY))]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " > ",
                    Style::default().fg(C_PINK).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    buffer.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "_",
                    Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Enter确认 │ ESC取消 ",
                Style::default().fg(C_GRAY),
            )),
        ];

        let para = Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(title)
                .border_style(Style::default().fg(C_PINK))
                .style(Style::default().bg(C_BG_ALT)),
        );

        frame.render_widget(para, popup_area);
    }

    // ============================================================
    // 内嵌终端
    // ============================================================
    fn render_shell(&self, frame: &mut Frame, area: Rect) {
        let (buffer, output) = match &self.mode {
            EditorMode::Shell { buffer, output } => (buffer, output),
            _ => return,
        };
        let popup_area = centered_rect(80, 75, area);
        frame.render_widget(Clear, popup_area);

        let shell_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(popup_area);

        let out_area = shell_chunks[0];
        let in_area = shell_chunks[1];

        let out_lines: Vec<Line> = output
            .iter()
            .rev()
            .take(out_area.height as usize)
            .rev()
            .map(|s| {
                if s.starts_with("$ ") {
                    Line::from(Span::styled(s.clone(), Style::default().fg(C_GREEN)))
                } else {
                    Line::from(Span::styled(
                        s.clone(),
                        Style::default().fg(Color::Rgb(220, 220, 230)),
                    ))
                }
            })
            .collect();

        let out_para = Paragraph::new(out_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" 终端输出 ")
                .border_style(Style::default().fg(C_ACCENT))
                .style(Style::default().bg(Color::Rgb(16, 16, 22))),
        );

        frame.render_widget(out_para, out_area);

        let in_line = Line::from(vec![
            Span::styled(
                " $ ",
                Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                buffer.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "█",
                Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
            ),
        ]);

        let in_para = Paragraph::new(in_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 命令 (ESC退出) ")
                .border_style(Style::default().fg(C_GREEN))
                .style(Style::default().bg(C_BG_ALT)),
        );

        frame.render_widget(in_para, in_area);
    }

    // ============================================================
    // 项目统计
    // ============================================================
    fn render_stats(&self, frame: &mut Frame, area: Rect) {
        let info = match &self.pending_stats {
            Some(i) => i,
            None => return,
        };
        let popup_area = centered_rect(60, 80, area);
        frame.render_widget(Clear, popup_area);

        let mb = |bytes: u64| bytes as f64 / 1024.0 / 1024.0;

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  项目名: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    info.project_name.clone(),
                    Style::default().fg(C_PINK).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  版本号: ", Style::default().fg(C_GRAY)),
                Span::styled(info.version.clone(), Style::default().fg(C_ACCENT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  当前总大小: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    format!("{:.2} MB", mb(info.total_size)),
                    Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  图片: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    format!("{:>4} 张", info.img_count),
                    Style::default().fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(
                    format!("{:.2} MB", mb(info.img_size)),
                    Style::default().fg(C_BLUE),
                ),
            ]),
            Line::from(vec![
                Span::styled("  音乐: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    format!("{:>4} 首", info.music_count),
                    Style::default().fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(
                    format!("{:.2} MB", mb(info.music_size)),
                    Style::default().fg(C_BLUE),
                ),
            ]),
            Line::from(vec![
                Span::styled("  语音: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    format!("{:>4} 个", info.voice_count),
                    Style::default().fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(
                    format!("{:.2} MB", mb(info.voice_size)),
                    Style::default().fg(C_BLUE),
                ),
            ]),
            Line::from(vec![
                Span::styled("  剧情: ", Style::default().fg(C_GRAY)),
                Span::styled(
                    format!("{:>4} 个", info.story_count),
                    Style::default().fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(
                    format!("{} 字", info.total_words),
                    Style::default().fg(C_YELLOW),
                ),
            ]),
        ];

        let has_missing = !info.missing_images.is_empty()
            || !info.missing_music.is_empty()
            || !info.missing_voices.is_empty();

        if has_missing {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  缺失内容",
                Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
            )));

            if !info.missing_images.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  图片",
                    Style::default().fg(C_ACCENT),
                )));
                for f in &info.missing_images {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", f),
                        Style::default().fg(Color::Rgb(220, 220, 230)),
                    )));
                }
            }
            if !info.missing_music.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  音乐",
                    Style::default().fg(C_ACCENT),
                )));
                for f in &info.missing_music {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", f),
                        Style::default().fg(Color::Rgb(220, 220, 230)),
                    )));
                }
            }
            if !info.missing_voices.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  语音",
                    Style::default().fg(C_ACCENT),
                )));
                for f in &info.missing_voices {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", f),
                        Style::default().fg(Color::Rgb(220, 220, 230)),
                    )));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  按任意键关闭",
            Style::default().fg(C_GRAY),
        )));

        let para = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" ◆ 项目统计 ")
                .border_style(
                    Style::default()
                        .fg(C_BLUE)
                        .add_modifier(Modifier::BOLD),
                )
                .style(Style::default().bg(C_BG_ALT)),
        );

        frame.render_widget(para, popup_area);
    }

    // ============================================================
    // 快捷键帮助
    // ============================================================
    fn render_help(&self, frame: &mut Frame, area: Rect) {
        if !self.show_help {
            return;
        }
        let popup_area = centered_rect(70, 90, area);
        frame.render_widget(Clear, popup_area);

        let help_lines = vec![
            Line::from(Span::styled(
                "  ngal 编辑器 - 快捷键帮助  ",
                Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 功能区 ",
                Style::default().fg(C_PINK).add_modifier(Modifier::BOLD),
            )),
            Line::from("  ↑/↓/j/k     上下选择菜单"),
            Line::from("  ←/→          内容区光标上/下"),
            Line::from("  Home/End     跳到菜单首/尾"),
            Line::from("  PgUp/PgDn    内容区快速跳转"),
            Line::from("  鼠标滚轮     滚动内容区"),
            Line::from("  Enter        执行选中功能"),
            Line::from("  空格         在光标处换行"),
            Line::from("  i            进入右侧编辑模式"),
            Line::from("  h            显示本帮助"),
            Line::from("  x/c/v        剪切/复制/粘贴行"),
            Line::from("  d/Delete     删除当前行"),
            Line::from("  Tab          切换到文件列表"),
            Line::from("  F5           运行测试"),
            Line::from("  q            保存并退出"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 剧情文件列表 ",
                Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD),
            )),
            Line::from("  ↑/↓/j/k      选择文件"),
            Line::from("  Home/End     首/尾"),
            Line::from("  PgUp/PgDn    快速翻页"),
            Line::from("  Enter        打开选中文件"),
            Line::from("  n            新建文件"),
            Line::from("  d            删除文件 (需确认)"),
            Line::from("  m            重命名文件"),
            Line::from("  r            刷新文件列表"),
            Line::from("  Tab          切到编辑区"),
            Line::from("  ESC          返回功能区"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 编辑模式 ",
                Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
            )),
            Line::from("  ←→↑↓         移动光标"),
            Line::from("  Home/End     行首/行尾"),
            Line::from("  PgUp/PgDn    翻页"),
            Line::from("  Enter        插入换行"),
            Line::from("  Backspace    删除字符"),
            Line::from("  Ctrl+S       保存文件"),
            Line::from("  Ctrl+Z       撤销"),
            Line::from("  F5           运行测试"),
            Line::from("  Tab/ESC      退出编辑"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 命令补全 ",
                Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
            )),
            Line::from("  打字         自动弹出匹配的命令"),
            Line::from("               (如 inp → input:)"),
            Line::from("  Tab/Enter    接受选中项"),
            Line::from("  ↑↓           在补全列表中移动"),
            Line::from("  ESC          关闭补全框"),
            Line::from("  唯一匹配     自动填入，无需选择"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 自动配对 ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from("  \" ' { [ ( <   输入时自动补右半边"),
            Line::from("  $()          在 input 中可直接用"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 选择器 (文件/场景/角色) ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from("  打字         搜索过滤"),
            Line::from("  ↑↓           从搜索栏进入列表"),
            Line::from("  鼠标滚轮     滚动列表"),
            Line::from("  Enter        确认选择"),
            Line::from("  ESC          取消"),
            Line::from(""),
            Line::from(Span::styled(
                " ◆ 鼠标操作 ",
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from("  滚轮         滚动内容 / 列表"),
            Line::from("  第一次点击   切换焦点/选中"),
            Line::from("  再次点击     执行 / 打开"),
            Line::from(""),
            Line::from(Span::styled(
                "  按任意键关闭本窗口",
                Style::default().fg(C_GRAY),
            )),
        ];

        let help_para = Paragraph::new(help_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" ◆ 帮助 ")
                .border_style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Rgb(18, 18, 28))),
        );

        frame.render_widget(help_para, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}