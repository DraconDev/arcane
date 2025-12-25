use ansi_to_tui::IntoText;
use arcane::DaemonStatus;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ChangeType {
    Modified,
    Untracked,
    Staged,
    Other,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: ChangeType,
}

#[derive(Debug, Clone)]
pub struct CommitStats {
    pub files: String,
    pub insertions: String,
    pub deletions: String,
}

pub struct App {
    pub should_quit: bool,
    pub status: Option<DaemonStatus>,
    pub last_tick: std::time::Instant,
    pub git_log: Text<'static>,
    pub events: Vec<String>,
    pub tabs: Vec<String>,
    pub current_tab: usize,
    pub scroll: u16,
    pub selected_row: usize,
    pub vp_height: u16,
    pub show_popup: bool,
    pub popup_scroll: u16,
    pub popup_content: Text<'static>,
    pub working_tree: Vec<FileStatus>,
    pub selected_file_idx: usize,
    pub commit_details: Text<'static>,
    pub commit_stats: HashMap<String, CommitStats>,
    pub ai_auto_commit: bool,
    // Vault/Identity State
    pub identity_sub_tab: usize,
    pub master_pubkey: Option<String>,
    pub team_members: Vec<String>,
    pub machine_keys: Vec<String>,
    pub scan_results: Vec<(String, Vec<String>)>,
    pub snapshots: Vec<(String, u64)>,
    pub selected_team_idx: usize,
    pub sub_tab_focused: bool, // True = focus on sub-tabs, False = focus on main tabs
    // AI Config State
    pub ai_config_sub_tab: usize,      // Sub-view selection
    pub ai_patterns_sub_tab: usize,    // Patterns sub-sub-view selection
    pub ai_config_focused: bool,       // Focus on Settings area
    pub ai_config_focus_level: usize,  // 0=2nd level, 1=3rd level, 2=Items
    pub ai_config_row: usize,          // Selected settings row
    pub ai_config_editing: bool,       // Currently editing?
    pub ai_config_input: String,       // Text input buffer
    pub ai_config_dropdown_idx: usize, // Dropdown selection
    pub provider_menu_open: bool,      // Is provider action menu open?
    pub provider_menu_idx: usize,      // Menu selection index
    pub provider_edit_target: String,  // Which provider we're editing
    pub input_mode_key: bool,          // True if inputting API key (masked)
    pub current_ai_provider: String,
    pub primary_model: String,
    pub backup_provider_1: String,
    pub backup1_model: String,
    pub backup_provider_2: String,
    pub backup2_model: String,
    pub inactivity_delay: u32,
    pub min_commit_delay: u32,
    pub version_bumping: bool,
    pub watch_roots: Vec<PathBuf>,
    pub ignore_patterns: Vec<String>,
    pub gitattributes_patterns: Vec<String>,
    pub system_prompt: String,
    pub model_overrides: HashMap<String, String>,
    pub api_key_status: std::collections::HashMap<String, bool>, // Provider -> has key
    pub connectivity_map: std::collections::HashMap<String, Option<crate::ai_service::AIAttempt>>, // Slot -> Result
    pub testing_connectivity: bool,
    pub connectivity_tx: std::sync::mpsc::Sender<(String, crate::ai_service::AIAttempt)>,
    pub connectivity_rx: std::sync::mpsc::Receiver<(String, crate::ai_service::AIAttempt)>,
    pub version_tx: std::sync::mpsc::Sender<arcane::version_manager::SemVerBump>,
    pub version_rx: std::sync::mpsc::Receiver<arcane::version_manager::SemVerBump>,
    pub confirmed_bump: Option<arcane::version_manager::SemVerBump>,
    // Input Popup State
    pub input_popup_active: bool,
    pub input_popup_title: String,
    pub input_popup_buffer: String,
    pub input_popup_callback: String, // "team_add", "deploy_gen", "deploy_auth", "edit_ignore", "edit_attr", "restore_commit"
    pub input_popup_index: usize,
    // Restore Confirmation
    pub restore_confirm_active: bool,
    pub pending_restore_hash: String,

    // Ops State
    pub ops_servers: Vec<crate::ops::config::ServerConfig>,
    pub ops_selected_server_idx: usize,
    pub ops_containers: Vec<crate::ops::monitor::ContainerInfo>,
    pub ops_selected_container_idx: usize,
    pub ops_stats: Vec<crate::ops::monitor::ContainerStats>,
    pub ops_loading: bool,
    pub ops_action_menu_open: bool,
    pub ops_action_idx: usize,
}

impl App {
    pub fn new() -> Self {
        // Load config for AI settings
        let config = arcane::config::ArcaneConfig::load().unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        let (v_tx, v_rx) = std::sync::mpsc::channel();

        let ops_config = crate::ops::config::OpsConfig::load();

        let mut app = App {
            should_quit: false,
            status: None,
            last_tick: std::time::Instant::now(),
            git_log: Text::raw("Loading log..."),
            events: vec![],
            tabs: vec![
                "Dashboard".to_string(),
                "Graph".to_string(),
                "Intelligence".to_string(),
                "Identity".to_string(),
                "Settings".to_string(),
                "Ops".to_string(),
            ],
            current_tab: 0,
            scroll: 0,
            selected_row: 0,
            vp_height: 0,
            show_popup: false,
            popup_scroll: 0,
            popup_content: Text::default(),
            working_tree: vec![],
            selected_file_idx: 0,
            commit_details: Text::default(),
            commit_stats: HashMap::new(),
            ai_auto_commit: false,
            identity_sub_tab: 0,
            master_pubkey: None,
            team_members: vec![],
            machine_keys: vec![],
            scan_results: vec![],
            snapshots: vec![],
            selected_team_idx: 0,
            sub_tab_focused: false,
            ai_config_sub_tab: 0,
            ai_patterns_sub_tab: 0,
            ai_config_focused: false,
            ai_config_focus_level: 0,
            ai_config_row: 0,
            ai_config_editing: false,
            ai_config_input: String::new(),
            ai_config_dropdown_idx: 0,
            provider_menu_open: false,
            provider_menu_idx: 0,
            provider_edit_target: String::new(),
            input_mode_key: false,
            current_ai_provider: config
                .ai_provider
                .as_ref()
                .map(|p| format!("{:?}", p))
                .unwrap_or_else(|| "None".to_string()),
            primary_model: config.primary_model.clone().unwrap_or_default(),
            backup_provider_1: config
                .backup_provider_1
                .as_ref()
                .map(|p| format!("{:?}", p))
                .unwrap_or_else(|| "None".to_string()),
            backup1_model: config.backup1_model.clone().unwrap_or_default(),
            backup_provider_2: config
                .backup_provider_2
                .as_ref()
                .map(|p| format!("{:?}", p))
                .unwrap_or_else(|| "None".to_string()),
            backup2_model: config.backup2_model.clone().unwrap_or_default(),
            inactivity_delay: config.timing.inactivity_delay,
            min_commit_delay: config.timing.min_commit_delay,
            version_bumping: config.version_bumping,
            watch_roots: config.daemon.watch_roots.clone(),
            ignore_patterns: config.ignore_patterns.clone(),
            gitattributes_patterns: config.gitattributes_patterns.clone(),
            system_prompt: config.system_prompt.clone(),
            model_overrides: config.model_overrides.clone(),
            api_key_status: {
                let mut status = std::collections::HashMap::new();
                let has_key = |provider: &str, env_var: &str| -> bool {
                    if let Some(key) = config.api_keys.get(provider) {
                        if !key.is_empty() {
                            return true;
                        }
                    }
                    std::env::var(env_var).is_ok()
                };
                status.insert("Gemini".to_string(), has_key("Gemini", "GEMINI_API_KEY"));
                status.insert(
                    "OpenRouter".to_string(),
                    has_key("OpenRouter", "OPENROUTER_API_KEY"),
                );
                status.insert("OpenAI".to_string(), has_key("OpenAI", "OPENAI_API_KEY"));
                status.insert(
                    "Anthropic".to_string(),
                    has_key("Anthropic", "ANTHROPIC_API_KEY"),
                );
                status.insert("Ollama".to_string(), true);
                status
            },
            connectivity_map: std::collections::HashMap::new(),
            testing_connectivity: false,
            connectivity_tx: tx,
            connectivity_rx: rx,
            version_tx: v_tx,
            version_rx: v_rx,
            confirmed_bump: None,
            input_popup_active: false,
            input_popup_title: String::new(),
            input_popup_buffer: String::new(),
            input_popup_callback: String::new(),
            input_popup_index: 0,
            restore_confirm_active: false,
            pending_restore_hash: String::new(),

            // Ops Init
            ops_servers: ops_config.servers,
            ops_selected_server_idx: 0,
            ops_containers: vec![],
            ops_selected_container_idx: 0,
            ops_stats: vec![],
            ops_loading: false,
            ops_action_menu_open: false,
            ops_action_idx: 0,
        };
        app.refresh_identity();
        app
    }

    pub fn on_tick(&mut self) {
        // Poll Connectivity Results
        if self.testing_connectivity {
            while let Ok((slot, result)) = self.connectivity_rx.try_recv() {
                self.connectivity_map.insert(slot, Some(result));
            }
            // Check if done: Primary, Backup 1, Backup 2
            let all_done = self.connectivity_map.contains_key("Primary")
                && self.connectivity_map.contains_key("Backup 1")
                && self.connectivity_map.contains_key("Backup 2");

            if all_done {
                self.testing_connectivity = false;
            }
        }

        // Poll Version Check Results
        while let Ok(bump) = self.version_rx.try_recv() {
            self.confirmed_bump = Some(bump);
        }

        // Poll status every 1 second
        if self.last_tick.elapsed().as_secs() >= 1 {
            self.status = DaemonStatus::load();
            self.last_tick = std::time::Instant::now();

            // Refresh Git Graph
            // Refresh Git Graph
            // Refresh Git Graph
            let git_cmd = std::process::Command::new("git")
                .args(&[
                    "log",
                    "--graph",
                    "--format=%C(auto)%h%d %s %C(white)%C(bold)%cr %C(cyan)<%an>%C(reset)",
                    "--all",
                    "--color=always", // Force color for ANSI parsing
                    "-n",
                    "100",
                ])
                .output();

            match git_cmd {
                Ok(output) if output.status.success() => {
                    // Parse ANSI to Ratatui Text
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Beautify ASCII to Unicode Box Drawing
                    // * -> ● (Big Dot)
                    // | -> │ (Vertical)
                    // / -> ╱ (Diagonal)
                    // \ -> ╲ (Back Diagonal)
                    // _ -> ─ (Horizontal)
                    let beautified = stdout
                        .replace('*', "●")
                        .replace('|', "│")
                        .replace('/', "╱")
                        .replace('\\', "╲")
                        .replace('_', "─");

                    if let Ok(text) = beautified.into_text() {
                        self.git_log = text;
                    } else {
                        self.git_log = Text::raw("Failed to parse git log ANSI");
                    }
                }
                Ok(_) => {
                    self.git_log = Text::raw("No commits yet (Empty repository)");
                }
                Err(_) => {
                    self.git_log = Text::raw("Git command failed (Is this a git repo?)");
                }
            }

            // Fetch Commit Stats (Inline Magnitude)
            let stats_cmd = std::process::Command::new("git")
                .args(&["log", "--shortstat", "--format=%h", "-n", "100"])
                .output();

            if let Ok(output) = stats_cmd {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut current_hash = String::new();

                for line in stdout.lines() {
                    if !line.starts_with(' ') && !line.is_empty() {
                        current_hash = line.trim().to_string();
                    } else if line.starts_with(' ') && !current_hash.is_empty() {
                        // " 1 file changed, 1 insertion(+)"
                        let parts: Vec<&str> = line.split(',').collect();
                        let mut files = "0";
                        let mut ins = "0";
                        let mut del = "0";

                        for part in parts {
                            let part = part.trim();
                            if part.contains("file") {
                                files = part.split_whitespace().next().unwrap_or("0");
                            } else if part.contains("insertion") {
                                ins = part.split_whitespace().next().unwrap_or("0");
                            } else if part.contains("deletion") {
                                del = part.split_whitespace().next().unwrap_or("0");
                            }
                        }

                        let stats = CommitStats {
                            files: files.to_string(),
                            insertions: ins.to_string(),
                            deletions: del.to_string(),
                        };
                        self.commit_stats.insert(current_hash.clone(), stats);
                    }
                }
            }

            // Inject stats into git_log
            if let Ok(hash_re) = regex::Regex::new(r"\b[0-9a-f]{7}\b") {
                for line in &mut self.git_log.lines {
                    let content = line.to_string();
                    if let Some(mat) = hash_re.find(&content) {
                        let hash = mat.as_str();
                        if let Some(stats) = self.commit_stats.get(hash) {
                            line.spans.push(ratatui::text::Span::styled(
                                format!(" [{}f ", stats.files),
                                Style::default().fg(Color::DarkGray),
                            ));
                            line.spans.push(ratatui::text::Span::styled(
                                format!("+{}", stats.insertions),
                                Style::default().fg(Color::Green),
                            ));
                            line.spans.push(ratatui::text::Span::styled(
                                format!("/-{}", stats.deletions),
                                Style::default().fg(Color::Red),
                            ));
                            line.spans.push(ratatui::text::Span::styled(
                                "]",
                                Style::default().fg(Color::DarkGray),
                            ));
                        }
                    }
                }
            }

            // Refresh Event Stream from Log File
            self.events.clear();
            if let Some(home) = home::home_dir() {
                let log_path = home.join(".arcane").join("daemon.log");
                if let Ok(content) = std::fs::read_to_string(log_path) {
                    // Take last 20 lines
                    self.events = content
                        .lines()
                        .rev()
                        .take(20)
                        .map(|s| s.to_string())
                        .collect();
                    // In TUI, index 0 is top, so we want newest (rev) at 0?
                    // Or oldest at 0? Paragraph renders top-down.
                    // If we want a scrolling log like tail, we want oldest first, and new lines at bottom.
                    // The .rev().take(20) gives us the newest 20 lines, but in reverse order (newest first).
                    // So we need to reverse again to display them chronologically.
                    self.events.reverse();
                }
            }

            // Add status info if no logs yet
            if self.events.is_empty() {
                if let Some(s) = &self.status {
                    if s.state == "Running" {
                        self.events
                            .push(format!("Daemon PID: {} (No logs yet)", s.pid));
                    }
                } else {
                    self.events
                        .push("Waiting for daemon activity...".to_string());
                }
            }

            // Refresh Status (Dashboard)
            self.refresh_status();
        }
    }

    pub fn refresh_status(&mut self) {
        let output = std::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.working_tree = stdout
                .lines()
                .map(|line| {
                    let (status, path) = line.split_at(2);
                    let path = path.trim().to_string();
                    let change_type = match status {
                        "!!" => ChangeType::Other, // Ignored
                        "??" => ChangeType::Untracked,
                        s if s.starts_with('M') || s.ends_with('M') => ChangeType::Modified,
                        s if s.starts_with('A') || s.starts_with('R') => ChangeType::Staged,
                        _ => ChangeType::Other,
                    };
                    FileStatus {
                        path,
                        status: change_type,
                    }
                })
                .collect();

            // Clamp selection
            if !self.working_tree.is_empty() && self.selected_file_idx >= self.working_tree.len() {
                self.selected_file_idx = self.working_tree.len() - 1;
            } else if self.working_tree.is_empty() {
                self.selected_file_idx = 0;
            }
        }
    }

    pub fn ignore_selected_file(&mut self) {
        if self.current_tab != 0 {
            return;
        }
        if self.working_tree.is_empty() {
            return;
        }
        if self.selected_file_idx >= self.working_tree.len() {
            return;
        }

        let file = &self.working_tree[self.selected_file_idx];

        let file_obj = OpenOptions::new()
            .create(true)
            .append(true)
            .open(".gitignore");

        if let Ok(mut f) = file_obj {
            if let Err(e) = writeln!(f, "{}", file.path) {
                // Ideally show error in UI
                eprintln!("Failed to write to .gitignore: {}", e);
            }
        }

        // Refresh immediately
        self.refresh_status();
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tabs.len();
        self.scroll = 0;
    }

    pub fn previous_tab(&mut self) {
        if self.current_tab > 0 {
            self.current_tab -= 1;
        } else {
            self.current_tab = self.tabs.len() - 1;
        }
        self.scroll = 0; // Reset scroll when switching tabs
    }

    pub fn scroll_up(&mut self) {
        if self.show_popup {
            if self.popup_scroll > 0 {
                self.popup_scroll -= 1;
            }
            return;
        }

        match self.current_tab {
            0 => {
                // Dashboard: File Selection
                if self.selected_file_idx > 0 {
                    self.selected_file_idx -= 1;
                }
            }
            1 => {
                // Git Graph: Commit Selection
                if self.selected_row > 0 {
                    self.selected_row -= 1;
                    // Scroll logic: if selection goes above viewport, decrease scroll
                    if (self.selected_row as u16) < self.scroll {
                        self.scroll = self.selected_row as u16;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn scroll_down(&mut self) {
        if self.show_popup {
            self.popup_scroll += 1;
            return;
        }

        match self.current_tab {
            0 => {
                // Dashboard: File Selection
                if self.selected_file_idx < self.working_tree.len().saturating_sub(1) {
                    self.selected_file_idx += 1;
                }
            }
            1 => {
                // Git Graph: Commit Selection
                // Limit selection to list size
                if self.selected_row < self.git_log.lines.len().saturating_sub(1) {
                    self.selected_row += 1;

                    // Smart Scroll Down
                    // Ensure selection is within [scroll, scroll + vp_height]
                    // We give a margin of 2 lines from bottom
                    let safe_height = self.vp_height.saturating_sub(2).max(1);

                    if (self.selected_row as u16) >= self.scroll + safe_height {
                        self.scroll = (self.selected_row as u16) + 1 - safe_height;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn toggle_daemon(&mut self) {
        if let Some(status) = &self.status {
            // Stop Daemon
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .arg(status.pid.to_string())
                    .output();
            }
            self.status = None; // Optimistic update
        } else {
            // Start Daemon
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .arg("daemon")
                    .arg("run")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        }
    }

    pub fn toggle_auto_commit(&mut self) {
        self.ai_auto_commit = !self.ai_auto_commit;
        // In a real implementation, we would send a signal to the daemon or write this to a config file.
        // For now, we simulate the state in the UI.
        self.events.push(format!(
            "AI Auto-Commit: {}",
            if self.ai_auto_commit {
                "ENABLED"
            } else {
                "DISABLED"
            }
        ));
    }

    pub fn inspect_commit(&mut self) {
        if self.current_tab != 1 {
            return;
        }
        if self.selected_row < self.git_log.lines.len() {
            let line_content = self.git_log.lines[self.selected_row].to_string();
            // Regex match 7-char hash
            if let Ok(re) = regex::Regex::new(r"\b[0-9a-f]{7}\b") {
                if let Some(mat) = re.find(&line_content) {
                    let hash = mat.as_str();
                    let cmd = std::process::Command::new("git")
                        .args(&["show", hash, "--color=always"])
                        .output();

                    if let Ok(output) = cmd {
                        if let Ok(text) = output.stdout.into_text() {
                            self.popup_content = text;
                            self.show_popup = true;
                            self.popup_scroll = 0;
                        }
                    }
                }
            }
        }
    }

    pub fn close_popup(&mut self) {
        self.show_popup = false;
        self.popup_scroll = 0;
        self.popup_content = Text::default();
    }

    pub fn update_selection_details(&mut self) {
        if self.current_tab != 1 {
            return;
        }
        if self.selected_row >= self.git_log.lines.len() {
            self.commit_details = Text::raw("No commit selected");
            return;
        }

        let line_content = self.git_log.lines[self.selected_row].to_string();
        if let Ok(re) = regex::Regex::new(r"\b[0-9a-f]{7}\b") {
            if let Some(mat) = re.find(&line_content) {
                let hash = mat.as_str();
                // Use --stat and formatted output for a nice summary
                // We want:
                // Author: ...
                // Date: ...
                //
                // Stats...
                let cmd = std::process::Command::new("git")
                    .args(&[
                        "show",
                        hash,
                        "--stat",
                        "--format=Author: %an%nDate:   %ad%n%n%s%n",
                        "--color=always",
                    ])
                    .output();

                if let Ok(output) = cmd {
                    if let Ok(text) = output.stdout.into_text() {
                        self.commit_details = text;
                    }
                }
            } else {
                self.commit_details = Text::raw("No hash found in line");
            }
        }
    }

    /// Refresh identity data from ArcaneSecurity
    pub fn refresh_identity(&mut self) {
        use arcane::security::ArcaneSecurity;

        if let Ok(sec) = ArcaneSecurity::new(None) {
            // Load master public key
            if let Ok(identity) = sec.load_master_identity() {
                self.master_pubkey = Some(identity.to_public().to_string());
            }

            // Load team members
            if let Ok(members) = sec.list_team_members() {
                self.team_members = members;
            }

            // Load snapshots
            if let Ok(snaps) = sec.list_snapshots() {
                self.snapshots = snaps
                    .iter()
                    .map(|(name, _, size)| (name.clone(), *size))
                    .collect();
            }
        }
    }

    /// Scan repository for secrets
    pub fn scan_repo(&mut self) {
        use arcane::security::ArcaneSecurity;

        self.events
            .push("🔍 Scanning repository for secrets...".to_string());

        if let Ok(sec) = ArcaneSecurity::new(None) {
            match sec.scan_repo() {
                Ok(results) => {
                    self.scan_results = results
                        .iter()
                        .map(|(path, secrets)| {
                            (path.to_string_lossy().to_string(), secrets.clone())
                        })
                        .collect();

                    if self.scan_results.is_empty() {
                        self.events.push("✅ No secrets detected!".to_string());
                    } else {
                        self.events.push(format!(
                            "⚠️ Found {} files with secrets!",
                            self.scan_results.len()
                        ));
                    }
                }
                Err(e) => {
                    self.events.push(format!("❌ Scan failed: {}", e));
                }
            }
        }
    }

    /// Rotate repository keys
    pub fn rotate_keys(&mut self) {
        use arcane::security::ArcaneSecurity;

        self.events
            .push("🔄 Rotating repository keys...".to_string());

        if let Ok(sec) = ArcaneSecurity::new(None) {
            // Get current team members to re-encrypt for
            let keep_members = self.team_members.clone();

            match sec.rotate_repo_key(&keep_members) {
                Ok(_) => {
                    self.events
                        .push("✅ Keys rotated successfully!".to_string());
                }
                Err(e) => {
                    self.events.push(format!("❌ Key rotation failed: {}", e));
                }
            }
        }
    }

    /// Available AI providers
    pub fn provider_options() -> Vec<&'static str> {
        vec![
            "Auto",
            "Gemini",
            "OpenRouter",
            "OpenAI",
            "Anthropic",
            "Ollama",
        ]
    }

    /// Save AI config to disk
    pub fn save_ai_config(&mut self) {
        use arcane::ai_service::AIProvider;

        if let Ok(mut config) = arcane::config::ArcaneConfig::load() {
            // Parse providers
            let parse_provider = |s: &str| -> Option<AIProvider> {
                match s {
                    "Gemini" => Some(AIProvider::Gemini),
                    "OpenRouter" => Some(AIProvider::OpenRouter),
                    "OpenAI" => Some(AIProvider::OpenAI),
                    "Anthropic" => Some(AIProvider::Anthropic),
                    "Ollama" => Some(AIProvider::Ollama),
                    _ => None,
                }
            };

            config.ai_provider = parse_provider(&self.current_ai_provider);
            config.backup_provider_1 = parse_provider(&self.backup_provider_1);
            config.backup_provider_2 = parse_provider(&self.backup_provider_2);

            // Save model selections
            config.primary_model = if self.primary_model.is_empty() {
                None
            } else {
                Some(self.primary_model.clone())
            };
            config.backup1_model = if self.backup1_model.is_empty() {
                None
            } else {
                Some(self.backup1_model.clone())
            };
            config.backup2_model = if self.backup2_model.is_empty() {
                None
            } else {
                Some(self.backup2_model.clone())
            };

            config.timing.inactivity_delay = self.inactivity_delay;
            config.timing.min_commit_delay = self.min_commit_delay;
            config.version_bumping = self.version_bumping;
            config.ignore_patterns = self.ignore_patterns.clone();
            config.gitattributes_patterns = self.gitattributes_patterns.clone();
            config.system_prompt = self.system_prompt.clone();

            // Save per-provider model overrides
            config.model_overrides = self.model_overrides.clone();

            match config.save() {
                Ok(_) => self.events.push("✅ Config saved!".to_string()),
                Err(e) => self.events.push(format!("❌ Save failed: {}", e)),
            }
        }
    }

    pub fn reset_config_section(&mut self, section: &str) {
        if let Ok(mut config) = crate::config::ArcaneConfig::load() {
            config.reset_to_defaults(section);
            if let Err(e) = config.save() {
                self.events
                    .push(format!("❌ Failed to reset section: {}", e));
                return;
            }
            // Reload into app
            match section {
                "gitignore" => self.ignore_patterns = config.ignore_patterns.clone(),
                "gitattributes" => {
                    self.gitattributes_patterns = config.gitattributes_patterns.clone()
                }
                "prompt" => self.system_prompt = config.system_prompt.clone(),
                _ => {}
            }
            self.events
                .push(format!("✅ Reset {} to defaults", section));
        }
    }

    pub fn add_team_member(&mut self, public_key: String) {
        let key_trimmed = public_key.trim().to_string();
        if key_trimmed.is_empty() {
            self.events.push("❌ Empty key".to_string());
            return;
        }

        // Add to in-memory list
        self.team_members.push(key_trimmed.clone());

        // Persist to file
        if let Err(e) = self.save_team_members() {
            self.events.push(format!("❌ Failed to save: {}", e));
        } else {
            self.events.push(format!(
                "✅ Added: {}...",
                &key_trimmed[..20.min(key_trimmed.len())]
            ));
        }
    }

    pub fn remove_team_member(&mut self, idx: usize) {
        if idx >= self.team_members.len() {
            return;
        }

        let removed = self.team_members.remove(idx);

        // Persist to file
        if let Err(e) = self.save_team_members() {
            self.events.push(format!("❌ Failed to save: {}", e));
        } else {
            self.events.push(format!(
                "❌ Removed: {}...",
                &removed[..20.min(removed.len())]
            ));
        }

        // Adjust selection
        if self.selected_team_idx >= self.team_members.len() && self.selected_team_idx > 0 {
            self.selected_team_idx -= 1;
        }
    }

    fn save_team_members(&self) -> anyhow::Result<()> {
        use std::io::Write;

        // Ensure .arcane directory exists
        std::fs::create_dir_all(".arcane")?;

        let mut file = std::fs::File::create(".arcane/recipients")?;
        for key in &self.team_members {
            writeln!(file, "{}", key)?;
        }

        Ok(())
    }
}
