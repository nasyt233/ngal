use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::env;
use std::io::stdout;
use std::panic;
use std::time::Duration;

use ngal::app::App;
use ngal::args::Args;
use ngal::ui;

fn main() -> Result<()> {
    let args = Args::parse();
    if args.help {
        Args::print_help();
        return Ok(());
    }

    if args.game_dir != std::path::PathBuf::from(".") {
        env::set_current_dir(&args.game_dir)?;
    }

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
        terminal.draw(|f| ui::draw(f, &mut app))?;

        app.update_auto_play();

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            if let ngal::app::AppState::Menu = app.state {
                                break;
                            } else {
                                app.handle_event(key.code);
                            }
                        }
                        _ => app.handle_event(key.code),
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if let ngal::app::AppState::Menu = app.state {
                                if app.selected > 0 {
                                    app.selected -= 1;
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if let ngal::app::AppState::Menu = app.state {
                                if app.selected < app.menu_options.len() - 1 {
                                    app.selected += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
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