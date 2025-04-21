use astro_swarm::{app::App, logging, terminal::TerminalManager, ui::map_renderer::render_app};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::Backend;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(100);

fn main() -> Result<()> {
    setup()?;
    
    let mut app = App::new(90, 15, 34, 45);
    let mut terminal_manager = TerminalManager::new()?;
    
    run_app(&mut app, terminal_manager.get_terminal())?;
    
    log::info!("Application terminated");
    Ok(())
}

fn setup() -> Result<()> {
    color_eyre::install()?;
    logging::setup_logging()?;
    log::info!("Application starting...");
    Ok(())
}

fn run_app<B: Backend>(app: &mut App, terminal: &mut ratatui::Terminal<B>) -> Result<()> {
    let mut last_tick = Instant::now();
    
    loop {
        terminal.draw(|frame| render_app(frame, frame.area(), app))?;
        
        if check_events()? {
            break;
        }
        
        if last_tick.elapsed() >= TICK_RATE {
            app.update();
            last_tick = Instant::now();
        }
        
        if let Some(timeout) = TICK_RATE.checked_sub(last_tick.elapsed()) {
            std::thread::sleep(std::cmp::min(timeout, Duration::from_millis(10)));
        }
    }
    
    Ok(())
}

fn check_events() -> Result<bool> {
    if event::poll(Duration::from_millis(10))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}