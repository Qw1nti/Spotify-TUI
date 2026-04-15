use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Section};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(4)])
        .split(frame.area());

    render_top_bar(frame, outer[0], app);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(40),
            Constraint::Length(36),
        ])
        .split(outer[1]);

    render_nav(frame, body[0], app);
    render_main(frame, body[1], app);
    render_detail(frame, body[2], app);
    render_footer(frame, outer[2], app);
}

fn render_top_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(20), Constraint::Length(32)])
        .split(area);

    let brand = Paragraph::new(Line::from(vec![
        Span::styled(" spotifytui ", green_bold()),
        Span::styled(" local ", muted()),
    ]))
    .block(panel_block(""))
    .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(brand, chunks[0]);

    let now_playing = now_playing_line(app);
    let center = Paragraph::new(now_playing)
        .block(panel_block("Now Playing"))
        .wrap(Wrap { trim: true });
    frame.render_widget(center, chunks[1]);

    let right_text = vec![
        Line::from(vec![
            Span::styled("Section: ", muted()),
            Span::styled(app.section_title(), green_bold()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", muted()),
            Span::raw(app.status.as_str()),
        ]),
    ];
    let right = Paragraph::new(right_text)
        .block(panel_block("Session"))
        .wrap(Wrap { trim: true });
    frame.render_widget(right, chunks[2]);
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
        let label = if selected {
            format!("> {}", section.title())
        } else {
            format!("  {}", section.title())
        };
        let style = if selected {
            green_bold().bg(panel_bg())
        } else {
            muted()
        };
        ListItem::new(Line::from(Span::styled(label, style)))
    })
    .collect::<Vec<_>>();

    let list = List::new(items).block(panel_block("Library"));
    frame.render_widget(list, area);
}

fn render_main(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = match app.section {
        Section::Search => search_title(app),
        _ => app.section_title().to_string(),
    };
    let block = panel_block(&title);

    if app.current_items().is_empty() {
        let text = app.empty_state().unwrap_or("Nothing to show yet.");
        let help = vec![
            Line::from(vec![Span::styled("No content", green_bold())]),
            Line::from(""),
            Line::from(text),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tips: ", muted()),
                Span::raw("tab switch, / search, enter play, space pause, F1 help"),
            ]),
        ];
        let paragraph = Paragraph::new(help).wrap(Wrap { trim: true }).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let items = if app.section == Section::Devices {
        device_items(app)
    } else {
        standard_items(app)
    };

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let detail = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(4)])
        .split(area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("spotifytui", green_bold()),
            Span::styled("  ", muted()),
            Span::styled(app.section_title(), light_green()),
        ]),
        Line::from(""),
    ];

    if let Some(user) = &app.user {
        lines.push(Line::from(vec![
            Span::styled("User: ", muted()),
            Span::raw(user.display_name.clone().unwrap_or_else(|| user.id.clone())),
        ]));
    }

    if let Some(playback) = &app.playback {
        if let Some(track) = &playback.item {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Track: ", muted()),
                Span::styled(track.name.clone(), green_bold()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Artist: ", muted()),
                Span::raw(track.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Album: ", muted()),
                Span::raw(track.album.name.clone()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("State: ", muted()),
                Span::raw(if playback.is_playing { "playing" } else { "paused" }),
            ]));
        }

        if let Some(device) = &playback.device {
            lines.push(Line::from(vec![
                Span::styled("Device: ", muted()),
                Span::raw(device.name.clone()),
            ]));
            if let Some(volume) = device.volume_percent {
                lines.push(Line::from(vec![
                    Span::styled("Volume: ", muted()),
                    Span::raw(format!("{}%", volume)),
                ]));
            }
        }
    }

    if let Some(device) = active_device(app) {
        lines.push(Line::from(vec![
            Span::styled("Target: ", muted()),
            Span::raw(device.name.clone()),
        ]));
    }

    let context = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel_block("Context"));
    frame.render_widget(context, detail[0]);

    let progress = playback_progress(app);
    let gauge = Gauge::default()
        .block(panel_block("Progress"))
        .gauge_style(Style::default().fg(spotify_green()).bg(panel_bg()))
        .ratio(progress.0)
        .label(progress.1);
    frame.render_widget(gauge, detail[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = vec![
        Span::styled("q", green_bold()),
        Span::raw(" quit  "),
        Span::styled("tab", green_bold()),
        Span::raw(" switch  "),
        Span::styled("/", green_bold()),
        Span::raw(" search  "),
        Span::styled("enter", green_bold()),
        Span::raw(" play/select device  "),
        Span::styled("space", green_bold()),
        Span::raw(" pause/play  "),
        Span::styled("n/b", green_bold()),
        Span::raw(" next prev  "),
        Span::styled("r", green_bold()),
        Span::raw(" refresh  "),
        Span::styled("F1", green_bold()),
        Span::raw(" help"),
    ];
    let status = vec![
        Span::styled(app.section_title(), green_bold()),
        Span::raw(" | "),
        Span::styled(app.status.as_str(), Style::default().fg(Color::White)),
    ];
    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let status_bar = Paragraph::new(Line::from(status))
        .block(Block::default().borders(Borders::ALL).border_style(border_style()));
    frame.render_widget(status_bar, footer[0]);

    let help_bar = Paragraph::new(Line::from(help))
        .block(Block::default().borders(Borders::ALL).border_style(border_style()));
    frame.render_widget(help_bar, footer[1]);
}

fn search_title(app: &App) -> String {
    match app.search_total {
        Some(total) if !app.search_query.is_empty() => {
            format!("{} [{}] - {}/{}", app.section_title(), app.search_query, app.search_results.len(), total)
        }
        _ => format!("{} [{}]", app.section_title(), app.search_query),
    }
}

fn standard_items(app: &App) -> Vec<ListItem<'static>> {
    app.current_items()
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected = idx == app.selected;
            let style = if selected {
                green_bold().bg(panel_bg())
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if selected { "> " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{item}"), style)))
        })
        .collect()
}

fn device_items(app: &App) -> Vec<ListItem<'static>> {
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
            let prefix = match (selected, preferred, active) {
                (true, true, _) => ">*",
                (true, false, _) => "> ",
                (false, true, _) => " *",
                _ => "  ",
            };
            let suffix = device
                .volume_percent
                .map(|v| format!("  {}%", v))
                .unwrap_or_default();
            let label = format!("{prefix} {}{suffix}", device.name);
            let style = if selected {
                green_bold().bg(panel_bg())
            } else if preferred {
                green_bold()
            } else if active {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect()
}

fn playback_progress(app: &App) -> (f64, String) {
    if let Some(playback) = &app.playback {
        if let Some(track) = &playback.item {
            if track.duration_ms > 0 {
                let ratio = playback.progress_ms.unwrap_or(0) as f64 / track.duration_ms as f64;
                let label = format!("{}%", ((ratio * 100.0).round() as i32).clamp(0, 100));
                return (ratio.clamp(0.0, 1.0), label);
            }
        }
    }
    (0.0, "No playback".into())
}

fn now_playing_line(app: &App) -> Vec<Line<'static>> {
    if let Some(playback) = &app.playback {
        if let Some(track) = &playback.item {
            return vec![
                Line::from(vec![
                    Span::styled("Track: ", muted()),
                    Span::styled(track.name.clone(), green_bold()),
                ]),
                Line::from(vec![
                    Span::styled("Artist: ", muted()),
                    Span::raw(track.artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
                ]),
            ];
        }
    }

    vec![
        Line::from(vec![
            Span::styled("Nothing playing", green_bold()),
        ]),
        Line::from(vec![
            Span::styled("Open Spotify or press o on a track.", muted()),
        ]),
    ]
}

fn active_device(app: &App) -> Option<&crate::api::Device> {
    app.preferred_device_id
        .as_deref()
        .and_then(|id| app.devices.iter().find(|device| device.id.as_deref() == Some(id)))
        .or_else(|| app.devices.iter().find(|device| device.is_active))
}

fn panel_block(title: &str) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style())
        .style(Style::default().bg(panel_bg()));
    if title.is_empty() {
        block
    } else {
        block.title(Span::styled(title.to_string(), green_bold()))
    }
}

fn spotify_green() -> Color {
    Color::Rgb(29, 185, 84)
}

fn light_green() -> Style {
    Style::default().fg(Color::Rgb(130, 255, 170))
}

fn green_bold() -> Style {
    Style::default()
        .fg(spotify_green())
        .add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(Color::Rgb(170, 170, 170))
}

fn border_style() -> Style {
    Style::default().fg(Color::Rgb(52, 52, 52))
}

fn panel_bg() -> Color {
    Color::Rgb(18, 18, 18)
}
