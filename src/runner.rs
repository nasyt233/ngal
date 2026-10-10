// src/runner.rs
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::panic;
use std::time::Duration;

use crate::app::App;
use crate::ui;

/// 启动游戏主循环
pub fn run_game() -> Result<()> {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    loop {
        app.update_animation();
        app.tick_sleep();
        terminal.draw(|f| ui::draw(f, &mut app))?;
        app.update_auto_play();

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    // Windows 下会同时发 Press 和 Release，只处理 Press / Repeat
                    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            if let crate::app::AppState::Menu = app.state {
                                break;
                            } else {
                                app.handle_event(key.code);
                            }
                        }
                        _ => app.handle_event(key.code),
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => match app.state {
                        crate::app::AppState::Menu => {
                            if app.selected > 0 {
                                app.selected -= 1;
                            }
                        }
                        crate::app::AppState::Gallery => {
                            if let Some(g) = &mut app.gallery {
                                g.prev();
                            }
                        }
                        _ => {}
                    },
                    MouseEventKind::ScrollDown => match app.state {
                        crate::app::AppState::Menu => {
                            if app.selected < app.menu_options.len() - 1 {
                                app.selected += 1;
                            }
                        }
                        crate::app::AppState::Gallery => {
                            if let Some(g) = &mut app.gallery {
                                g.next();
                            }
                        }
                        _ => {}
                    },
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let crate::app::AppState::Gallery = app.state {
                            if let Some(g) = &mut app.gallery {
                                let la = g.list_area;
                                let col = mouse.column;
                                let row = mouse.row;
                
                                if la.width > 2
                                    && la.height > 2
                                    && col >= la.x
                                    && col < la.x + la.width
                                    && row >= la.y + 1
                                    && row < la.y + la.height - 1
                                {
                                    let idx = (row - la.y - 1) as usize;
                                    if idx < g.images.len() {
                                        g.selected = idx;
                                        g.load_current();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    app.stop_voice();
    app.stop_bgm();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}