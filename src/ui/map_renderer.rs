use ratatui::{
    layout::{Rect, Layout, Direction, Constraint},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Frame,
};

use crate::map::noise::Map;
use crate::app::App;

pub fn render_app(frame: &mut Frame, area: Rect, app: &App) {
    // Split the screen into main map and sidebar
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(area);
    
    // Render the map in the main area
    render_map_with_robots(frame, chunks[0], app);
    
    // Render the sidebar with stats
    render_sidebar_statistics(frame, chunks[1], app);
}

pub fn render_map_with_robots(frame: &mut Frame, area: Rect, app: &App) {
    let map_guard = app.map.read().unwrap();
    
    // Create a mutable copy of the map display
    let mut display_lines = create_styled_lines(&map_guard);
    
    // Add exploration robots (represented as 'X')
    for robot in &app.exploration_robots {
        if robot.y < display_lines.len() {
            let line = &mut display_lines[robot.y];
            let mut spans = line.spans.clone();
            
            if robot.x < spans.len() {
                spans[robot.x] = Span::styled("X", Style::default().fg(Color::Red));
                *line = Line::from(spans);
            }
        }
    }
    
    // Add collection robots (represented as 'C')
    for robot in &app.collection_robots {
        if robot.y < display_lines.len() {
            let line = &mut display_lines[robot.y];
            let mut spans = line.spans.clone();
            
            if robot.x < spans.len() {
                spans[robot.x] = Span::styled("C", Style::default().fg(Color::Magenta));
                *line = Line::from(spans);
            }
        }
    }

    // Add scientific robots (represented as 'S')
    for robot in &app.scientific_robots {
        if robot.y < display_lines.len() {
            let line = &mut display_lines[robot.y];
            let mut spans = line.spans.clone();
            
            if robot.x < spans.len() {
                spans[robot.x] = Span::styled("S", Style::default().fg(Color::Cyan));
                *line = Line::from(spans);
            }
        }
    }
    
    let paragraph = create_map_widget(display_lines);
    frame.render_widget(paragraph, area);
}

fn render_sidebar_statistics(frame: &mut Frame, area: Rect, app: &App) {
    // Create resource stats
    let mut items = Vec::new();
    
    items.push(ListItem::new("Resources Collected:"));
    for (resource_type, amount) in &app.collected_resources {
        let text = format!("  {}: {}", match resource_type {
            crate::communication::channels::ResourceType::Energy => "Energy",
            crate::communication::channels::ResourceType::Minerals => "Minerals",
            crate::communication::channels::ResourceType::SciencePoints => "Science",
        }, amount);
        items.push(ListItem::new(text));
    }
    
    items.push(ListItem::new(""));
    items.push(ListItem::new(format!("Scientific Data: {}", app.scientific_data)));
    items.push(ListItem::new(""));
    items.push(ListItem::new(format!("Areas Explored: {}", app.total_explored)));
    items.push(ListItem::new(""));
    items.push(ListItem::new(format!("Robots Active: {}", 
        app.exploration_robots.len() + app.collection_robots.len())));
    items.push(ListItem::new(format!("  Explorers: {}", app.exploration_robots.len())));
    items.push(ListItem::new(format!("  Collectors: {}", app.collection_robots.len())));
    items.push(ListItem::new(format!("  Scientists: {}", app.scientific_robots.len())));
    
    let stats_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Statistics"));
    
    frame.render_widget(stats_list, area);
}

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