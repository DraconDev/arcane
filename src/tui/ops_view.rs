use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_ops(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    // --- Left Panel: Fleet (Groups + Servers) ---
    let mut fleet_targets = Vec::new();
    for g in &app.ops_groups {
        fleet_targets.push((format!("🌐 Group: {}", g.name), true));
    }
    for s in &app.ops_servers {
        fleet_targets.push((format!("🖥️  {}", s.name), false));
    }

    let fleet_items: Vec<ListItem> = fleet_targets
        .iter()
        .enumerate()
        .map(|(i, (name, is_group))| {
            let style = if i == app.ops_selected_server_idx {
                Style::default()
                    .fg(if *is_group {
                        Color::Yellow
                    } else {
                        Color::Cyan
                    })
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![Span::styled(name, style)]))
        })
        .collect();

    // Add "Add Server" option or similar if list is empty?
    // For now just list.

    let servers_block = Block::default()
        .borders(Borders::ALL)
        .title(" Fleet ")
        .border_style(Style::default().fg(Color::Cyan));

    let servers_list = List::new(fleet_items)
        .block(servers_block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(servers_list, left_area);

    // --- Right Panel: Containers / Action ---
    // If we have stats/containers loaded, show them

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(right_area);

    let content_area = right_chunks[0];
    let help_area = right_chunks[1];

    if app.ops_loading {
        let loading = Paragraph::new("⏳ Connecting to server...")
            .block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(loading, content_area);
    } else if app.ops_containers.is_empty() {
        let empty =
            Paragraph::new("No containers found or not connected.\nPress [Enter] to refresh.")
                .block(Block::default().borders(Borders::ALL).title(" Containers "));
        f.render_widget(empty, content_area);
    } else {
        // List Containers
        let items: Vec<ListItem> = app
            .ops_containers
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let color = if c.status.to_lowercase().contains("up") {
                    Color::Green
                } else {
                    Color::Red
                };

                let selected = i == app.ops_selected_container_idx;

                let style = if selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // format: ID | Image | Name | Status | Ports
                let content = format!(
                    " {:<12} {:<20} {:<20} {:<15} {}",
                    &c.id[..12.min(c.id.len())],
                    &c.image[..20.min(c.image.len())],
                    &c.name[..20.min(c.name.len())],
                    c.status,
                    c.ports
                );

                ListItem::new(Span::styled(content, Style::default().fg(color))).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Containers "))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD)); // Handled manually above

        f.render_widget(list, content_area);
    }

    // Help Footer for Ops
    let help_text = "[Enter]Refresh  [D]eploy  [L]ogs  [S]hell  [↑/↓]Nav";
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, help_area);
}
