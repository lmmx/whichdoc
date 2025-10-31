//! whichdoc: A cargo documentation diagnostics-driven editor.
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use whichdoc::{app_state, edit_plan, input, ui};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let coords = if args.len() > 2 && args[1] == "--load-docs" {
        let file_content = std::fs::read_to_string(&args[2])?;
        let plan: edit_plan::EditPlan = serde_json::from_str(&file_content)?;
        let coords = input::read_cargo_diagnostics()?;
        let mut state = app_state::AppState::new(coords);
        state.load_docs(plan);
        return run_tui(state);
    } else {
        input::read_cargo_diagnostics()?
    };

    if coords.is_empty() {
        eprintln!("No missing_docs diagnostics found");
        return Ok(());
    }

    let state = app_state::AppState::new(coords);
    run_tui(state)
}

fn run_tui(mut app: app_state::AppState) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    } else {
        let plan = app.generate_edit_plan();
        let json = serde_json::to_string_pretty(&plan)?;
        println!("{}", json);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut app_state::AppState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.current_view {
                app_state::View::List => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => {
                            if app.list_index > 0 {
                                app.list_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.list_index < app.entries.len() - 1 {
                                app.list_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            app.enter_detail_view();
                        }
                        _ => {}
                    }
                }
                app_state::View::Detail => {
                    match key.code {
                        KeyCode::Char(':') => {
                            app.current_view = app_state::View::Command;
                            app.command_buffer.clear();
                            app.message = None;
                        }
                        KeyCode::Char(c) => {
                            app.detail_text.push(c);
                            app.entries[app.list_index].dirty = app.detail_text != app.detail_saved_text;
                        }
                        KeyCode::Backspace => {
                            app.detail_text.pop();
                            app.entries[app.list_index].dirty = app.detail_text != app.detail_saved_text;
                        }
                        KeyCode::Enter => {
                            app.detail_text.push('\n');
                        }
                        KeyCode::Esc => {
                            if app.entries[app.list_index].dirty {
                                app.message = Some("Unsaved changes! Use :q to discard or :x to save".to_string());
                            } else {
                                app.exit_detail_view(false);
                            }
                        }
                        _ => {}
                    }
                }
                app_state::View::Command => {
                    match key.code {
                        KeyCode::Char(c) => {
                            app.command_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.command_buffer.pop();
                        }
                        KeyCode::Enter => {
                            let cmd = app.command_buffer.clone();
                            app.current_view = app_state::View::Detail;

                            match cmd.as_str() {
                                "w" => {
                                    app.save_current();
                                }
                                "x" => {
                                    app.save_current();
                                    app.exit_detail_view(true);
                                }
                                "q" => {
                                    if app.entries[app.list_index].dirty {
                                        app.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                                        app.current_view = app_state::View::Detail;
                                    } else {
                                        app.exit_detail_view(false);
                                    }
                                }
                                "q!" => {
                                    app.exit_detail_view(false);
                                }
                                "wn" => {
                                    app.save_current();
                                    if let Some(next) = app.find_next_undocumented() {
                                        app.exit_detail_view(true);
                                        app.list_index = next;
                                        app.enter_detail_view();
                                    } else {
                                        app.message = Some("No more undocumented items".to_string());
                                    }
                                }
                                "wp" => {
                                    app.save_current();
                                    if let Some(prev) = app.find_prev_undocumented() {
                                        app.exit_detail_view(true);
                                        app.list_index = prev;
                                        app.enter_detail_view();
                                    } else {
                                        app.message = Some("No previous undocumented items".to_string());
                                    }
                                }
                                _ => {
                                    app.message = Some(format!("Unknown command: {}", cmd));
                                }
                            }
                            app.command_buffer.clear();
                        }
                        KeyCode::Esc => {
                            app.current_view = app_state::View::Detail;
                            app.command_buffer.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
