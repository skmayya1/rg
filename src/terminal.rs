//! Terminal lifecycle and keyboard event handling.

use std::{io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{app::App, ui::draw};

pub(crate) fn run(app: &mut App) -> Result<()> {
    let mut stdout = io::stdout();
    let _terminal_guard =
        TerminalGuard::enter(&mut stdout).context("could not initialize terminal")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("could not create terminal")?;
    run_event_loop(&mut terminal, app).context("terminal event loop failed")
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        else {
            continue;
        };
        match (code, modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => return Ok(()),
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => app.select_previous(),
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => app.select_next(),
            (KeyCode::Enter, _) => app.copy_selected_file(),
            (KeyCode::Char('y'), KeyModifiers::NONE) => app.copy_selected_location(),
            (KeyCode::Backspace, _) => app.delete_last_character(),
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => app.delete_previous_word(),
            (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                app.append_query(character);
            }
            _ => {}
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut io::Stdout) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
    }
}
