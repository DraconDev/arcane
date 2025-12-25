use crate::ai_service::{AIConfig, AIProvider, AIService};
use crate::tui::app::App;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: App,
) -> Result<()> {
    loop {
        app.on_tick();
        terminal.draw(|f| crate::tui::ui::ui::<B>(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Check if provider menu is open
                if app.provider_menu_open {
                    handle_provider_menu(&mut app, key.code);
                    continue;
                }

                // Check if we're in editing mode
                if app.ai_config_editing {
                    handle_ai_config_editing(&mut app, key.code);
                    continue;
                }

                // Modal Handling (Smart Squash)
                if app.analyzing_squash {
                    // Ignore inputs while analyzing
                    continue;
                }

                if app.squash_plan.is_some() || app.squash_error.is_some() {
                    match key.code {
                        KeyCode::Enter => {
                            if app.squash_plan.is_some() {
                                app.execute_squash_plan();
                            } else {
                                app.cancel_squash(); // Dismiss error
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.cancel_squash();
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Char('s') => {
                        if app.current_tab == 1 {
                            // Graph: Smart Squash
                            app.trigger_squash_analysis();
                        } else if app.current_tab == 3 && app.identity_sub_tab == 3 {
                            // Identity/Security: Scan Repo
                            app.scan_repo();
                        } else {
                            // Default: Toggle Daemon
                            app.toggle_daemon();
                        }
                    }
                    KeyCode::Char('l') => {
                        if app.current_tab == 1 {
                            // Graph tab only: Bulk Squash
                            app.trigger_lazy_squash();
                        }
                    }
                    KeyCode::Char('b') => {
                        if app.current_tab == 1 {
                            // Graph: Cycle branch mode
                            app.graph_branch_mode = (app.graph_branch_mode + 1) % 3;
                        } else if app.current_tab == 0
                            || (app.current_tab == 2 && app.ai_config_sub_tab == 0)
                        {
                            // Dashboard or AI Overview: Toggle shadow branches
                            app.toggle_shadow_branches();
                        }
                    }
                    KeyCode::Char('m') => {
                        // Toggle Bulk Squash Minor mode (in Versioning tab)
                        if app.current_tab == 2 && app.ai_config_sub_tab == 3 {
                            app.config.bulk_squash_minor = !app.config.bulk_squash_minor;
                            let _ = app.config.save();
                            app.events.push(format!(
                                "⚙️  Bulk Squash: {} bump",
                                if app.config.bulk_squash_minor {
                                    "Minor"
                                } else {
                                    "Major"
                                }
                            ));
                        }
                    }
                    KeyCode::Char('D') => {
                        if app.current_tab == 5 {
                            let total_groups = app.ops_groups.len();
                            let total_servers = app.ops_servers.len();
                            let total_targets = total_groups + total_servers;

                            if total_targets == 0 {
                                app.events
                                    .push("❌ No servers or groups configured.".to_string());
                            } else if app.ops_selected_server_idx < total_groups {
                                // Target is a Group
                                let group_name =
                                    app.ops_groups[app.ops_selected_server_idx].name.clone();
                                app.events
                                    .push(format!("🚀 Deploying to group {}...", group_name));
                                app.trigger_deploy(group_name);
                            } else {
                                // Target is a Server
                                let server_idx = app.ops_selected_server_idx - total_groups;
                                let server_name = app.ops_servers[server_idx].name.clone();
                                app.events
                                    .push(format!("🚀 Deploying to server {}...", server_name));
                                app.trigger_deploy(server_name);
                            }
                        }
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        if app.current_tab == 2 && app.ai_config_sub_tab == 1 {
                            run_connectivity_test(&mut app);
                        }
                    }
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    // Left/Right: Navigate sub-tabs or main tabs
                    KeyCode::Right => {
                        if app.current_tab == 4 && app.sub_tab_focused {
                            // Identity (was 3)
                            app.identity_sub_tab = (app.identity_sub_tab + 1) % 5;
                        } else if app.current_tab == 2 && app.ai_config_focused {
                            // AI (was 4)
                            app.ai_config_sub_tab = (app.ai_config_sub_tab + 1) % 5;
                            app.ai_config_row = 0;
                        } else if app.current_tab == 3 && app.ai_config_focused {
                            // Repo (was 4/patterns)
                            app.ai_patterns_sub_tab = (app.ai_patterns_sub_tab + 1) % 2;
                            app.ai_config_row = 0;
                        } else {
                            app.next_tab();
                        }
                    }
                    KeyCode::Left => {
                        if app.current_tab == 4 && app.sub_tab_focused {
                            // Identity
                            if app.identity_sub_tab > 0 {
                                app.identity_sub_tab -= 1;
                            } else {
                                app.identity_sub_tab = 4;
                            }
                        } else if app.current_tab == 2 && app.ai_config_focused {
                            // AI
                            if app.ai_config_sub_tab > 0 {
                                app.ai_config_sub_tab -= 1;
                            } else {
                                app.ai_config_sub_tab = 4; // 5 tabs (0-4)
                            }
                            app.ai_config_row = 0;
                        } else if app.current_tab == 3 && app.ai_config_focused {
                            // Repo
                            if app.ai_patterns_sub_tab > 0 {
                                app.ai_patterns_sub_tab -= 1;
                            } else {
                                app.ai_patterns_sub_tab = 1;
                            }
                            app.ai_config_row = 0;
                        } else {
                            app.previous_tab();
                        }
                    }
                    // Up/Down: Navigate rows
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.current_tab == 2 && app.ai_config_focused {
                            // AI Tab
                            if app.ai_config_row > 0 {
                                app.ai_config_row -= 1;
                                // Skip separator row in Providers
                                if app.ai_config_sub_tab == 1 && app.ai_config_row == 3 {
                                    app.ai_config_row -= 1;
                                }
                            } else {
                                app.ai_config_focused = false; // Exit focus
                            }
                        } else if app.current_tab == 3 && app.ai_config_focused {
                            // Repo Tab
                            if app.ai_config_row > 0 {
                                app.ai_config_row -= 1;
                            } else {
                                app.ai_config_focused = false;
                            }
                        } else if app.current_tab == 4 && app.sub_tab_focused {
                            // Identity (was 3)
                            app.sub_tab_focused = false;
                        } else if app.current_tab == 5 {
                            if app.ops_selected_server_idx > 0 {
                                app.ops_selected_server_idx -= 1;
                                app.ops_selected_container_idx = 0;
                            }
                        } else {
                            app.scroll_up();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.current_tab == 2 && app.ai_config_focused {
                            // AI
                            // Navigate rows
                            let limit: usize = match app.ai_config_sub_tab {
                                1 => 9, // Providers
                                2 => 2, // Timing
                                3 => 1, // Versioning
                                _ => 0,
                            };
                            if app.ai_config_row < limit.saturating_sub(1) {
                                app.ai_config_row += 1;
                                // Skip separator in Providers
                                if app.ai_config_sub_tab == 1 && app.ai_config_row == 3 {
                                    app.ai_config_row += 1;
                                }
                            }
                        } else if app.current_tab == 3 && app.ai_config_focused {
                            // Repo
                            let limit: usize = match app.ai_patterns_sub_tab {
                                0 => app.ignore_patterns.len(),
                                1 => app.gitattributes_patterns.len(),
                                2 => 1, // Prompt
                                _ => 0,
                            };
                            if app.ai_config_row < limit.saturating_sub(1) {
                                app.ai_config_row += 1;
                            }
                        } else if app.current_tab == 4 && !app.sub_tab_focused {
                            // Identity
                            app.sub_tab_focused = true;
                        } else if app.current_tab == 2 && !app.ai_config_focused {
                            // Enter AI Focus
                            app.ai_config_focused = true;
                            app.ai_config_row = 0;
                        } else if app.current_tab == 3 && !app.ai_config_focused {
                            // Enter Repo Focus
                            app.ai_config_focused = true;
                            app.ai_config_row = 0;
                        } else if app.current_tab == 5 {
                            let total_targets = app.ops_groups.len() + app.ops_servers.len();
                            if app.ops_selected_server_idx < total_targets.saturating_sub(1) {
                                app.ops_selected_server_idx += 1;
                                app.ops_selected_container_idx = 0;
                            }
                        } else {
                            app.scroll_down();
                        }
                    }
                    KeyCode::Enter => {
                        if app.current_tab == 5 {
                            let total_groups = app.ops_groups.len();
                            if app.ops_selected_server_idx < total_groups {
                                app.events.push(
                                    "🌐 Group selected. Use 'D' to deploy to all.".to_string(),
                                );
                                app.ops_containers.clear();
                            } else {
                                let server_idx = app.ops_selected_server_idx - total_groups;
                                if !app.ops_servers.is_empty() {
                                    app.ops_loading = true;
                                    let server = app.ops_servers[server_idx].clone();
                                    match crate::ops::monitor::Monitor::list_containers(&server) {
                                        Ok(c) => {
                                            app.ops_containers = c;
                                            app.events.push("✅ Refreshed containers".to_string());
                                        }
                                        Err(e) => {
                                            app.events.push(format!("❌ Connection failed: {}", e))
                                        }
                                    }
                                    app.ops_loading = false;
                                }
                            }
                        } else if app.input_popup_active {
                            // Handle popup submission
                            let input = app.input_popup_buffer.clone();
                            let callback = app.input_popup_callback.clone();

                            app.input_popup_active = false;
                            app.input_popup_buffer.clear();

                            match callback.as_str() {
                                "team_add" => app.add_team_member(input),
                                "deploy_auth" => {
                                    app.machine_keys.push(input.trim().to_string());
                                    app.events.push("✅ Deploy key authorized".to_string());
                                }
                                "edit_ignore" => {
                                    if app.input_popup_index < app.ignore_patterns.len() {
                                        app.ignore_patterns[app.input_popup_index] = input.clone();
                                        app.save_ai_config();
                                        app.events
                                            .push(format!("✅ Updated .gitignore: {}", input));
                                    }
                                }
                                "edit_attr" => {
                                    if app.input_popup_index < app.gitattributes_patterns.len() {
                                        app.gitattributes_patterns[app.input_popup_index] =
                                            input.clone();
                                        app.save_ai_config();
                                        app.events
                                            .push(format!("✅ Updated .gitattributes: {}", input));
                                    }
                                }
                                "add_ignore" => {
                                    if !input.trim().is_empty() {
                                        app.ignore_patterns.push(input.trim().to_string());
                                        app.save_ai_config();
                                        app.events.push(format!("✅ Added .gitignore: {}", input));
                                    }
                                }
                                "add_attr" => {
                                    if !input.trim().is_empty() {
                                        app.gitattributes_patterns.push(input.trim().to_string());
                                        app.save_ai_config();
                                        app.events
                                            .push(format!("✅ Added .gitattributes: {}", input));
                                    }
                                }
                                "edit_prompt" => {
                                    app.system_prompt = input;
                                    app.save_ai_config();
                                    app.events.push("✅ Updated Commit Prompt".to_string());
                                }
                                _ => {}
                            }
                        } else if app.current_tab == 4
                            && app.ai_config_focused
                            && (app.ai_config_sub_tab == 1
                                || app.ai_config_sub_tab == 2
                                || app.ai_config_sub_tab == 3)
                        {
                            start_ai_config_edit(&mut app);
                        } else if app.current_tab == 4
                            && app.ai_config_focused
                            && app.ai_config_sub_tab == 4
                        {
                            match app.ai_patterns_sub_tab {
                                0 => {
                                    // Edit ignore
                                    if !app.ignore_patterns.is_empty() {
                                        app.input_popup_active = true;
                                        app.input_popup_title =
                                            "Edit .gitignore Pattern".to_string();
                                        app.input_popup_buffer =
                                            app.ignore_patterns[app.ai_config_row].clone();
                                        app.input_popup_callback = "edit_ignore".to_string();
                                        app.input_popup_index = app.ai_config_row;
                                    }
                                }
                                1 => {
                                    // Edit attr
                                    if !app.gitattributes_patterns.is_empty() {
                                        app.input_popup_active = true;
                                        app.input_popup_title =
                                            "Edit .gitattributes Pattern".to_string();
                                        app.input_popup_buffer =
                                            app.gitattributes_patterns[app.ai_config_row].clone();
                                        app.input_popup_callback = "edit_attr".to_string();
                                        app.input_popup_index = app.ai_config_row;
                                    }
                                }
                                2 => {
                                    // Edit prompt
                                    app.input_popup_active = true;
                                    app.input_popup_title = "Edit AI Commit Prompt".to_string();
                                    app.input_popup_buffer = app.system_prompt.clone();
                                    app.input_popup_callback = "edit_prompt".to_string();
                                }
                                _ => {}
                            }
                        } else if app.current_tab == 3 {
                            match app.identity_sub_tab {
                                1 => {
                                    app.input_popup_active = true;
                                    app.input_popup_title =
                                        "Add Team Member - Paste Public Key".to_string();
                                    app.input_popup_buffer.clear();
                                    app.input_popup_callback = "team_add".to_string();
                                }
                                2 => {
                                    app.input_popup_active = true;
                                    app.input_popup_title = "Authorize Deploy Key".to_string();
                                    app.input_popup_buffer.clear();
                                    app.input_popup_callback = "deploy_auth".to_string();
                                }
                                _ => {}
                            }
                        } else {
                            app.inspect_commit();
                        }
                    }
                    KeyCode::Char(c) if app.input_popup_active => {
                        app.input_popup_buffer.push(c);
                    }
                    KeyCode::Backspace if app.input_popup_active => {
                        app.input_popup_buffer.pop();
                    }
                    KeyCode::Char('g') if !app.input_popup_active => {
                        if app.current_tab == 3 && app.identity_sub_tab == 2 {
                            use arcane::security::ArcaneSecurity;
                            let (public, secret) = ArcaneSecurity::generate_machine_identity();
                            app.events.push(format!("🔑 Public: {}", public));
                            app.events.push(format!("🔐 Secret: {}", secret));
                            app.events
                                .push("⚠️  Save the secret key securely!".to_string());
                        }
                    }
                    KeyCode::Char('x') if !app.input_popup_active => {
                        if app.current_tab == 3 && app.ai_config_focused {
                            match app.ai_patterns_sub_tab {
                                0 => {
                                    if app.ai_config_row < app.ignore_patterns.len() {
                                        let removed = app.ignore_patterns.remove(app.ai_config_row);
                                        app.save_ai_config();
                                        app.events
                                            .push(format!("❌ Removed .gitignore: {}", removed));
                                    }
                                }
                                1 => {
                                    if app.ai_config_row < app.gitattributes_patterns.len() {
                                        let removed =
                                            app.gitattributes_patterns.remove(app.ai_config_row);
                                        app.save_ai_config();
                                        app.events.push(format!(
                                            "❌ Removed .gitattributes: {}",
                                            removed
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        } else if app.current_tab == 3 && app.identity_sub_tab == 1 {
                            if !app.team_members.is_empty()
                                && app.selected_team_idx < app.team_members.len()
                            {
                                app.remove_team_member(app.selected_team_idx);
                            }
                        }
                    }
                    KeyCode::Char('r')
                        if !app.input_popup_active && !app.restore_confirm_active =>
                    {
                        if app.current_tab == 1 {
                            // Git Graph: Restore to selected commit
                            if app.selected_row < app.git_log.lines.len() {
                                let line_content = app.git_log.lines[app.selected_row].to_string();
                                if let Ok(re) = regex::Regex::new(r"\b[0-9a-f]{7}\b") {
                                    if let Some(mat) = re.find(&line_content) {
                                        let hash = mat.as_str().to_string();
                                        app.pending_restore_hash = hash.clone();
                                        app.restore_confirm_active = true;
                                    }
                                }
                            }
                        } else if app.current_tab == 2 // AI Tab
                            && app.ai_config_focused
                            && app.ai_config_sub_tab == 4
                        // Prompt
                        {
                            app.reset_config_section("prompt");
                        } else if app.current_tab == 3 // Repo Tab
                            && app.ai_config_focused
                        {
                            let section = match app.ai_patterns_sub_tab {
                                0 => "gitignore",
                                1 => "gitattributes",
                                _ => "",
                            };
                            if !section.is_empty() {
                                app.reset_config_section(section);
                            }
                        } else if app.current_tab == 3 && app.identity_sub_tab == 3 {
                            use arcane::security::ArcaneSecurity;
                            match ArcaneSecurity::new(None) {
                                Ok(sec) => {
                                    app.events.push("🔄 Rotating keys...".to_string());
                                    match sec.rotate_repo_key(&[]) {
                                        Ok(_) => {
                                            app.events.push("✅ Keys rotated!".to_string());
                                            app.refresh_identity();
                                        }
                                        Err(e) => {
                                            app.events.push(format!("❌ Rotation failed: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    app.events.push(format!("❌ Init failed: {}", e));
                                }
                            }
                        } else {
                            app.refresh_identity();
                        }
                    }
                    KeyCode::Char('y') if app.restore_confirm_active => {
                        // Confirm restore
                        let hash = app.pending_restore_hash.clone();
                        app.restore_confirm_active = false;
                        app.pending_restore_hash.clear();

                        let result = std::process::Command::new("git")
                            .args(&["checkout", &hash])
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                app.events.push(format!("✅ Restored to commit {}", hash));
                            }
                            Ok(output) => {
                                let err = String::from_utf8_lossy(&output.stderr);
                                app.events.push(format!("❌ Restore failed: {}", err));
                            }
                            Err(e) => {
                                app.events.push(format!("❌ Git error: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Esc if app.restore_confirm_active => {
                        // Cancel restore
                        app.restore_confirm_active = false;
                        app.pending_restore_hash.clear();
                        app.events.push("⏸️ Restore cancelled".to_string());
                    }
                    KeyCode::Char('e') if !app.input_popup_active => {
                        if app.current_tab == 2
                            && app.ai_config_focused
                            && app.ai_config_sub_tab == 4
                        {
                            app.input_popup_active = true;
                            app.input_popup_title = "Edit AI Commit Prompt".to_string();
                            app.input_popup_buffer = app.system_prompt.clone();
                            app.input_popup_callback = "edit_prompt".to_string();
                        }
                    }
                    KeyCode::Char('a') if !app.input_popup_active => {
                        if app.current_tab == 3 // Repo Config
                             && app.ai_config_focused
                        {
                            match app.ai_patterns_sub_tab {
                                0 => {
                                    app.input_popup_active = true;
                                    app.input_popup_title = "Add .gitignore Pattern".to_string();
                                    app.input_popup_buffer.clear();
                                    app.input_popup_callback = "add_ignore".to_string();
                                }
                                1 => {
                                    app.input_popup_active = true;
                                    app.input_popup_title =
                                        "Add .gitattributes Pattern".to_string();
                                    app.input_popup_buffer.clear();
                                    app.input_popup_callback = "add_attr".to_string();
                                }
                                _ => {}
                            }
                        } else {
                            app.toggle_auto_commit()
                        }
                    }
                    KeyCode::Char('p') if !app.input_popup_active => {
                        app.toggle_auto_push();
                    }
                    KeyCode::Char('v') if !app.input_popup_active => {
                        app.toggle_version_bumping();
                    }
                    KeyCode::Char('i') if !app.input_popup_active => app.ignore_selected_file(),
                    KeyCode::Esc => {
                        if app.input_popup_active {
                            app.input_popup_active = false;
                            app.input_popup_buffer.clear();
                        } else {
                            app.close_popup();
                        }
                    }

                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn start_ai_config_edit(app: &mut App) {
    if app.ai_config_sub_tab == 1 {
        // Providers sub-tab
        match app.ai_config_row {
            0 | 1 | 2 => {
                // Provider chain selection (Slot Menu)
                app.provider_menu_open = true;
                app.provider_menu_idx = 0;
                app.provider_edit_target = match app.ai_config_row {
                    0 => "Primary",
                    1 => "Backup 1",
                    2 => "Backup 2",
                    _ => "",
                }
                .to_string();
            }
            4 | 5 | 6 | 7 | 8 => {
                // Per-provider config
                app.provider_menu_open = true;
                app.provider_menu_idx = 0;
                app.provider_edit_target = match app.ai_config_row {
                    4 => "Gemini",
                    5 => "OpenRouter",
                    6 => "OpenAI",
                    7 => "Anthropic",
                    8 => "Ollama",
                    _ => "",
                }
                .to_string();
            }
            _ => {}
        }
    } else if app.ai_config_sub_tab == 2 {
        // Timing sub-tab: 0=inactivity, 1=min commit
        match app.ai_config_row {
            0 => {
                app.ai_config_editing = true;
                app.ai_config_input = app.inactivity_delay.to_string();
            }
            1 => {
                app.ai_config_editing = true;
                app.ai_config_input = app.min_commit_delay.to_string();
            }
            _ => {}
        }
    } else if app.ai_config_sub_tab == 3 {
        // Versioning sub-tab - toggle version bumping
        if app.ai_config_row == 0 {
            app.version_bumping = !app.version_bumping;
            app.save_ai_config();
        }
    } else if app.ai_config_sub_tab == 4 {
        app.input_popup_active = true;
        app.input_popup_title = "Edit AI Commit Prompt".to_string();
        app.input_popup_buffer = app.system_prompt.clone();
        app.input_popup_callback = "edit_prompt".to_string();
    }
}

fn handle_provider_menu(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.provider_menu_open = false;
        }
        KeyCode::Up => {
            if app.provider_menu_idx > 0 {
                app.provider_menu_idx -= 1;
            }
        }
        KeyCode::Down => {
            if app.provider_menu_idx < 2 {
                app.provider_menu_idx += 1;
            }
        }
        KeyCode::Enter => {
            app.provider_menu_open = false;
            let is_slot_config = matches!(
                app.provider_edit_target.as_str(),
                "Primary" | "Backup 1" | "Backup 2"
            );

            match app.provider_menu_idx {
                0 => {
                    if is_slot_config {
                        // Action: Select Provider -> Dropdown
                        app.ai_config_editing = true;
                        app.provider_edit_target = "Selecting".to_string();
                        app.ai_config_dropdown_idx = 0;
                    } else {
                        // Action: Set API Key -> Input
                        app.ai_config_editing = true;
                        app.input_mode_key = true;
                        app.ai_config_input.clear();
                    }
                }
                1 => {
                    app.ai_config_editing = true;
                    app.input_mode_key = false;

                    if is_slot_config {
                        // Action: Set Slot Model -> Input
                        let current = match app.provider_edit_target.as_str() {
                            "Primary" => &app.primary_model,
                            "Backup 1" => &app.backup1_model,
                            "Backup 2" => &app.backup2_model,
                            _ => "",
                        }
                        .to_string();
                        app.ai_config_input = current;
                    } else {
                        // Action: Set Provider Default Model -> Input
                        let target = app.provider_edit_target.clone();
                        let current = app
                            .model_overrides
                            .get(&target)
                            .cloned()
                            .unwrap_or_default();
                        app.ai_config_input = current;
                    }
                }
                2 => {
                    if is_slot_config {
                        // Action: Reset Slot Model -> Msg/Clear
                        match app.provider_edit_target.as_str() {
                            "Primary" => app.primary_model.clear(),
                            "Backup 1" => app.backup1_model.clear(),
                            "Backup 2" => app.backup2_model.clear(),
                            _ => {}
                        }
                        app.save_ai_config();
                    } else {
                        // Action: Reset Provider Default -> Clear
                        app.model_overrides.remove(&app.provider_edit_target);
                        app.save_ai_config();
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_ai_config_editing(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.ai_config_editing = false;
            app.ai_config_input.clear();
        }
        KeyCode::Enter => {
            if app.ai_config_sub_tab == 1 {
                if app.ai_config_row < 3 {
                    if app.provider_edit_target == "Selecting" {
                        // Provider dropdown
                        let options = App::provider_options();
                        if app.ai_config_dropdown_idx < options.len() {
                            let selected = options[app.ai_config_dropdown_idx].to_string();
                            match app.ai_config_row {
                                0 => app.current_ai_provider = selected,
                                1 => app.backup_provider_1 = selected,
                                2 => app.backup_provider_2 = selected,
                                _ => {}
                            }
                            app.save_ai_config();
                        }
                    } else {
                        // Slot Model Override (Input)
                        if !app.ai_config_input.is_empty() {
                            match app.ai_config_row {
                                0 => app.primary_model = app.ai_config_input.clone(),
                                1 => app.backup1_model = app.ai_config_input.clone(),
                                2 => app.backup2_model = app.ai_config_input.clone(),
                                _ => {}
                            }
                            app.save_ai_config();
                        }
                    }
                } else {
                    // Text Input (Key or Model)
                    if app.input_mode_key {
                        // Save API key to config AND set env var for this session
                        let provider_name = app.provider_edit_target.clone();
                        let key_name = match provider_name.as_str() {
                            "Gemini" => "GEMINI_API_KEY",
                            "OpenRouter" => "OPENROUTER_API_KEY",
                            "OpenAI" => "OPENAI_API_KEY",
                            "Anthropic" => "ANTHROPIC_API_KEY",
                            _ => "",
                        };
                        if !key_name.is_empty() && !app.ai_config_input.is_empty() {
                            // Set env var for immediate use
                            std::env::set_var(key_name, &app.ai_config_input);

                            // Save to config file for persistence
                            if let Ok(mut config) = arcane::config::ArcaneConfig::load() {
                                config
                                    .api_keys
                                    .insert(provider_name.clone(), app.ai_config_input.clone());
                                if config.save().is_ok() {
                                    app.events
                                        .push(format!("✅ {} API key saved!", provider_name));
                                } else {
                                    app.events.push(format!(
                                        "⚠️ {} key set for session only",
                                        provider_name
                                    ));
                                }
                            }

                            app.api_key_status.insert(provider_name, true);
                        }
                    } else {
                        // Set Model Override
                        app.model_overrides.remove(&app.provider_edit_target);
                        if !app.ai_config_input.is_empty() {
                            app.model_overrides.insert(
                                app.provider_edit_target.clone(),
                                app.ai_config_input.clone(),
                            );
                        }
                        app.save_ai_config();
                    }
                }
            } else if app.ai_config_sub_tab == 2 {
                // Timing number input
                if let Ok(num) = app.ai_config_input.parse::<u32>() {
                    match app.ai_config_row {
                        0 => app.inactivity_delay = num,
                        1 => app.min_commit_delay = num,
                        _ => {}
                    }
                    app.save_ai_config();
                }
            }
            app.ai_config_editing = false;
            app.ai_config_input.clear();
        }
        KeyCode::Up => {
            if app.ai_config_sub_tab == 1 && app.ai_config_row < 3 && app.ai_config_dropdown_idx > 0
            {
                app.ai_config_dropdown_idx -= 1;
            }
        }
        KeyCode::Down => {
            if app.ai_config_sub_tab == 1 && app.ai_config_row < 3 {
                let max = App::provider_options().len() - 1;
                if app.ai_config_dropdown_idx < max {
                    app.ai_config_dropdown_idx += 1;
                }
            }
        }
        KeyCode::Char(c) => {
            if app.ai_config_sub_tab == 1 {
                // Providers: Allow text input if row >= 4 OR (row < 3 AND not selecting provider)
                let is_slot_text = app.ai_config_row < 3 && app.provider_edit_target != "Selecting";
                if app.ai_config_row >= 4 || is_slot_text {
                    app.ai_config_input.push(c);
                }
            } else if app.ai_config_sub_tab == 2 && c.is_ascii_digit() {
                // Number input
                app.ai_config_input.push(c);
            } else if app.current_tab == 4
                && app.ai_config_focused
                && app.ai_config_sub_tab == 3
                && c == 'c'
            {
                run_version_check(app);
            }
        }
        KeyCode::Backspace => {
            app.ai_config_input.pop();
        }
        _ => {}
    }
}

fn parse_provider(s: &str) -> AIProvider {
    match s {
        "Gemini" => AIProvider::Gemini,
        "OpenRouter" => AIProvider::OpenRouter,
        "OpenAI" => AIProvider::OpenAI,
        "Anthropic" => AIProvider::Anthropic,
        "Copilot" => AIProvider::Copilot,
        "Ollama" => AIProvider::Ollama,
        _ => AIProvider::Ollama,
    }
}

fn run_version_check(app: &mut App) {
    let tx = app.version_tx.clone();

    // Load API keys from config first, then env vars as fallback
    let config = arcane::config::ArcaneConfig::load().unwrap_or_default();
    let mut api_keys = std::collections::HashMap::new();

    let get_key = |provider: &str, env_var: &str| -> Option<String> {
        if let Some(key) = config.api_keys.get(provider) {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
        std::env::var(env_var).ok()
    };

    if let Some(k) = get_key("Gemini", "GEMINI_API_KEY") {
        api_keys.insert(AIProvider::Gemini, k);
    }
    if let Some(k) = get_key("OpenRouter", "OPENROUTER_API_KEY") {
        api_keys.insert(AIProvider::OpenRouter, k);
    }
    if let Some(k) = get_key("OpenAI", "OPENAI_API_KEY") {
        api_keys.insert(AIProvider::OpenAI, k);
    }
    if let Some(k) = get_key("Anthropic", "ANTHROPIC_API_KEY") {
        api_keys.insert(AIProvider::Anthropic, k);
    }
    if let Some(k) = get_key("Copilot", "COPILOT_API_KEY") {
        api_keys.insert(AIProvider::Copilot, k);
    }
    // Ollama has no key

    // Models
    let mut models = std::collections::HashMap::new();
    // Default config values + overrides
    // We can just grab what's in app (partially) or reconstruct.
    // Reconstructing form app state is complex because app state is split.
    // Easier to load from ConfigManager / ArcaneConfig again?
    // Or just use the model overrides in App.
    for (p, m) in &app.model_overrides {
        let provider = parse_provider(p);
        models.insert(provider, m.clone());
    }

    // Provider Chain
    let primary = parse_provider(&app.current_ai_provider);
    // Backups
    let backup1 = if app.backup_provider_1 != "None" {
        Some(parse_provider(&app.backup_provider_1))
    } else {
        None
    };
    let backup2 = if app.backup_provider_2 != "None" {
        Some(parse_provider(&app.backup_provider_2))
    } else {
        None
    };

    let mut backups = Vec::new();
    if let Some(b) = backup1 {
        backups.push(b);
    }
    if let Some(b) = backup2 {
        backups.push(b);
    }

    let config = AIConfig {
        primary_provider: primary,
        backup_providers: backups,
        provider_models: models,
        api_keys,
    };

    let ai_service = AIService::new(config);

    tokio::spawn(async move {
        // 1. Get Diff
        // We'll use git command directly for simplicity in this tasks context
        let diff_output = std::process::Command::new("git")
            .args(&["diff", "--staged"])
            .output();

        let diff = if let Ok(output) = diff_output {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::new()
        };

        // Fallback to unstaged if staged is empty?
        let final_diff = if diff.trim().is_empty() {
            let unstaged = std::process::Command::new("git")
                .args(&["diff"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            unstaged
        } else {
            diff
        };

        if final_diff.trim().is_empty() {
            // Nothing to analyze
            let _ = tx.send(arcane::version_manager::SemVerBump::None);
            return;
        }

        // 2. Analyze
        if let Ok(bump) = ai_service.analyze_semver(&final_diff).await {
            let _ = tx.send(bump);
        } else {
            let _ = tx.send(arcane::version_manager::SemVerBump::None);
        }
    });
}

fn run_connectivity_test(app: &mut App) {
    app.testing_connectivity = true;
    app.connectivity_map.clear();
    let tx = app.connectivity_tx.clone();

    // Load API keys from config first, then env vars as fallback
    let config = arcane::config::ArcaneConfig::load().unwrap_or_default();
    let mut api_keys = std::collections::HashMap::new();

    // Helper to get key from config or env
    let get_key = |provider: &str, env_var: &str| -> Option<String> {
        if let Some(key) = config.api_keys.get(provider) {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
        std::env::var(env_var).ok()
    };

    if let Some(k) = get_key("Gemini", "GEMINI_API_KEY") {
        api_keys.insert(AIProvider::Gemini, k);
    }
    if let Some(k) = get_key("OpenRouter", "OPENROUTER_API_KEY") {
        api_keys.insert(AIProvider::OpenRouter, k);
    }
    if let Some(k) = get_key("OpenAI", "OPENAI_API_KEY") {
        api_keys.insert(AIProvider::OpenAI, k);
    }
    if let Some(k) = get_key("Anthropic", "ANTHROPIC_API_KEY") {
        api_keys.insert(AIProvider::Anthropic, k);
    }
    if let Some(k) = get_key("Copilot", "COPILOT_API_KEY") {
        api_keys.insert(AIProvider::Copilot, k);
    }

    // Models for slots
    let specs = vec![
        (
            "Primary".to_string(),
            app.current_ai_provider.clone(),
            app.primary_model.clone(),
        ),
        (
            "Backup 1".to_string(),
            app.backup_provider_1.clone(),
            app.backup1_model.clone(),
        ),
        (
            "Backup 2".to_string(),
            app.backup_provider_2.clone(),
            app.backup2_model.clone(),
        ),
    ];

    // Build Minimal Config
    let config = AIConfig {
        primary_provider: parse_provider(&specs[0].1),
        backup_providers: vec![],
        provider_models: std::collections::HashMap::new(),
        api_keys,
    };

    tokio::spawn(async move {
        let service = AIService::new(config);

        for (slot, provider_str, model_str) in specs {
            if provider_str == "None" || provider_str == "Auto" || provider_str.is_empty() {
                // Send dummy result to unblock UI
                let _ = tx.send((
                    slot,
                    crate::ai_service::AIAttempt {
                        provider: AIProvider::Ollama, // Dummy
                        model: None,
                        duration: std::time::Duration::from_millis(0),
                        success: false,
                        message: Some("Not configured".to_string()),
                        error: None,
                    },
                ));
                continue;
            }
            let provider = parse_provider(&provider_str);
            let model = if model_str.is_empty() {
                None
            } else {
                Some(model_str)
            };

            let result = service.check_connectivity(provider, model).await;
            let _ = tx.send((slot, result));
        }
    });
}
