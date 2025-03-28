use astro_swarm::app::App;
use astro_swarm::terminal::TerminalManager;
use astro_swarm::ui::map_renderer::render_map;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::Backend, Terminal};
use std::io;

fn main() -> std::io::Result<()> {
    let app = App::new(110, 15, 34, 12345);
    let mut terminal_manager = TerminalManager::new()?;

    run_app(&app, terminal_manager.get_terminal())?;

    Ok(())
}

fn run_app<B: Backend>(app: &App, terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            render_map(frame, frame.area(), &app.map);
        })?;

        if should_quit()? {
            break;
        }
    }
    Ok(())
}

fn should_quit() -> io::Result<bool> {
    if let Event::Key(key) = event::read()? {
        return Ok(key.code == KeyCode::Char('q'));
    }
    Ok(false)
}