# Git Arcane 🔮 !!!

> **Encrypted Secrets in Git. Zero Cloud Required.**

Arcane is a **Git-native secrets manager** that encrypts your `.env` files transparently. Secrets are encrypted on commit, decrypted on checkout—just like normal files.

## Why Arcane?

| Problem                               | Solution                              |
| ------------------------------------- | ------------------------------------- |
| `.env` files in `.gitignore`          | ✅ Commit encrypted secrets safely    |
| Cloud dashboards (Doppler, Infisical) | ✅ No cloud required. Secrets in Git. |
| Team key sharing headaches            | ✅ Add teammates with one command     |
| Server deployment secrets             | ✅ Machine keys authorize decryption  |

## Quick Start

```bash
# Install
cargo install --git https://github.com/DraconDev/arcane

# Install AI (Optional - for Auto-Commits)
# curl -fsSL https://ollama.com/install.sh | sh

# Setup identity (once, ever)
arcane identity new

# Enable encryption in your project
cd myproject
arcane init

# That's it! .env files are now auto-encrypted on commit
echo "API_KEY=secret" >> .env
git add .env && git commit -m "Add secrets"  # Encrypted in Git!
```

## Commands

| Command                         | Purpose                                     |
| ------------------------------- | ------------------------------------------- |
| `arcane identity show`          | Show your public key (share with teammates) |
| `arcane identity new`           | Generate your master identity               |
| `arcane deploy gen-key`         | Generate a server key pair                  |
| `arcane deploy allow <key>`     | Authorize a server to decrypt               |
| `arcane team add <alias> <key>` | Add a teammate                              |
| `arcane run -- <cmd>`           | Run command with decrypted secrets          |
| `arcane scan <file>`            | Scan for leaked secrets                     |
| `arcane daemon ...`             | Auto-init new repos in background           |
| `arcane dashboard`              | Launch the Sovereign Terminal (TUI)         |
| `arcane start [path]`           | Start AI Auto-Commit Daemon                 |

## How It Works

```
Developer Machine              Git Repository            Server
─────────────────              ──────────────            ──────
.env (plaintext)   ─commit→    .env (encrypted)   ─clone→  .env (encrypted)
     │                                                          │
     └── auto-decrypt ←────────────────────────────── arcane run ──┘
         on checkout                                   (decrypts at runtime)
```

-   **Single Source of Truth**: Edit secrets locally, commit, push. Everyone gets the same `.env`.
-   **Envelope Encryption**: Each repo has a unique key, wrapped for each authorized user/machine.
-   **No Cloud**: Everything stored in `.git/arcane/` (encrypted, versionable).
-   **Instant Revocation**: Delete a key file → access revoked immediately.

## Team & Server Access

```bash
# Invite a teammate (they share their public key with you)
arcane team add alice age1alice...
git add .git/arcane && git commit -m "Add Alice" && git push

# Revoke instantly
rm .git/arcane/keys/user:alice.age && git add -u && git commit -m "Bye Alice" && git push

# Authorize a server
arcane deploy gen-key            # On server: generates key pair
arcane deploy allow age1server...  # On laptop: authorize that key
```

**No passwords. No cloud accounts. No API calls at runtime.**

## Documentation

-   [**QUICKSTART.md**](QUICKSTART.md) — Solo, Team, and Server setup guides
-   [**docs/CLI.md**](docs/CLI.md) — Command reference
-   [**docs/KEY_ARCHITECTURE.md**](docs/KEY_ARCHITECTURE.md) — How envelope encryption works
-   [**docs/TEAM_WORKFLOW.md**](docs/TEAM_WORKFLOW.md) — Inviting teammates
-   [**docs/COMPETITORS.md**](docs/COMPETITORS.md) — Arcane vs Infisical, Doppler, SOPS, etc.
-   [**docs/GUARDIAN.md**](docs/GUARDIAN.md) — Sovereign Guardian (Auto-Init) setup
-   [**docs/INTELLIGENCE.md**](docs/INTELLIGENCE.md) — Sovereign Intelligence (Auto-Commit) guide

## Project Structure

```
arcane/
├── src/                 # Core: Git filter, crypto, CLI, TUI
├── examples/secrets-demo/ # Demo project for testing
└── docs/                # Documentation
```

## Status

| Feature                        | Status    |
| ------------------------------ | --------- |
| Git filter encryption          | ✅ Stable |
| `arcane run` (runtime decrypt) | ✅ Stable |
| Team key sharing               | ✅ Stable |
| Machine/server keys            | ✅ Stable |
| Sovereign Guardian (Auto-Init) | ✅ Stable |
| AI-powered commits             | ✅ Beta   |
| Sovereign Terminal (TUI)       | ✅ Stable |

## License

**Free** for individuals, open source, and companies with fewer than 5 employees.

**Commercial license required** for companies with 5+ employees.  
See [LICENSE](LICENSE) for details and pricing.
