# Arcane Project Status 🔮

**Last Updated:** December 27, 2024

## ✅ Production Ready

| Feature                      | Status    | Description                                          |
| ---------------------------- | --------- | ---------------------------------------------------- |
| **Git Filter Encryption**    | ✅ Stable | `.env` auto-encrypts on commit, decrypts on checkout |
| **Identity Management**      | ✅ Stable | `arcane identity show/new`                           |
| **Team Sharing**             | ✅ Stable | `arcane team add <alias> <key>`                      |
| **Server Authorization**     | ✅ Stable | `arcane deploy gen-key/allow`                        |
| **Runtime Decryption**       | ✅ Stable | `arcane run -- <command>`                            |
| **Secret Scanning**          | ✅ Stable | `arcane scan <path>`                                 |
| **Dashboard (TUI)**          | ✅ Stable | `arcane dashboard`                                   |
| **AI Commits**               | ✅ Beta   | `arcane start` with Ollama/OpenRouter                |
| **Auto-Init Daemon**         | ✅ Stable | `arcane daemon` watches for new repos                |
| **Auto-Ingress (Traefik)**   | ✅ Stable | Automatic HTTPS & Subdomain routing via labels       |
| **Standard Context Pruning** | ✅ Stable | Dynamic `.dockerignore` / `.gitignore` support       |

## 📚 Documentation

| Doc                                                                  | Purpose                       |
| -------------------------------------------------------------------- | ----------------------------- |
| [QUICKSTART.md](QUICKSTART.md)                                       | Solo, team, and server setup  |
| [docs/CLI.md](docs/CLI.md)                                           | Command reference             |
| [docs/KEY_ARCHITECTURE.md](docs/KEY_ARCHITECTURE.md)                 | Envelope encryption explained |
| [docs/TEAM_WORKFLOW.md](docs/TEAM_WORKFLOW.md)                       | Team collaboration guide      |
| [docs/COMPETITORS.md](docs/COMPETITORS.md)                           | Comparison with other tools   |
| [docs/GUARDIAN.md](docs/GUARDIAN.md)                                 | Auto-Init daemon setup        |
| [docs/INTELLIGENCE.md](docs/INTELLIGENCE.md)                         | AI commit configuration       |
| [docs/secrets-management-guide.md](docs/secrets-management-guide.md) | Deep dive into crypto         |

## 🎯 Roadmap

1. **GitHub Release** — Pre-built binaries for easy install (v0.1.38 ready)
2. **Persistent Volume Management** — Data survival on redeploy
3. **Remote Build Strategy** — Build on Spark, not locally
4. **Wildcard Certs** — `*.app.com` via Traefik DNS-01
