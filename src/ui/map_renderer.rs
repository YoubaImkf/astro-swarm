use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;

use crate::{
    app::App, communication::channels::ResourceType, logging, map::noise::Map, robot::RobotState
};

pub fn render_app(frame: &mut Frame, area: Rect, app: &App) {

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(80), // map and sidebar
            Constraint::Percentage(20), // logs
        ])
        .split(area);

    let top_area = main_chunks[0];
    let log_area = main_chunks[1];

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(top_area);

    render_map_with_robots(frame, horizontal_chunks[0], app);
    render_sidebar_statistics(frame, horizontal_chunks[1], app);

    // Render the log widget
    let log_widget = logging::create_log_widget();
    frame.render_widget(log_widget, log_area);
}

/// Renders the map grid and overlays robot symbols based on their current state.
fn render_map_with_robots(frame: &mut Frame, area: Rect, app: &App) {
    let map_guard = app.map.read().expect("Map lock poisoned during render");

    let mut display_lines = create_styled_lines(&map_guard);
    drop(map_guard);


    overlay_robots(
        display_lines.as_mut_slice(),
        &app.scientific_robots,
        'S',
        Style::default().fg(Color::Gray),
    );
    overlay_robots(
        display_lines.as_mut_slice(),
        &app.collection_robots,
        'C',
        Style::default().fg(Color::White),
    );
    overlay_robots(
        display_lines.as_mut_slice(),
        &app.exploration_robots,
        'X',
        Style::default().fg(Color::Red),
    );

    let map_widget = create_map_widget(display_lines);
    frame.render_widget(map_widget, area);
}

fn overlay_robots(
    display_lines: &mut [Line<'_>],
    robots: &HashMap<u32, RobotState>,
    symbol: char,
    style: Style,
) {
    for robot_state in robots.values() {
        // Check Y
        if let Some(line) = display_lines.get_mut(robot_state.y) {
            // Check X
            if robot_state.x < line.width() {
                if robot_state.x < line.spans.len() {
                    line.spans[robot_state.x] = Span::styled(symbol.to_string(), style);
                } else {
                    log::warn!(
                        "Robot {} ({},{}) out of bounds for lne spans (len {})",
                        robot_state.id,
                        robot_state.x,
                        robot_state.y,
                        line.spans.len()
                    );
                }
            } else {
                log::trace!(
                    "Robot {} ({},{}) out of bounds for lne width ({})",
                    robot_state.id,
                    robot_state.x,
                    robot_state.y,
                    line.width()
                );
            }
        } else {
            log::warn!(
                "Robot {} ({},{}) out of bounds for display lines (len {})",
                robot_state.id,
                robot_state.x,
                robot_state.y,
                display_lines.len()
            );
        }
    }
}

fn render_sidebar_statistics(frame: &mut Frame, area: Rect, app: &App) {
    let mut items = Vec::new();

    items.push(ListItem::new(Line::from("--- Totals ---").bold()));

    items.push(ListItem::new("Collected Resources:"));

    let mut sorted_resources: Vec<_> = app.collected_resources.iter().collect();
    sorted_resources.sort_by_key(|(k, _)| format!("{:?}", k));

    if sorted_resources.is_empty() {
        items.push(ListItem::new(Line::from("  None yet").italic()));
    } else {
        for (resource_type, amount) in sorted_resources {
            let resource_name = match resource_type {
                ResourceType::Energy => "Energy",
                ResourceType::Minerals => "Minerals",
                ResourceType::SciencePoints => "Science Pts (Raw)",
            };
            items.push(ListItem::new(format!("  {}: {}", resource_name, amount)));
        }
    }

    items.push(ListItem::new(""));
    items.push(ListItem::new(format!(
        "Total Science Value: {}",
        app.scientific_data
    )));
    items.push(ListItem::new(format!(
        "Explored Tiles: {} / {}",
        app.total_explored,
        app.map_width * app.map_height
    )));
    items.push(ListItem::new(""));

    // --- Robots Section ---
    items.push(ListItem::new(Line::from("--- Robots ---").bold()));
    let exploration_count = app.exploration_robots.len();
    let collection_count = app.collection_robots.len();
    let scientific_count = app.scientific_robots.len();
    let total_robots = exploration_count + collection_count + scientific_count;

    items.push(ListItem::new(format!("Active: {}", total_robots)));

    items.push(ListItem::new(Line::from(vec![
        Span::raw("  Explorers : "),
        Span::styled(
            exploration_count.to_string(),
            Style::default().fg(Color::Red).bold(),
        ),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::raw("  Collectors: "),
        Span::styled(
            collection_count.to_string(),
            Style::default().fg(Color::Magenta).bold(),
        ),
    ])));
    items.push(ListItem::new(Line::from(vec![
        Span::raw("  Scientists: "),
        Span::styled(
            scientific_count.to_string(),
            Style::default().fg(Color::Cyan).bold(),
        ),
    ])));

    let stats_list =
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Statistics "));

    frame.render_widget(stats_list, area);
}

fn create_styled_lines(map: &Map) -> Vec<Line<'static>> {
    map.to_string().lines().map(create_styled_line).collect()
}

fn create_styled_line(line_str: &str) -> Line<'static> {
    line_str
        .chars()
        .map(create_styled_span)
        .collect::<Vec<_>>()
        .into()
}

fn create_styled_span(c: char) -> Span<'static> {
    let style = match c {
        '█' => Style::default().fg(Color::Gray),
        ' ' => Style::default().fg(Color::Rgb(50, 50, 50)),
        'E' => Style::default().fg(Color::Yellow),
        'M' => Style::default().fg(Color::Blue),
        'S' => Style::default().fg(Color::Green),
        '⌂' => Style::default().fg(Color::Indexed(208)),
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
