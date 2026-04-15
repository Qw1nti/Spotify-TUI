use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Section};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(40),
            Constraint::Length(34),
        ])
        .split(outer[0]);

    render_nav(frame, body[0], app);
    render_main(frame, body[1], app);
    render_detail(frame, body[2], app);
    render_footer(frame, outer[1], app);
}

fn render_nav(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let items = [
        Section::Home,
        Section::Search,
        Section::Library,
        Section::Playlists,
        Section::Devices,
    ]
    .into_iter()
    .map(|section| {
        let selected = section == app.section;
        let style = if selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        ListItem::new(Line::from(Span::styled(section.title(), style)))
    })
    .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title("spotifytui")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_main(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.current_items().is_empty() {
        let text = app
            .empty_state()
            .unwrap_or("Nothing to show yet.");
        let block = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::default().title(app.section_title()).borders(Borders::ALL));
        frame.render_widget(block, area);
        return;
    }

    let title = match app.section {
        Section::Search => match app.search_total {
            Some(total) if !app.search_query.is_empty() => {
                format!("{} [{}] - {}/{}", app.section_title(), app.search_query, app.search_results.len(), total)
            }
            _ => format!("{} [{}]", app.section_title(), app.search_query),
        },
        _ => app.section_title().to_string(),
    };

    let items = if app.section == Section::Devices {
        app.devices
            .iter()
            .enumerate()
            .map(|(idx, device)| {
                let selected = idx == app.selected;
                let preferred = app
                    .preferred_device_id
                    .as_deref()
                    .is_some_and(|id| device.id.as_deref() == Some(id));
                let active = device.is_active;
                let label = format!(
                    "{}{} {}{}",
                    if selected { ">" } else { " " },
                    if preferred { "*" } else { " " },
                    device.name,
                    device
                        .volume_percent
                        .map(|v| format!("  {}%", v))
                        .unwrap_or_default()
                );
                let style = if selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if preferred {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if active {
                    Style::default().fg(Color::LightGreen)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect::<Vec<_>>()
    } else {
        app
            .current_items()
            .into_iter()
            .enumerate()
            .map(|(idx, item)| {
                let style = if idx == app.selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(item, style)))
            })
            .collect::<Vec<_>>()
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let detail = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let mut lines = vec![];
    if let Some(user) = &app.user {
        lines.push(Line::from(vec![
            Span::styled("User: ", Style::default().fg(Color::Gray)),
            Span::raw(user.display_name.clone().unwrap_or_else(|| user.id.clone())),
        ]));
    }

    if let Some(playback) = &app.playback {
        if let Some(track) = &playback.item {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Now: ", Style::default().fg(Color::Gray)),
                Span::styled(&track.name, Style::default().add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(Color::Gray)),
                Span::raw(track.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Album: ", Style::default().fg(Color::Gray)),
                Span::raw(track.album.name.clone()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("State: ", Style::default().fg(Color::Gray)),
                Span::raw(if playback.is_playing { "playing" } else { "paused" }),
            ]));
        }
        if let Some(device) = &playback.device {
            lines.push(Line::from(vec![
                Span::styled("Playback: ", Style::default().fg(Color::Gray)),
                Span::raw(device.name.clone()),
            ]));
            if let Some(volume) = device.volume_percent {
                lines.push(Line::from(vec![
                    Span::styled("Volume: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{}%", volume)),
                ]));
            }
        }
    }

    if let Some(device) = app.devices.iter().find(|d| d.is_active) {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Device: ", Style::default().fg(Color::Gray)),
            Span::raw(device.name.clone()),
        ]));
        if app
            .preferred_device_id
            .as_deref()
            .is_some_and(|id| device.id.as_deref() == Some(id))
        {
            lines.push(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::Gray)),
                Span::raw("selected"),
            ]));
        }
        if let Some(volume) = device.volume_percent {
            lines.push(Line::from(vec![
                Span::styled("Device Volume: ", Style::default().fg(Color::Gray)),
                Span::raw(format!("{}%", volume)),
            ]));
        }
    }

    if let Some(id) = app.preferred_device_id.as_deref() {
        if let Some(device) = app.devices.iter().find(|d| d.id.as_deref() == Some(id)) {
            lines.push(Line::from(vec![
                Span::styled("Target Device: ", Style::default().fg(Color::Gray)),
                Span::raw(device.name.clone()),
            ]));
        }
    }

    let block = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().title("Context").borders(Borders::ALL));
    frame.render_widget(block, detail[0]);

    let mut progress = 0.0;
    let mut label = "No playback".to_string();
    if let Some(playback) = &app.playback {
        if let Some(track) = &playback.item {
            if track.duration_ms > 0 {
                progress = playback
                    .progress_ms
                    .unwrap_or(0) as f64
                    / track.duration_ms as f64;
                label = format!(
                    "{}%",
                    ((progress * 100.0).round() as i32).clamp(0, 100)
                );
            }
        }
    }

    let gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .ratio(progress.clamp(0.0, 1.0))
        .label(label);
    frame.render_widget(gauge, detail[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let left = format!("{} | {}", app.section_title(), app.status);
    let right = "q quit  tab switch  / search  enter search/play or pick device  a queue  o play  n/b next prev  space play/pause  r refresh  Esc keep input  F1 help";
    let text = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::Gray)),
        Span::raw("   "),
        Span::styled(right, Style::default().fg(Color::DarkGray)),
    ]);
    let block = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(block, area);
}
