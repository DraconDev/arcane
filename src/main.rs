use arcane::ai_service;
use arcane::config;
use arcane::doctor;
use arcane::file_watcher;
use arcane::git_operations;
// use arcane::history; // Unused
// use arcane::repo_manager; // Unused
use arcane::security;
use arcane::shadow;
// use arcane::timeline; // Unused

use clap::{Arg, Command};
use config::ConfigManager;
use file_watcher::FileWatcher;
use git_operations::GitOperations;
use std::path::Path;

pub mod ops;
pub mod tui; // TUI Module // Ops Module (Arcane Ops)

use arcane::DaemonStatus;

#[tokio::main]
async fn main() {
    let matches = Command::new("arcane")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Arcane Team")
        .about("Invisible Git Client with AI-powered workflows")
        .subcommand(
            Command::new("start")
                .about("Watch for changes and auto-commit (AI-powered)")
                .arg(
                    Arg::new("paths")
                        .required(false)
                        .default_value(".")
                        .num_args(1..),
                ),
        )
        .subcommand(
            Command::new("timeline")
                .about("Show timeline of changes")
                .hide(true),
        )
        .subcommand(
            Command::new("compare")
                .about("Compare changes between commits")
                .hide(true),
        )
        .subcommand(
            Command::new("review")
                .about("Review code changes")
                .hide(true),
        )
        .subcommand(
            Command::new("navigate")
                .about("Navigate through code history")
                .hide(true),
        )
        .subcommand(
            Command::new("clean")
                .about("Git clean filter (encrypt)")
                .arg(Arg::new("file").required(false))
                .hide(true),
        )
        .subcommand(
            Command::new("smudge")
                .about("Git smudge filter (decrypt)")
                .hide(true),
        )
        .subcommand(
            Command::new("setup").about("Configure global git filters (run once after install)"),
        )
        .subcommand(Command::new("init").about("Initialize Arcane security for this repo"))
        .subcommand(
            Command::new("scan")
                .about("Check files for leaked secrets (API keys, passwords)")
                .arg(Arg::new("path").required(true)),
        )
        .subcommand(
            Command::new("team")
                .about("Share access with teammates")
                .subcommand(
                    Command::new("create")
                        .about("Create a new Team (Sovereign SSO)")
                        .arg(Arg::new("name").required(true)),
                )
                .subcommand(
                    Command::new("add-repo")
                        .about("Add this repository to a Team")
                        .arg(Arg::new("team_name").required(true)),
                )
                .subcommand(
                    Command::new("invite")
                        .about("Create an invite for a user")
                        .arg(Arg::new("team_name").required(true))
                        .arg(Arg::new("user_pk").required(true).help("User's Public Key")),
                )
                .subcommand(
                    Command::new("accept")
                        .about("Accept a Team Invite file")
                        .arg(Arg::new("file").required(true)),
                )
                .subcommand(
                    Command::new("add")
                        .about("Add a team member directly (Legacy/Tier 3 only)")
                        .arg(Arg::new("alias").required(true))
                        .arg(Arg::new("key").required(true)),
                )
                .subcommand(Command::new("list").about("List team members")),
        )
        .subcommand(
            Command::new("deploy")
                .about("Arcane Ops deployment commands")
                .subcommand(Command::new("gen-key").about("Generate Machine Identity"))
                .subcommand(
                    Command::new("allow")
                        .about("Whitelist a Machine Key")
                        .arg(Arg::new("pub_key").required(true)),
                ),
        )
        .subcommand(
            Command::new("push")
                .about("Push deployment to remote server (Sovereign Cloud)")
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .required(true)
                        .help("Target server name (from servers.toml)"),
                )
                .arg(
                    Arg::new("app")
                        .short('a')
                        .long("app")
                        .required(true)
                        .help("App name (e.g. 'chimera')"),
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .default_value("latest")
                        .help("Image tag to deploy"),
                )
                .arg(
                    Arg::new("ports")
                        .long("ports")
                        .help("Comma-separated ports for Blue/Green deploy (e.g. '8001,8002')"),
                ),
        )
        .subcommand(
            Command::new("pull")
                .about("Pull state or logs from remote server (Placeholder)")
                .arg(Arg::new("target").short('t').required(true)),
        )
        .subcommand(
            Command::new("identity")
                .about("Manage your Arcane identity")
                .subcommand(
                    Command::new("show").about("Show your public key (share this with teammates)"),
                )
                .subcommand(Command::new("new").about("Generate a new master identity")),
        )
        .subcommand(
            Command::new("daemon")
                .about("Sovereign Guardian (Auto-Init Daemon)")
                .subcommand(Command::new("run").about("Start the daemon"))
                .subcommand(
                    Command::new("config").about("Configure the daemon").arg(
                        Arg::new("add")
                            .long("add")
                            .required(false)
                            .help("Add a watch root"),
                    ),
                ),
        )
        .subcommand(Command::new("status").about("Check daemon status"))
        .subcommand(Command::new("stop").about("Stop the daemon"))
        .subcommand(
            Command::new("run")
                .about("Execute command with secrets decrypted in memory")
                .arg(
                    Arg::new("env-file")
                        .long("env-file")
                        .short('e')
                        .help("Path to encrypted .env file (default: .env)")
                        .default_value(".env"),
                )
                .arg(Arg::new("command").num_args(1..).last(true).required(true)),
        )
        .subcommand(Command::new("ui").about("Alias for 'dashboard'").hide(true))
        .subcommand(
            Command::new("shadow")
                .about("View and restore automatic backups")
                .subcommand(
                    Command::new("list")
                        .about("List shadow commits")
                        .arg(Arg::new("limit").short('n').default_value("20")),
                )
                .subcommand(Command::new("restore").about("Restore from a shadow commit")),
        )
        .subcommand(
            Command::new("dashboard")
                .about("Launch the Sovereign Terminal implementation")
                .alias("dash")
                .alias("d"),
        )
        .subcommand(
            Command::new("install-hooks")
                .about("Install git hooks")
                .hide(true),
        )
        .subcommand(
            Command::new("run-hook")
                .about("Run a specific git hook")
                .arg(Arg::new("hook_name").required(true))
                .hide(true),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("install-hooks", _)) => {
            let repo_root =
                security::ArcaneSecurity::find_repo_root().expect("Failed to find repo root");
            let hooks_dir = repo_root.join(".git").join("hooks");
            std::fs::create_dir_all(&hooks_dir).expect("Failed to create hooks dir");

            let pre_commit_path = hooks_dir.join("pre-commit");
            let exe_path = std::env::current_exe().expect("Failed to get exe path");
            let exe_str = exe_path.to_string_lossy();

            let script = format!("#!/bin/sh\n'{}' run-hook pre-commit\n", exe_str);

            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&pre_commit_path, script).expect("Failed to write hook");
            let mut perms = std::fs::metadata(&pre_commit_path)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&pre_commit_path, perms).expect("Failed to set permissions");

            println!("✅ Installed pre-commit hook");
        }
        Some(("run-hook", sub_matches)) => {
            let _hook_name = sub_matches.get_one::<String>("hook_name");

            println!("🔮 Arcane Doctor (Pre-Commit Check)...");
            let repo_root = security::ArcaneSecurity::find_repo_root()
                .map_err(|e| e.to_string())
                .expect("Failed to find repo root");
            let report = doctor::ArcaneDoctor::new().run(&repo_root);

            if report.overall_health == doctor::CheckStatus::Fail {
                println!("❌ Commit blocked by Arcane Doctor.");
                for check in report.checks {
                    if check.status == doctor::CheckStatus::Fail {
                        println!("   - FAILING: {}", check.message);
                    }
                }
                std::process::exit(1);
            } else {
                println!("✅ Arcane Checks Passed");
            }
        }
        Some(("start", sub_matches)) => {
            let paths = sub_matches
                .get_many::<String>("paths")
                .expect("Paths are required")
                .map(|v| v.as_str())
                .collect::<Vec<_>>();

            start_arcane_daemon(paths).await;
        }
        Some(("status", _)) => {
            if let Some(status) = DaemonStatus::load() {
                println!("🤖 Arcane Daemon Status");
                println!("PID: {}", status.pid);
                println!("State: {}", status.state);
                println!("Watching: {:?}", status.watching);
                println!(
                    "Last Commit: {}",
                    status.last_commit.unwrap_or_else(|| "None".to_string())
                );
            } else {
                println!("❌ Arcane daemon is not running (or status file missing)");
            }
        }
        Some(("stop", _)) => {
            if let Some(status) = DaemonStatus::load() {
                println!("🛑 Stopping Arcane daemon (PID: {}...", status.pid);

                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg(status.pid.to_string()).output();
                }

                println!("✅ Daemon stopped.");
            } else {
                println!("❌ Could not find running daemon to stop.");
            }
        }
        Some(("log", _)) => {
            // For MVP, just run git log in the first watched path if available
            if let Some(status) = DaemonStatus::load() {
                if let Some(first_path) = status.watching.first() {
                    println!("📜 Recent Commits for {}", first_path);
                    std::process::Command::new("git")
                        .current_dir(first_path)
                        .args(&["log", "--oneline", "-n", "10"])
                        .status()
                        .expect("Failed to run git log");
                }
            } else {
                println!("❌ Daemon not running, cannot determine watched paths.");
            }
        }
        Some(("clean", sub_matches)) => {
            let file = sub_matches.get_one::<String>("file").map(|s| s.as_str());
            let security =
                security::ArcaneSecurity::new(None).expect("Failed to initialize security");
            if let Err(e) = security.seal_clean(file) {
                eprintln!("❌ Clean Filter Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(("smudge", _)) => {
            let security =
                security::ArcaneSecurity::new(None).expect("Failed to initialize security");
            if let Err(e) = security.seal_smudge() {
                eprintln!("❌ Smudge Filter Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(("setup", _)) => {
            // Global git filter configuration
            println!("🔧 Setting up Arcane global git filters...");

            let exe_path = std::env::current_exe().expect("Failed to get executable path");
            let exe_str = exe_path.to_string_lossy();

            // Configure git-arcane filter (new standard)
            let filters = [
                ("filter.git-arcane.clean", format!("'{}' clean %f", exe_str)),
                ("filter.git-arcane.smudge", format!("'{}' smudge", exe_str)),
                ("filter.git-arcane.required", "true".to_string()),
                // Also register as git-seal for backward compatibility with legacy repos
                ("filter.git-seal.clean", format!("'{}' clean %f", exe_str)),
                ("filter.git-seal.smudge", format!("'{}' smudge", exe_str)),
                ("filter.git-seal.required", "true".to_string()),
            ];

            let mut success = true;
            for (key, value) in &filters {
                let result = std::process::Command::new("git")
                    .args(&["config", "--global", key, value])
                    .output();

                match result {
                    Ok(output) if output.status.success() => {
                        println!("  ✓ {}", key);
                    }
                    _ => {
                        eprintln!("  ✗ Failed to set {}", key);
                        success = false;
                    }
                }
            }

            if success {
                println!("\n✅ Global setup complete!");
                println!("   - git-arcane filter: configured (new repos)");
                println!("   - git-seal filter: configured (legacy repos)");
                println!("\n💡 Tip: Run 'arcane init' in each repo to set up encryption.");
            } else {
                eprintln!("\n❌ Some configurations failed. Check git permissions.");
            }
        }
        Some(("init", _)) => {
            let security =
                security::ArcaneSecurity::new(None).expect("Failed to initialize security");
            match security.init_repo() {
                Ok(path) => println!("✅ Initialized Arcane security. Key saved to {:?}", path),
                Err(e) => eprintln!("❌ Init failed: {}", e),
            }
        }
        Some(("scan", sub_matches)) => {
            let path_str = sub_matches
                .get_one::<String>("path")
                .expect("Path required");
            let path = Path::new(path_str);
            if !path.exists() {
                eprintln!("❌ File not found: {}", path.display());
                std::process::exit(1);
            }

            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let security =
                        security::ArcaneSecurity::new(None).expect("Failed to initialize security");
                    let secrets = security.scan_content(&content);
                    if secrets.is_empty() {
                        println!("✅ No secrets found in {}", path.display());
                    } else {
                        println!("🚫 SECRETS DETECTED in {}:", path.display());
                        for secret in secrets {
                            println!("   - Found potential {}", secret);
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to read file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("team", sub_matches)) => match sub_matches.subcommand() {
            Some(("create", args)) => {
                let name = args.get_one::<String>("name").expect("Name required");
                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.create_team(name) {
                    Ok(_) => println!(
                        "✅ Created Team '{}'. Key saved to ~/.arcane/teams/{}.key",
                        name, name
                    ),
                    Err(e) => {
                        eprintln!("❌ Failed to create team: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("add-repo", args)) => {
                let team_name = args
                    .get_one::<String>("team_name")
                    .expect("Team name required");
                // Need repo context
                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize"); // Will auto-find repo
                match security.add_repo_to_team(team_name) {
                    Ok(_) => println!("✅ Added repository to Team '{}'", team_name),
                    Err(e) => {
                        eprintln!("❌ Failed to add repo to team: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("invite", args)) => {
                let team_name = args.get_one::<String>("team_name").expect("Team required");
                let user_pk = args.get_one::<String>("user_pk").expect("User PK required");

                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.create_team_invite(team_name, user_pk) {
                    Ok(_) => println!("✅ Invite created in .git/arcane/invites/{}/", team_name),
                    Err(e) => {
                        eprintln!("❌ Failed to create invite: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("accept", args)) => {
                let file_path = args.get_one::<String>("file").expect("File path required");
                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.accept_team_invite(Path::new(file_path)) {
                    Ok(team_name) => println!(
                        "✅ Accepted invite! You are now a member of Team '{}'",
                        team_name
                    ),
                    Err(e) => {
                        eprintln!("❌ Failed to accept invite: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("add", args)) => {
                let alias = args.get_one::<String>("alias").expect("Alias required");
                let key = args.get_one::<String>("key").expect("Key required");

                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.add_team_member(alias, key) {
                    Ok(_) => println!("✅ Added team member '{}'", alias),
                    Err(e) => {
                        eprintln!("❌ Failed to add member: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("list", _)) => {
                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.list_team_members() {
                    Ok(members) => {
                        println!("👥 Team Members:");
                        for member in members {
                            println!("   - {}", member);
                        }
                    }
                    Err(e) => eprintln!("❌ Failed to list members: {}", e),
                }
            }
            _ => println!("Use 'arcane team --help'"),
        },
        Some(("deploy", sub_matches)) => match sub_matches.subcommand() {
            Some(("gen-key", _)) => {
                let (priv_key, pub_key) = security::ArcaneSecurity::generate_machine_identity();
                println!("🤖 Generated Machine Identity");
                println!("");
                println!("Public Key (Authorize this):");
                println!("{}", pub_key);
                println!("");
                println!("Private Key (Set as ARCANE_MACHINE_KEY Env Var):");
                println!("{}", priv_key);
                println!("");
                println!("⚠️  Keep the Private Key secret! Do not commit it.");
            }
            Some(("allow", args)) => {
                let pub_key = args
                    .get_one::<String>("pub_key")
                    .expect("Public Key required");
                let security = security::ArcaneSecurity::new(None).expect("Failed to initialize");
                match security.whitelist_machine(pub_key) {
                    Ok(_) => println!("✅ Machine Authorized. It can now access this repo."),
                    Err(e) => {
                        eprintln!("❌ Failed to whitelist machine: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("push", args)) => {
                let target = args.get_one::<String>("target").unwrap();
                let _app = args.get_one::<String>("app").unwrap();
                let _tag = args.get_one::<String>("tag").unwrap();
                let _ports = args.get_one::<String>("ports").map(|s| s.as_str());

                match crate::ops::push::PushDeploy::deploy(target) {
                    Ok(_) => println!("✅ Push Successful"),
                    Err(e) => {
                        eprintln!("❌ Push Failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => println!("Use 'arcane deploy --help'"),
        },
        Some(("push", args)) => {
            let target = args.get_one::<String>("target").unwrap();
            let _app = args.get_one::<String>("app").unwrap();
            let _tag = args.get_one::<String>("tag").unwrap();
            let _ports = args.get_one::<String>("ports").map(|s| s.as_str());

            // Use new Source Push logic (Simple Shell)
            match crate::ops::push::PushDeploy::deploy(target) {
                Ok(_) => println!("✅ Push Successful"),
                Err(e) => {
                    eprintln!("❌ Push Failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("pull", _)) => {
            println!("📥 Arcane Pull: Not implemented yet (Coming soon: Logs/State sync)");
        }
        Some(("identity", sub_matches)) => match sub_matches.subcommand() {
            Some(("show", _)) => {
                // Read the master identity and derive public key
                let identity_path = dirs::home_dir()
                    .expect("Could not find home directory")
                    .join(".arcane")
                    .join("identity.age");

                if !identity_path.exists() {
                    eprintln!("❌ No identity found. Run 'arcane identity new' first.");
                    std::process::exit(1);
                }

                match std::fs::read_to_string(&identity_path) {
                    Ok(content) => {
                        // Parse the identity to get the public key
                        use age::x25519;
                        use std::str::FromStr;

                        // Find the secret key line
                        for line in content.lines() {
                            if line.starts_with("AGE-SECRET-KEY-") {
                                match x25519::Identity::from_str(line) {
                                    Ok(identity) => {
                                        let public_key = identity.to_public();
                                        println!("🔑 Your Arcane Identity");
                                        println!();
                                        println!("Public Key (share this with teammates):");
                                        println!("{}", public_key);
                                        println!();
                                        println!("Identity File: {}", identity_path.display());
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Failed to parse identity: {}", e);
                                        std::process::exit(1);
                                    }
                                }
                                return;
                            }
                        }
                        eprintln!("❌ No valid identity key found in file");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read identity file: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Some(("new", _)) => {
                // Generate a new master identity
                let identity_dir = dirs::home_dir()
                    .expect("Could not find home directory")
                    .join(".arcane");

                let identity_path = identity_dir.join("identity.age");

                if identity_path.exists() {
                    eprintln!(
                        "⚠️  Identity already exists at: {}",
                        identity_path.display()
                    );
                    eprintln!(
                        "   To regenerate, delete it first: rm {}",
                        identity_path.display()
                    );
                    std::process::exit(1);
                }

                // Create directory if needed
                std::fs::create_dir_all(&identity_dir).expect("Failed to create .arcane directory");

                // Generate key
                let (priv_key, pub_key) = security::ArcaneSecurity::generate_machine_identity();

                // Write to file
                let content = format!(
                    "# created: {}\n# public key: {}\n{}\n",
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                    pub_key,
                    priv_key
                );

                std::fs::write(&identity_path, content).expect("Failed to write identity file");

                println!("🔐 Created new Arcane Identity");
                println!();
                println!("Public Key (share this with teammates):");
                println!("{}", pub_key);
                println!();
                println!("Identity saved to: {}", identity_path.display());
                println!();
                println!("⚠️  Back up your identity file! It's your master key.");
            }
            _ => println!("Use 'arcane identity --help'"),
        },
        Some(("daemon", sub_matches)) => match sub_matches.subcommand() {
            Some(("run", _)) => {
                if let Err(e) = arcane::daemon::start_daemon() {
                    eprintln!("❌ Daemon failed: {}", e);
                    std::process::exit(1);
                }
            }
            Some(("config", args)) => {
                if let Some(path_str) = args.get_one::<String>("add") {
                    let path = Path::new(path_str).to_path_buf();
                    if let Err(e) = arcane::daemon::add_watch_root(path) {
                        eprintln!("❌ Failed to add watch root: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("Usage: arcane daemon config --add <path>");
                }
            }
            _ => println!("Use 'arcane daemon --help'"),
        },
        Some(("run", sub_matches)) => {
            // POC: Just decrypt .env if it exists and run command
            let cmd_args: Vec<_> = sub_matches
                .get_many::<String>("command")
                .map(|v| v.collect())
                .unwrap_or_else(Vec::new);

            if cmd_args.is_empty() {
                eprintln!("❌ No command provided. Usage: arcane run -- <command>");
                std::process::exit(1);
            }

            // 1. Init Security (detects Machine Key automatically)
            let security = match security::ArcaneSecurity::new(None) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("❌ Security Init Failed (Check ARCANE_MACHINE_KEY): {}", e);
                    std::process::exit(1);
                }
            };

            // 2. Load Repo Key (Verify Access)
            if let Err(e) = security.load_repo_key() {
                eprintln!("❌ Access Denied: {}", e);
                std::process::exit(1);
            }

            // 3. Decrypt env file if exists
            let env_file = sub_matches
                .get_one::<String>("env-file")
                .map(|s| s.as_str())
                .unwrap_or(".env");

            let mut env_vars = std::collections::HashMap::new();
            if Path::new(env_file).exists() {
                if let Ok(content) = std::fs::read(env_file) {
                    if let Ok(repo_key) = security.load_repo_key() {
                        // Try decrypt (assuming it might be ciphertext)
                        if let Ok(decrypted) = security.decrypt_with_repo_key(&repo_key, &content) {
                            if let Ok(str_content) = String::from_utf8(decrypted) {
                                for line in str_content.lines() {
                                    if let Some((k, v)) = line.split_once('=') {
                                        env_vars.insert(k.trim().to_string(), v.trim().to_string());
                                    }
                                }
                                println!(
                                    "✅ Decrypted {} and injected {} variables.",
                                    env_file,
                                    env_vars.len()
                                );
                            }
                        }
                    }
                }
            } else {
                eprintln!(
                    "⚠️  Env file {} not found, proceeding without secrets.",
                    env_file
                );
            }

            // 4. Run Command
            let program = &cmd_args[0];
            let args = &cmd_args[1..];

            use std::os::unix::process::CommandExt;
            use std::process::Command;
            let err = Command::new(program).args(args).envs(&env_vars).exec();

            eprintln!("❌ Failed to exec: {}", err);
            std::process::exit(1);
        }
        Some(("ui", _)) => {
            // Legacy alias - redirect to dashboard
            println!("ℹ️  'arcane ui' is deprecated. Use 'arcane dashboard' instead.");
            run_dashboard();
        }
        Some(("shadow", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", args)) => {
                let limit: usize = args
                    .get_one::<String>("limit")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(20);

                let cwd = std::env::current_dir().expect("Failed to get current directory");
                let manager = shadow::ShadowManager::new(&cwd);
                match manager.list_shadow_commits(limit) {
                    Ok(commits) => {
                        println!("👻 Shadow Commits:");
                        for commit in commits {
                            println!(
                                "   {} | {} | {}",
                                &commit.sha[..8],
                                commit.date,
                                commit.message
                            );
                        }
                    }
                    Err(e) => eprintln!("❌ Failed to list shadow commits: {}", e),
                }
            }
            Some(("restore", args)) => {
                let sha = args.get_one::<String>("sha").expect("SHA required");

                let cwd = std::env::current_dir().expect("Failed to get current directory");
                let manager = shadow::ShadowManager::new(&cwd);
                match manager.restore_from_shadow(sha) {
                    Ok(_) => println!("✅ Restored from shadow commit: {}", sha),
                    Err(e) => {
                        eprintln!("❌ Failed to restore: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => println!("Use 'arcane shadow --help'"),
        },
        Some(("dashboard", _)) => {
            run_dashboard();
        }
        _ => {
            // Default to dashboard if no subcommand is provided
            run_dashboard();
        }
    }
}

fn run_dashboard() {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use std::io;

    // Setup Terminal
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).expect("Failed to setup terminal");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    // Create App and Run
    let app = tui::app::App::new();
    let res = tui::events::run_app(&mut terminal, app);

    // Restore Terminal
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .expect("Failed to restore terminal");
    terminal.show_cursor().expect("Failed to show cursor");

    if let Err(err) = res {
        eprintln!("\u{274c} TUI Error: {:?}", err);
    }
}

async fn start_arcane_daemon(paths: Vec<&str>) {
    println!("🚀 Starting Arcane daemon...");

    let config = ConfigManager::new().expect("Failed to load configuration");
    let ai_service = ai_service::AIService::new(config.ai_config());

    for path in paths {
        let path = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        println!("👀 Watching: {}", path.display());

        if !path.exists() {
            eprintln!("❌ Path does not exist: {}", path.display());
            continue;
        }

        if !is_git_repository(&path) {
            eprintln!("❌ Not a git repository: {}", path.display());
            continue;
        }

        println!("📁 Watching repository: {}", path.display());

        let git_ops = GitOperations::new();
        let security =
            security::ArcaneSecurity::new(Some(&path)).expect("Failed to initialize security");
        let mut file_watcher =
            FileWatcher::new(path.clone(), git_ops, ai_service.clone(), security);

        tokio::spawn(async move {
            let _ = file_watcher.start_watching().await;
        });
    }

    // Keep the daemon running
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    println!("👋 Arcane daemon stopped");
    std::process::exit(0);
}

fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use crate::ai_service::{AIConfig, AIProvider, AIService};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_ai_commit_message() {
        // Create a test configuration with the Gemini API key
        let mut provider_models = HashMap::new();
        provider_models.insert(AIProvider::Gemini, "gemini-1.5-flash-latest".to_string());

        let mut api_keys = HashMap::new();
        // Load key from env or use dummy for test structure
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            api_keys.insert(AIProvider::Gemini, key);
        } else {
            api_keys.insert(AIProvider::Gemini, "dummy_key_for_test".to_string());
        }

        let config = AIConfig {
            primary_provider: AIProvider::Gemini,
            backup_providers: vec![AIProvider::OpenRouter, AIProvider::OpenAI],
            provider_models,
            api_keys,
        };

        let ai_service = AIService::new(config);

        // Test with a simple diff
        let diff = "diff --git a/test.txt b/test.txt\nindex 1234567..abcdefg 100644\n--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-test content\n+modified content for AI testing";

        let result = ai_service.generate_commit_message(diff).await;
        // In most cases we expect a result, but since api_key might be dummy or network issues,
        // we just ensure it doesn't panic.
        println!("AI Result: {:?}", result);
    }
}
