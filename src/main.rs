//! whichdoc: A cargo documentation diagnostics-driven editor.
#![allow(clippy::multiple_crate_versions)]
use edtui::EditorEventHandler;
use ratatui::crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use whichdoc::{app_state, config, edit_plan, input, ui};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cfg = config::Config::load();

    let coords = if args.len() > 2 && args[1] == "--load-docs" {
        let file_content = std::fs::read_to_string(&args[2])?;
        let plan: edit_plan::EditPlan = serde_json::from_str(&file_content)?;
        let coords = input::read_cargo_diagnostics()?;
        let mut state = app_state::AppState::new(coords, cfg.max_width);
        state.load_docs(plan);
        return run_tui(state, &cfg);
    } else {
        input::read_cargo_diagnostics()?
    };

    if coords.is_empty() {
        eprintln!("No missing_docs diagnostics found");
        return Ok(());
    }

    let state = app_state::AppState::new(coords, cfg.max_width);
    run_tui(state, &cfg)
}

fn run_tui(mut app: app_state::AppState, cfg: &config::Config) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor_handler = EditorEventHandler::default();

    let result = run_app(&mut terminal, &mut app, cfg, &mut editor_handler);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    } else {
        let plan = app.generate_edit_plan();
        let json = serde_json::to_string_pretty(&plan)?;
        println!("{json}");
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut app_state::AppState,
    cfg: &config::Config,
    editor_handler: &mut EditorEventHandler,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app, cfg))?;

        if let Event::Key(key) = event::read()? {
            match app.current_view {
                app_state::View::List => match key.code {
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
                },
                app_state::View::Detail => {
                    match key.code {
                        KeyCode::Char(':') => {
                            app.current_view = app_state::View::Command;
                            app.command_buffer.clear();
                            app.message = None;
                        }
                        KeyCode::Esc => {
                            // Let edtui handle Esc for mode switching, only exit if in Normal mode
                            if let Some(ref editor_state) = app.editor_state {
                                if editor_state.mode == edtui::EditorMode::Normal {
                                    if app.entries[app.list_index].dirty {
                                        app.message = Some(
                                            "Unsaved changes! Use :q to discard or :x to save"
                                                .to_string(),
                                        );
                                    } else {
                                        app.exit_detail_view(false);
                                    }
                                } else {
                                    editor_handler
                                        .on_key_event(key, app.editor_state.as_mut().unwrap());
                                }
                            }
                        }
                        _ => {
                            if let Some(ref mut editor_state) = app.editor_state {
                                editor_handler.on_key_event(key, editor_state);
                                app.entries[app.list_index].dirty = true;
                            }
                        }
                    }
                }
                app_state::View::Command => match key.code {
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
                                if let Err(e) = app.save_current() {
                                    app.message = Some(format!("Error saving: {e}"));
                                }
                            }
                            "x" => {
                                if let Err(e) = app.save_current() {
                                    app.message = Some(format!("Error saving: {e}"));
                                } else {
                                    app.exit_detail_view(true);
                                }
                            }
                            "q" => {
                                if app.entries[app.list_index].dirty {
                                    app.message =
                                        Some("Unsaved changes! Use :q! to force quit".to_string());
                                    app.current_view = app_state::View::Detail;
                                } else {
                                    app.exit_detail_view(false);
                                }
                            }
                            "q!" => {
                                app.exit_detail_view(false);
                            }
                            "wn" => {
                                if let Err(e) = app.save_current() {
                                    app.message = Some(format!("Error saving: {e}"));
                                } else if let Some(next) = app.find_next_undocumented() {
                                    app.exit_detail_view(true);
                                    app.list_index = next;
                                    app.enter_detail_view();
                                } else {
                                    app.message = Some("No more undocumented items".to_string());
                                }
                            }
                            "wp" => {
                                if let Err(e) = app.save_current() {
                                    app.message = Some(format!("Error saving: {e}"));
                                } else if let Some(prev) = app.find_prev_undocumented() {
                                    app.exit_detail_view(true);
                                    app.list_index = prev;
                                    app.enter_detail_view();
                                } else {
                                    app.message =
                                        Some("No previous undocumented items".to_string());
                                }
                            }
                            _ => {
                                app.message = Some(format!("Unknown command: {cmd}"));
                            }
                        }
                        app.command_buffer.clear();
                    }
                    KeyCode::Esc => {
                        app.current_view = app_state::View::Detail;
                        app.command_buffer.clear();
                    }
                    _ => {}
                },
            }
        }
    }
}
