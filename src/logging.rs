use log::LevelFilter;
use ratatui::style::{Color, Style, Stylize};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};
use color_eyre::Result; 

/// Initialize the logging system for the app
/// Set up tui-logger for displaying logs in UI.
pub fn setup_logging() -> Result<()> {
    tui_logger::init_logger(LevelFilter::Trace)?;

    tui_logger::set_default_level(LevelFilter::Info);

    log::info!("TUI Logger Initialized");
    Ok(())
}

/// Creates and configures the TuiLoggerWidget for rendering.
pub fn create_log_widget<'a>() -> TuiLoggerWidget<'a> {
    TuiLoggerWidget::default()
        .block(
            ratatui::widgets::Block::default()
                .title("Logs")
                .border_style(Style::default().fg(Color::White))
                .borders(ratatui::widgets::Borders::ALL),
        )
        .output_separator('|')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_target(false) 
        .output_file(false)
        .output_line(false)

        .style_error(Style::default().fg(Color::Red).bold())
        .style_debug(Style::default().fg(Color::Blue))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Gray))
        .style_info(Style::default().fg(Color::Green))
}