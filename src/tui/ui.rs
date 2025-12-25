use crate::tui::app::App;
use crate::tui::ops_view::render_ops;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn ui<B: Backend>(f: &mut Frame, app: &mut App) {
    // Dynamic layout based on current tab
    let show_status_hub = app.current_tab == 0;

    let chunks = if show_status_hub {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Tabs
                    Constraint::Length(3), // Status Hub (Dashboard only)
                    Constraint::Min(0),    // Main Content
                    Constraint::Length(3), // Footer/Help
                ]
                .as_ref(),
            )
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Tabs
                    Constraint::Min(0),    // Main Content (more space!)
                    Constraint::Length(3), // Footer/Help
                ]
                .as_ref(),
            )
            .split(f.area())
    };

    // 1. Tabs
    let titles = app
        .tabs
        .iter()
        .map(|t| Line::from(t.as_str()))
        .collect::<Vec<_>>();

    let views_focused = !app.ai_config_focused && !app.sub_tab_focused;
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Views ")
                .border_style(if views_focused {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                }),
        )
        .select(app.current_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    // 2. Status Hub (Dashboard only)
    let (main_area, footer_area) = if show_status_hub {
        let status_block = Block::default().borders(Borders::ALL).title(" Status Hub ");
        let status_lines = if let Some(status) = &app.status {
            let watched_path = app
                .watch_roots
                .first()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "None".to_string());

            let pid_span = Span::styled(
                format!(
                    " Daemon: RUNNING (PID: {}) | State: {} | Watched: {} ",
                    status.pid, status.state, watched_path
                ),
                Style::default().fg(Color::Green),
            );

            vec![Line::from(pid_span)]
        } else {
            vec![Line::from(Span::styled(
                " Daemon: STOPPED ",
                Style::default().fg(Color::Red),
            ))]
        };

        let p = Paragraph::new(status_lines).block(status_block);
        f.render_widget(p, chunks[1]);
        (chunks[2], chunks[3])
    } else {
        (chunks[1], chunks[2])
    };

    // 3. Main Content
    match app.current_tab {
        0 => render_dashboard(f, app, main_area),
        1 => render_graph(f, app, main_area),
        2 => render_ai(f, app, main_area),         // New AI Tab
        3 => render_repository(f, app, main_area), // New Repo Tab
        4 => render_identity(f, app, main_area),
        5 => render_ops(f, app, main_area),
        _ => {}
    }

    // 4. Footer
    let help = Paragraph::new(format!(
        "Tab: Switch View | 's': Daemon | Enter: Inspect | Scrl: {}/{} | Sel: {}",
        app.scroll,
        app.git_log.lines.len(),
        app.selected_row
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, footer_area);

    if app.show_popup {
        let area = centered_rect(80, 80, f.area());
        f.render_widget(ratatui::widgets::Clear, area); // Clear background
        let popup_block = Block::default()
            .title(" Commit Details ")
            .borders(Borders::ALL);
        let popup_text = Paragraph::new(app.popup_content.clone())
            .block(popup_block)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((app.popup_scroll, 0));
        f.render_widget(popup_text, area);
    }

    // Input Popup (for Team Add, Deploy Auth, etc.)
    if app.input_popup_active {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(ratatui::widgets::Clear, area);

        let title = format!(" {} ", app.input_popup_title);
        let popup_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let input_text = format!(
            "\n  > {}_\n\n  (Enter to submit, Esc to cancel)",
            app.input_popup_buffer
        );

        let popup_para = Paragraph::new(input_text)
            .block(popup_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(popup_para, area);
    }

    // Restore Confirmation Popup
    if app.restore_confirm_active {
        let area = centered_rect(50, 25, f.area());
        f.render_widget(ratatui::widgets::Clear, area);

        let popup_block = Block::default()
            .title(" ⚠️  Restore Confirmation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let confirm_text = format!(
            "\n  Restore to commit {}?\n\n  This will move HEAD to this commit.\n  Uncommitted changes may be lost.\n\n  [y] Yes, restore   [n/Esc] Cancel",
            app.pending_restore_hash
        );

        let popup_para = Paragraph::new(confirm_text)
            .block(popup_block)
            .style(Style::default().fg(Color::Yellow));

        f.render_widget(popup_para, area);
    }
}

// Helper for centering popup
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn render_dashboard(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(8), // Security/Alerts
                Constraint::Min(0),    // Working Tree
                Constraint::Length(3), // Controls (New)
            ]
            .as_ref(),
        )
        .split(area);

    // Security/Alerts (Top)
    let events_block = Block::default()
        .borders(Borders::ALL)
        .title(" Security & Alerts ");

    let events_text = if app.events.is_empty() {
        "System secure. No alerts.".to_string()
    } else {
        app.events.join("\n")
    };

    let events = Paragraph::new(events_text).block(events_block);
    f.render_widget(events, chunks[0]);

    // Working Tree (Middle)
    let work_block = Block::default()
        .borders(Borders::ALL)
        .title(" Working Tree (i: Ignore) ");

    let items: Vec<ListItem> = app
        .working_tree
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = match file.status {
                crate::tui::app::ChangeType::Untracked => Style::default().fg(Color::Red),
                crate::tui::app::ChangeType::Modified => Style::default().fg(Color::Yellow),
                crate::tui::app::ChangeType::Staged => Style::default().fg(Color::Green),
                _ => Style::default(),
            };

            let prefix = match file.status {
                crate::tui::app::ChangeType::Untracked => "??",
                crate::tui::app::ChangeType::Modified => " M",
                crate::tui::app::ChangeType::Staged => "M ",
                _ => "  ",
            };

            let line = format!("{} {}", prefix, file.path);
            let mut span = ratatui::text::Span::styled(line, style);

            if i == app.selected_file_idx {
                span.style = span.style.add_modifier(Modifier::REVERSED);
            }

            ListItem::new(span)
        })
        .collect();

    let list = List::new(items).block(work_block);
    f.render_widget(list, chunks[1]);

    // Controls (Bottom)
    let controls_block = Block::default()
        .borders(Borders::ALL)
        .title(" Dashboard Controls ");

    let separator = Span::raw("   ");

    // Daemon Button
    let daemon_running = app.status.is_some();
    let daemon_btn = if daemon_running {
        Span::styled(
            " [S] Stop Daemon ",
            Style::default()
                .bg(Color::Red)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [S] Start Daemon ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    };

    // Auto-Commit Button
    let auto_commit_btn = if app.ai_auto_commit {
        Span::styled(
            " [A] Auto-Commit: ON ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [A] Auto-Commit: OFF ",
            Style::default().fg(Color::DarkGray),
        )
    };

    // Auto-Push Button

    let auto_push_btn = Span::styled(
        if app.ai_auto_push {
            " [P] Auto-Push: ON "
        } else {
            " [P] Auto-Push: OFF "
        },
        Style::default().fg(if app.ai_auto_push {
            Color::Green
        } else {
            Color::Gray
        }),
    );

    let version_btn = Span::styled(
        if app.version_bumping {
            " [V] Auto-Version: ON "
        } else {
            " [V] Auto-Version: OFF "
        },
        Style::default().fg(if app.version_bumping {
            Color::Yellow
        } else {
            Color::Gray
        }),
    );

    let deploy_btn = Span::styled(
        if app.ai_auto_deploy {
            " [D] Auto-Deploy: ON "
        } else {
            " [D] Auto-Deploy: OFF "
        },
        Style::default().fg(if app.ai_auto_deploy {
            Color::Magenta
        } else {
            Color::Gray
        }),
    );

    let shadow_btn = Span::styled(
        if app.shadow_branches {
            " [B] Shadow Branches: ON "
        } else {
            " [B] Shadow Branches: OFF "
        },
        Style::default().fg(if app.shadow_branches {
            Color::Magenta
        } else {
            Color::Gray
        }),
    );

    let controls_line = Line::from(vec![
        daemon_btn,
        separator.clone(),
        auto_commit_btn,
        separator.clone(),
        auto_push_btn,
        separator.clone(),
        version_btn,
        separator.clone(),
        deploy_btn,
        separator,
        shadow_btn,
    ]);

    let controls = Paragraph::new(controls_line)
        .block(controls_block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(controls, chunks[2]);
}

fn render_graph(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let graph_block = Block::default()
        .borders(Borders::ALL)
        .title(" Full Git Graph ");
    let graph_text = if app.git_log.lines.is_empty() {
        ratatui::text::Text::raw("Loading graph...")
    } else {
        // Capture viewport height for smart scrolling
        app.vp_height = area.height;

        // Apply selection style
        let mut text = app.git_log.clone();
        if app.selected_row < text.lines.len() {
            // Apply selection style to the line wrapper
            text.lines[app.selected_row].style = Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Yellow);
            // Force spans to reverse too
            for span in &mut text.lines[app.selected_row].spans {
                span.style = span.style.add_modifier(Modifier::REVERSED);
            }
        }
        text
    };
    let graph = Paragraph::new(graph_text)
        .block(graph_block)
        .scroll((app.scroll, 0));
    f.render_widget(graph, area);
}

fn render_identity(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Sub-tab bar
                Constraint::Min(0),    // Content
            ]
            .as_ref(),
        )
        .split(area);

    // Sub-tab bar
    let sub_tabs = vec!["My ID", "Team", "Deploy", "Security", "Snaps"];
    let sub_tab_titles: Vec<Line> = sub_tabs.iter().map(|t| Line::from(*t)).collect();

    let sub_tab_title = if app.sub_tab_focused {
        " Vault Sub-Views (←/→ navigate, ↑ exit) "
    } else {
        " Vault Sub-Views (↓ to enter) "
    };

    let sub_tab_widget = Tabs::new(sub_tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(sub_tab_title)
                .border_style(if app.sub_tab_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                }),
        )
        .select(app.identity_sub_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(sub_tab_widget, chunks[0]);

    // Content based on sub-tab
    let content_area = chunks[1];
    match app.identity_sub_tab {
        0 => render_my_identity(f, app, content_area),
        1 => render_team_access(f, app, content_area),
        2 => render_deploy_keys(f, app, content_area),
        3 => render_security_ops(f, app, content_area),
        4 => render_snapshots(f, app, content_area),
        _ => {}
    }
}

fn render_my_identity(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" My Sovereign Identity ")
        .border_style(Style::default().fg(Color::Cyan));

    let key_display = app
        .master_pubkey
        .clone()
        .unwrap_or_else(|| "No Master Identity Found (run 'arcane init')".to_string());

    let content = Paragraph::new(format!("\n  Public Key:\n  {}", key_display))
        .block(block)
        .style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(content, area);
}

fn render_team_access(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Team Access (Enter: Add, x: Remove) ");

    let items: Vec<ListItem> = if app.team_members.is_empty() {
        vec![
            ListItem::new("  No team members. You are the only one with access.")
                .style(Style::default().fg(Color::DarkGray)),
        ]
    } else {
        app.team_members
            .iter()
            .enumerate()
            .map(|(i, member)| {
                let style = if i == app.selected_team_idx {
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  • {}", member)).style(style)
            })
            .collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_deploy_keys(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Deploy Keys (g: Generate, Enter: Authorize) ");

    let items: Vec<ListItem> = if app.machine_keys.is_empty() {
        vec![ListItem::new("  No machine keys authorized.")
            .style(Style::default().fg(Color::DarkGray))]
    } else {
        app.machine_keys
            .iter()
            .map(|key| ListItem::new(format!("  🔑 {}", key)))
            .collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_security_ops(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
        .split(area);

    // Controls
    let controls = Paragraph::new("\n  [s] Scan Repo for Secrets    [r] Rotate Keys")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Security Operations "),
        )
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(controls, chunks[0]);

    // Scan Results
    let results_block = Block::default()
        .borders(Borders::ALL)
        .title(" Scan Results ");

    let result_items: Vec<ListItem> = if app.scan_results.is_empty() {
        vec![ListItem::new("  ✅ No secrets detected (or scan not run).")
            .style(Style::default().fg(Color::Green))]
    } else {
        app.scan_results
            .iter()
            .map(|(file, secrets)| {
                ListItem::new(format!("  ⚠️ {} → {:?}", file, secrets))
                    .style(Style::default().fg(Color::Red))
            })
            .collect()
    };

    let results_list = List::new(result_items).block(results_block);
    f.render_widget(results_list, chunks[1]);
}

fn render_snapshots(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Shadow Backups ");

    let items: Vec<ListItem> = if app.snapshots.is_empty() {
        vec![ListItem::new("  No shadow backups yet.").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.snapshots
            .iter()
            .map(|(name, size)| ListItem::new(format!("  📦 {} ({} bytes)", name, size)))
            .collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_ai(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Sub-tab bar
                Constraint::Min(0),    // Content
            ]
            .as_ref(),
        )
        .split(area);

    // Sub-tab bar
    let sub_tabs = vec!["Overview", "Providers", "Timing", "Versioning", "Prompt"];
    let sub_tab_titles: Vec<Line> = sub_tabs.iter().map(|t| Line::from(*t)).collect();

    let sub_tab_title = if app.ai_config_focused {
        " AI Configuration (←/→ navigate, ↑ exit) "
    } else {
        " AI Configuration (↓ to enter) "
    };

    let sub_tab_widget = Tabs::new(sub_tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(sub_tab_title)
                .border_style(if app.ai_config_focused && app.ai_config_focus_level == 0 {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                }),
        )
        .select(app.ai_config_sub_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(sub_tab_widget, chunks[0]);

    match app.ai_config_sub_tab {
        0 => render_ai_overview(f, app, chunks[1]),
        1 => render_ai_providers(f, app, chunks[1]),
        2 => render_ai_timing(f, app, chunks[1]),
        3 => render_ai_versioning(f, app, chunks[1]),
        4 => render_ai_prompt(f, app, chunks[1]),
        _ => {}
    }
}

fn render_ai_overview(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)].as_ref())
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI Configuration Summary ");

    let version_icon = if app.version_bumping { "✅" } else { "❌" };

    // Build watch roots display
    let watch_roots_display = if app.watch_roots.is_empty() {
        "None configured".to_string()
    } else {
        app.watch_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let summary = format!(
        "\n  Provider Chain: {} → {} → {}\n\n  Timing: {}s inactivity, {}s min commit\n\n  Version Bumping: {} {}\n\n  Ignore Patterns: {} | Gitattributes: {}\n\n  Watch Roots: {}",
        app.current_ai_provider,
        app.backup_provider_1,
        app.backup_provider_2,
        app.inactivity_delay,
        app.min_commit_delay,
        version_icon,
        if app.version_bumping { "Enabled" } else { "Disabled" },
        app.ignore_patterns.len(),
        app.gitattributes_patterns.len(),
        watch_roots_display
    );

    let para = Paragraph::new(summary).block(block).style(Style::default());
    f.render_widget(para, chunks[0]);

    // Config path + hint
    let config_block = Block::default().borders(Borders::ALL).title(" Config ");
    let config_para =
        Paragraph::new("  ~/.arcane/config.toml\n  Edit [daemon.watch_roots] to add directories")
            .block(config_block)
            .style(Style::default().fg(Color::DarkGray));
    f.render_widget(config_para, chunks[1]);
}

fn render_ai_providers(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Provider Configuration (Enter: Edit, 't': Test) ");

    // Helper to get API key status icon
    let key_icon = |provider: &str| -> &str {
        if app.api_key_status.get(provider).copied().unwrap_or(false) {
            "✓ KEY"
        } else {
            "✗ NO KEY"
        }
    };

    // Helper to get model for provider
    let get_model = |provider: &str| -> String {
        // Check model overrides first, then default
        for (p, m) in &app.model_overrides {
            if p == provider {
                return m.clone();
            }
        }
        match provider {
            "Gemini" => "gemini-2.0-flash-lite".to_string(),
            "OpenRouter" => "qwen/qwen3-coder:free".to_string(),
            "OpenAI" => "gpt-4o-mini".to_string(),
            "Anthropic" => "claude-3-5-sonnet".to_string(),
            "Ollama" => "qwen2.5:7b".to_string(),
            _ => "(default)".to_string(),
        }
    };

    // Display helpers
    let fmt_slot = |provider: &str, model: &str| -> String {
        if provider == "None" {
            "None".to_string()
        } else if !model.is_empty() {
            format!("{} (Model: {})", provider, model)
        } else {
            format!("{}", provider)
        }
    };

    // Connectivity status helper
    let get_status = |slot: &str| -> String {
        if let Some(res_opt) = app.connectivity_map.get(slot) {
            if let Some(res) = res_opt {
                if res.success {
                    format!(" ✅ {}ms", res.duration.as_millis())
                } else {
                    format!(" ❌ Err") // Keep short
                }
            } else {
                " ⏳ Testing...".to_string()
            }
        } else if app.testing_connectivity {
            " ⏳ Queued...".to_string()
        } else {
            "".to_string()
        }
    };

    let settings = vec![
        // Row 0-2: Provider chain selection (Slot Logic)
        format!(
            "  ⭐ Primary:    {}{}",
            fmt_slot(&app.current_ai_provider, &app.primary_model),
            get_status("Primary")
        ),
        format!(
            "  🔄 Backup 1:   {}{}",
            fmt_slot(&app.backup_provider_1, &app.backup1_model),
            get_status("Backup 1")
        ),
        format!(
            "  🔄 Backup 2:   {}{}",
            fmt_slot(&app.backup_provider_2, &app.backup2_model),
            get_status("Backup 2")
        ),
        // Row 3: Separator
        format!("  ─────────────────────────────────────"),
        // Row 4-8: Per-provider status
        format!(
            "  Gemini       {}   Model: {}",
            key_icon("Gemini"),
            get_model("Gemini")
        ),
        format!(
            "  OpenRouter   {}   Model: {}",
            key_icon("OpenRouter"),
            get_model("OpenRouter")
        ),
        format!(
            "  OpenAI       {}   Model: {}",
            key_icon("OpenAI"),
            get_model("OpenAI")
        ),
        format!(
            "  Anthropic    {}   Model: {}",
            key_icon("Anthropic"),
            get_model("Anthropic")
        ),
        format!(
            "  Ollama       {}   Model: {}",
            key_icon("Ollama"),
            get_model("Ollama")
        ),
    ];

    let items: Vec<ListItem> = settings
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == app.ai_config_row && app.ai_config_focused {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Magenta)
            } else if i == 3 {
                // Separator row
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(s.as_str()).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);

    // If editing, show overlay
    if app.provider_menu_open {
        render_provider_menu(f, app, area);
    } else if app.ai_config_editing {
        if app.ai_config_row < 3 {
            if app.provider_edit_target == "Selecting" {
                render_provider_dropdown(f, app, area);
            } else {
                render_text_input(f, app, area);
            }
        } else if app.ai_config_row >= 4 {
            render_text_input(f, app, area);
        }
    }
}

fn render_provider_menu(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let popup_area = centered_rect(40, 25, area);
    f.render_widget(Clear, popup_area);

    let title = format!(" Configure {} ", app.provider_edit_target);

    let is_slot_config = matches!(
        app.provider_edit_target.as_str(),
        "Primary" | "Backup 1" | "Backup 2"
    );

    let options = if is_slot_config {
        vec![
            "📡 Select Provider",
            "🤖 Set Model (Slot)",
            "🔄 Reset Model",
        ]
    } else {
        vec!["🔑 Set API Key", "🤖 Set Default Model", "🔄 Reset Default"]
    };

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == app.provider_menu_idx {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!("  {}", opt)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(list, popup_area);
}

fn render_text_input(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let title = if app.input_mode_key {
        format!(" Set API Key for {} ", app.provider_edit_target)
    } else {
        format!(" Set Model for {} ", app.provider_edit_target)
    };

    let display_text = if app.input_mode_key {
        "*".repeat(app.ai_config_input.len())
    } else {
        app.ai_config_input.clone()
    };

    let content = format!("\n  > {}_", display_text);
    let para = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(para, popup_area);
}

fn render_provider_dropdown(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let popup_area = centered_rect(40, 50, area);
    f.render_widget(Clear, popup_area);

    let options = App::provider_options();
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == app.ai_config_dropdown_idx {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!("  {}", opt)).style(style)
        })
        .collect();

    let title = match app.ai_config_row {
        0 => " Select Primary Provider ",
        1 => " Select Backup 1 ",
        2 => " Select Backup 2 ",
        _ => " Select Provider ",
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(list, popup_area);
}

fn render_ai_timing(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Commit Timing (Enter to edit) ");

    let settings = vec![
        format!("  ⏱️  Inactivity Delay:   {} seconds", app.inactivity_delay),
        format!("  ⏳ Min Commit Delay:    {} seconds", app.min_commit_delay),
        format!(""),
        format!("  How it works:"),
        format!("    • Inactivity: Wait after last file change before commit"),
        format!("    • Min Delay: Minimum time between auto-commits"),
    ];

    let items: Vec<ListItem> = settings
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i < 2
                && app.ai_config_sub_tab == 2
                && app.ai_config_focused
                && app.ai_config_row == i
            {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Magenta)
            } else if i >= 2 {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(s.as_str()).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);

    // Show input popup if editing timing
    if app.ai_config_editing && app.ai_config_sub_tab == 2 && app.ai_config_row < 2 {
        render_timing_input(f, app, area);
    }
}

fn render_timing_input(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let popup_area = centered_rect(40, 20, area);
    f.render_widget(Clear, popup_area);

    let title = if app.ai_config_row == 0 {
        " Inactivity Delay (seconds) "
    } else {
        " Min Commit Delay (seconds) "
    };

    let content = format!("\n  > {}_", app.ai_config_input);
    let para = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(para, popup_area);
}

fn render_ai_versioning(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Smart Versioning (Enter: Toggle) ");

    // Detect version (Inline for now, consider moving to App state if slow)
    let repo_root = std::env::current_dir().unwrap_or_default();
    let (ver_file, ver_num) = if let Some(path) =
        arcane::version_manager::VersionManager::detect_version_file(&repo_root)
    {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let version = arcane::version_manager::VersionManager::get_current_version(&path)
            .unwrap_or("Unknown".to_string());
        (file_name, version)
    } else {
        ("None".to_string(), "N/A".to_string())
    };

    let settings = vec![
        format!(
            "  📦 Auto Version Bump:  {}",
            if app.version_bumping {
                "✅ ENABLED"
            } else {
                "❌ DISABLED"
            }
        ),
        format!(""),
        format!("  📂 Detected File:   {}", ver_file),
        format!("  🏷️  Current Version: {}", ver_num),
        format!(""),
        format!("  AI Strategy (if enabled):"),
        format!("    • Patch: Bug fixes, refactors"),
        format!("    • Minor: New features"),
        format!("    • Major: Breaking changes"),
        format!(""),
        format!("  Manual Override:"),
        format!("    • Press 'c' to check/simulate bump"),
    ];

    let items: Vec<ListItem> = settings
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == 0
                && app.ai_config_focused
                && app.ai_config_sub_tab == 3
                && app.ai_config_row == 0
            {
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Magenta)
            } else if i > 0 {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(s.as_str()).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_ai_patterns(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    match app.ai_patterns_sub_tab {
        0 => render_gitignore_patterns(f, app, area),
        1 => render_gitattributes_patterns(f, app, area),
        2 => render_ai_prompt(f, app, area),
        _ => {}
    }
}

fn render_gitignore_patterns(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let is_focused = app.ai_config_focused && app.ai_config_focus_level == 2;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" .gitignore Patterns (Press 'a' to add, 'Enter' to edit, 'x' to remove, 'r' to reset) ")
        .border_style(if is_focused {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = if app.ignore_patterns.is_empty() {
        vec![ListItem::new("  (none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.ignore_patterns
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let mut style = Style::default();
                if is_focused && app.ai_config_row == i {
                    style = style.add_modifier(Modifier::REVERSED).fg(Color::Magenta);
                }
                ListItem::new(format!("  \u{2022} {}", p)).style(style)
            })
            .collect()
    };
    f.render_widget(List::new(items).block(block), area);
}

fn render_gitattributes_patterns(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let is_focused = app.ai_config_focused && app.ai_config_focus_level == 2;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" .gitattributes Patterns (Press 'a' to add, 'Enter' to edit, 'x' to remove, 'r' to reset) ")
        .border_style(if is_focused {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = if app.gitattributes_patterns.is_empty() {
        vec![ListItem::new("  (none)").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.gitattributes_patterns
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let mut style = Style::default();
                if is_focused && app.ai_config_row == i {
                    style = style.add_modifier(Modifier::REVERSED).fg(Color::Magenta);
                }
                ListItem::new(format!("  \u{2022} {}", p)).style(style)
            })
            .collect()
    };
    f.render_widget(List::new(items).block(block), area);
}

fn render_ai_prompt(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let is_focused = app.ai_config_focused && app.ai_config_focus_level == 2;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI Commit Prompt (Press 'e' to edit, 'r' to reset) ")
        .border_style(if is_focused {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default()
        });

    let prompt = &app.system_prompt;
    let paragraph = Paragraph::new(prompt.clone())
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_repository(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Sub-tab bar
                Constraint::Min(0),    // Content
            ]
            .as_ref(),
        )
        .split(area);

    // Sub-tab bar
    let sub_tabs = vec![".gitignore", ".gitattributes"];

    let sub_tab_titles: Vec<Line> = sub_tabs.iter().map(|t| Line::from(*t)).collect();

    let mode_str = match app.pattern_mode {
        arcane::config::PatternMode::Append => "APPEND",
        arcane::config::PatternMode::Override => "OVERRIDE",
    };

    let sub_tab_title = if app.ai_config_focused {
        format!(" Repository Config (Mode: {} [m], ←/→/↑) ", mode_str)
    } else {
        format!(" Repository Config (Mode: {} [m], ↓ enter) ", mode_str)
    };

    let sub_tab_widget = Tabs::new(sub_tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(sub_tab_title)
                .border_style(if app.ai_config_focused {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                }),
        )
        .select(app.ai_patterns_sub_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(sub_tab_widget, chunks[0]);

    // Use existing render_ai_patterns function
    render_ai_patterns(f, app, chunks[1]);
}
