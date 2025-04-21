use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub struct TerminalManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalManager {
    pub fn new() -> io::Result<Self> {
        let terminal = Self::setup_terminal()?;
        Ok(Self { terminal })
    }

    fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Terminal::new(CrosstermBackend::new(stdout))
    }

    pub fn get_terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
