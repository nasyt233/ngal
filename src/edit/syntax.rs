// src/edit/syntax.rs
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::state::{C_ACCENT, C_BLUE, C_GRAY, C_GREEN, C_PINK, C_RED, C_YELLOW};

pub fn is_scene_marker(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('[') && s.ends_with(']') && !s.contains(':') && s.len() >= 3
}

pub fn highlight_code(s: &str) -> Vec<Span<'static>> {
    let s = s.to_string();

    if is_scene_marker(&s) {
        return vec![Span::styled(
            s,
            Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD),
        )];
    }

    if s.trim() == "end" {
        return vec![Span::styled(
            s,
            Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
        )];
    }

    let keywords: &[(&str, Color)] = &[
        ("music:", C_BLUE),
        ("bg:", C_BLUE),
        ("img:", C_BLUE),
        ("choose:", C_PINK),
        ("load:", C_RED),
        ("input:", C_GREEN),
        ("if ", C_ACCENT),
    ];

    for (kw, color) in keywords {
        if s.starts_with(kw) {
            let rest = s[kw.len()..].to_string();
            let kw_str = s[..kw.len()].to_string();
            return vec![
                Span::styled(
                    kw_str,
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(rest, Style::default().fg(Color::Rgb(220, 220, 230))),
            ];
        }
    }

    if s.contains("$(") {
        return vec![Span::styled(
            s,
            Style::default().fg(Color::Rgb(180, 130, 255)),
        )];
    }

    if let Some(eq_pos) = s.find('=') {
        let left = s[..eq_pos].to_string();
        let right = s[eq_pos..].to_string();
        let is_assign =
            !s.contains(">=") && !s.contains("<=") && !s.contains("==") && !s.contains("!=");
        if is_assign && !left.contains('(') && left.trim().len() < 30 {
            return vec![
                Span::styled(left, Style::default().fg(Color::Rgb(150, 220, 255))),
                Span::styled(right, Style::default().fg(Color::Rgb(220, 220, 230))),
            ];
        }
    }

    if let Some(colon_pos) = s.find(':') {
        let speaker = &s[..colon_pos];
        let text = &s[colon_pos..];
        if !speaker.contains('=')
            && !speaker.contains(' ')
            && speaker.len() < 24
            && !speaker.is_empty()
        {
            return vec![
                Span::styled(speaker.to_string(), Style::default().fg(C_ACCENT)),
                Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::Rgb(220, 220, 230)),
                ),
            ];
        }
    }

    vec![Span::styled(
        s,
        Style::default().fg(Color::Rgb(220, 220, 230)),
    )]
}

pub fn highlight_line(s: &str) -> Vec<Span<'static>> {
    if let Some(hash_pos) = s.find('#') {
        let code_part = &s[..hash_pos];
        let comment_part = &s[hash_pos..];
        let mut spans = highlight_code(code_part);
        spans.push(Span::styled(
            comment_part.to_string(),
            Style::default().fg(C_GRAY),
        ));
        return spans;
    }
    highlight_code(s)
}

pub fn highlight_line_with_cursor(s: &str, cursor_col: usize) -> Vec<Span<'static>> {
    let chars: Vec<char> = s.chars().collect();
    let pos = cursor_col.min(chars.len());

    let mut spans = Vec::new();

    if pos > 0 {
        let before: String = chars[..pos].iter().collect();
        spans.extend(highlight_line(&before));
    }

    if pos < chars.len() {
        let c = chars[pos].to_string();
        spans.push(Span::styled(
            c,
            Style::default()
                .fg(Color::Black)
                .bg(C_GREEN)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled(" ", Style::default().bg(C_GREEN)));
    }

    if pos + 1 < chars.len() {
        let after: String = chars[pos + 1..].iter().collect();
        spans.extend(highlight_line(&after));
    }

    spans
}