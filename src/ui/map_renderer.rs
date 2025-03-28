use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::map::noise::Map;


pub fn render_map(frame: &mut Frame, area: Rect, map: &Map) {
    let lines = create_styled_lines(map);
    let paragraph = create_map_widget(lines);
    frame.render_widget(paragraph, area);
}

fn create_styled_lines(map: &Map) -> Vec<Line<'static>> {
    map.to_string()
        .lines()
        .map(|line| create_styled_line(line))
        .collect()
}

fn create_styled_line(line: &str) -> Line<'static> {
    let spans: Vec<Span> = line
        .chars()
        .map(|c| create_styled_span(c))
        .collect();
    Line::from(spans)
}

fn create_styled_span(c: char) -> Span<'static> {
    let style = match c {
        '#' => Style::default().fg(Color::Gray),
        'E' => Style::default().fg(Color::Yellow),
        'M' => Style::default().fg(Color::Blue),
        'S' => Style::default().fg(Color::Green),
        _ => Style::default().fg(Color::White),
    };
    Span::styled(c.to_string(), style)
}

fn create_map_widget(lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Astro Swarm Map"),
    )
}