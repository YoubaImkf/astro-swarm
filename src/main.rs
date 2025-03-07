use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{style::{Color, Style}, text::Span, widgets::{canvas::Canvas, Block, Borders}, DefaultTerminal, Frame};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut exited = false;

    while exited == false {
        terminal.draw(render)?;

        match event::read()? {
            Event::Key(key_event) => {
                if key_event.code == KeyCode::Char('q') {
                    exited = true;
                }
            }
            _ => {}
        }

    }
    Ok(())
}

fn  render(frame: &mut Frame) {
    let area = frame.area();

    let canvas = Canvas::default()
        .block(Block::default().title("My map").borders(Borders::ALL))
        .paint(|ctx| {
            ctx.layer();
            ctx.print(
                0.0,
                0.0,
                Span::styled("X", Style::default().fg(Color::Red))
            );
        });

    frame.render_widget(canvas, area);
}