use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::panic;
use std::time::Duration;

use ngal::app::App;
use ngal::ui;

fn main() -> Result<()> {
    // 设置 panic hook 确保终端恢复
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

        // 自动播放更新
        app.update_auto_play();

        // 非阻塞事件轮询
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            if let ngal::app::AppState::Menu = app.state {
                                break; // 主菜单按 q 退出
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
                            // 滚轮向上滚动，菜单选项向上移动
                            if let ngal::app::AppState::Menu = app.state {
                                if app.selected > 0 {
                                    app.selected -= 1;
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            // 滚轮向下滚动，菜单选项向下移动
                            if let ngal::app::AppState::Menu = app.state {
                                if app.selected < app.menu_options.len() - 1 {
                                    app.selected += 1;
                                }
                            }
                        }
                        MouseEventKind::Down(button) => {
                            // 鼠标按下事件
                            if button == MouseButton::Left {
                                // 获取当前鼠标位置，mouse_x 暂时未使用，加下划线避免警告
                                let (_mouse_x, mouse_y) = (mouse.column, mouse.row);
                                
                                // 只有在主菜单状态才处理鼠标点击
                                if let ngal::app::AppState::Menu = app.state {
                                    // 获取终端尺寸
                                    let size = terminal.size()?;
                                    
                                    // 计算菜单区域的位置（与 ui.rs 中的布局保持一致）
                                    let y_offset = if app.logo.is_some() { 6 } else { 0 };
                                    let menu_start_y = (size.height.saturating_sub(8)) / 2 + y_offset + 4;
                                    let menu_height = app.menu_options.len() as u16;
                                    
                                    // 检查鼠标点击是否在菜单区域内
                                    if mouse_y >= menu_start_y && mouse_y < menu_start_y + menu_height {
                                        let selected_index = (mouse_y - menu_start_y) as usize;
                                        if selected_index < app.menu_options.len() {
                                            // 设置选中的选项并执行
                                            app.selected = selected_index;
                                            app.execute_menu();
                                        }
                                    }
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