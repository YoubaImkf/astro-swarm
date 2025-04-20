use astro_swarm::app::App;
use astro_swarm::terminal::TerminalManager;
use astro_swarm::ui::map_renderer::render_app;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::Backend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> std::io::Result<()> {
    let mut app = App::new(90, 15, 34, 45);
    let mut terminal_manager = TerminalManager::new()?;

    run_app(&mut app, terminal_manager.get_terminal())?;

    Ok(())
}

fn run_app<B: Backend>(app: &mut App, terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut last_update = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        // Update app state at regular intervals
        let now = Instant::now();
        if now.duration_since(last_update) >= tick_rate {
            app.update();
            last_update = now;
        }

        terminal.draw(|frame| {
            render_app(frame, frame.area(), &app);
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
