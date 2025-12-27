# ⚗️ Arcane

> **Simpler than Kamal. More secure than Coolify. No cloud like Vercel.**

Arcane is the **sovereign DevOps toolkit** for developers who want encrypted secrets, zero-downtime deployments, and AI-powered Git—without complex infrastructure or monthly fees.

---

## 🤔 Why Does This Exist?

| The Problem                                       | Arcane's Solution                            |
| ------------------------------------------------- | -------------------------------------------- |
| `.env` files in `.gitignore` create secret sprawl | ✅ Commit encrypted secrets directly to Git  |
| Doppler, Infisical, Vault require cloud accounts  | ✅ No cloud. Everything stored in your repo. |
| Coolify/Dokku need a control plane on your server | ✅ No server agent. Just SSH.                |
| Kubernetes is overkill for 99% of projects        | ✅ Docker Compose + SSH. Done.               |
| Terraform/Kamal configs are complex               | ✅ One command: `arcane deploy`              |

---

## 🚀 What Arcane Does

### 1. 🔐 Encrypted Secrets in Git

Commit your `.env` files safely. They're encrypted on commit, decrypted on checkout—transparently.

```
Developer Laptop              Git Repository              Production Server
─────────────────             ──────────────              ─────────────────
.env (plaintext)  ─commit→    .env (encrypted)  ─clone→   .env (encrypted)
      │                                                          │
      └── auto-decrypts ←────────────────────────── arcane run ──┘
          on checkout                              (decrypts at runtime)
```

**What makes it different:**

-   **No secret sprawl** — One `.env`, versioned in Git, shared with the team.
-   **Envelope encryption** — Each repo has a unique key, wrapped for each user/machine.
-   **Zero dev access to production** — Servers have their own keys. Devs never see prod secrets.
-   **Instant revocation** — Delete a key file → access revoked immediately.

### 2. 🚢 One-Command Deployments

Push Docker images or entire Compose stacks to any server.

```bash
# Deploy a single image (builds locally, pushes via SSH)
arcane deploy --target production --image myapp:latest

# Deploy a 9-service Docker Compose stack
arcane deploy --target staging --compose docker-compose.yaml --env staging
```

**How we compete:**

| Feature         | Kamal             | Coolify      | Arcane                                |
| --------------- | ----------------- | ------------ | ------------------------------------- |
| Build location  | Server            | Server       | **Your laptop** (faster, cheaper VPS) |
| Control plane   | Traefik on server | Full web UI  | **Nothing** (direct SSH)              |
| Secret handling | 1Password/Doppler | Web form     | **Encrypted in Git**                  |
| Deploy command  | `kamal deploy`    | Click button | `arcane deploy`                       |

**Under the hood:**

-   **Zstd Warp Drive** — Docker images compressed and pushed via SSH (no registry).
-   **Distributed Locking** — One deploy at a time per server.
-   **Blue/Green with Caddy** — Zero-downtime traffic switching.
-   **Environment Injection** — Secrets decrypted and baked into remote `.env`.

### 3. 🧠 AI-Powered Git Intelligence

Let AI handle the boring stuff.

-   **Auto-Commits** — Daemon watches for changes, generates semantic commit messages via Ollama.
-   **Smart Squash** — AI groups messy commits into clean version bumps.
-   **Semantic Versioning** — Automatic Major/Minor/Patch based on content.

### 4. 🖥️ Sovereign Terminal (TUI)

A keyboard-driven dashboard for everything.

```bash
arcane  # or `arcane dashboard`
```

| Tab        | Purpose                                       |
| ---------- | --------------------------------------------- |
| Dashboard  | Git status, working tree, ignored files       |
| Graph      | Rich, colorful commit history                 |
| Deploy     | Server groups, one-click deployments          |
| AI         | Configure prompts, models, toggle auto-commit |
| Vault      | Team members, machine keys, identity          |
| Versioning | Smart Squash, version bumps                   |

### 5. 👁️ Background Guardian (Daemon)

Set it and forget it.

-   **Auto-Init** — Automatically enables encryption for new Git repos.
-   **Secret Scanner** — Blocks commits with exposed API keys in source code.

-   **Desktop Notifications** — Alerts if secrets are about to leak.

### 6. ⚡ Arcane Spark (Self-Hosted CI)

Push-to-Deploy, solved.

-   **Webhook Server**: Listens for GitHub/GitLab pushes.
-   **Auto-Ingress**: Automatically adds host routing (`app.example.com`) and HTTPS.
-   **Status Reporting**: Updates GitHub commit status (✅ Success / ❌ Fail).

---

## 🏁 Quick Start

```bash
# 1. Install
cargo install --git https://github.com/DraconDev/arcane

# 2. Create identity (once, ever)
arcane identity new

# 3. Enable encryption in any project
cd myproject && arcane init

# 4. Done! Secrets encrypted on commit.
echo "API_KEY=sk_live_secret" >> .env
git add .env && git commit -m "Add secrets"
```

---

## 🔧 Commands

| Command                         | Purpose                            |
| ------------------------------- | ---------------------------------- |
| `arcane init`                   | Initialize encryption for repo     |
| `arcane identity new`           | Generate master identity key       |
| `arcane team add <alias> <key>` | Authorize a teammate               |
| `arcane deploy --target <name>` | Deploy (image or Compose)          |
| `arcane deploy gen-key`         | Generate server key pair           |
| `arcane deploy allow <key>`     | Authorize a server                 |
| `arcane logs <target>`          | Stream remote Docker logs          |
| `arcane exec <target> -- <cmd>` | Run command on server via SSH      |
| `arcane run -- <cmd>`           | Run locally with decrypted secrets |
| `arcane scan <file>`            | Scan for leaked secrets            |
| `arcane daemon start`           | Start background Guardian          |

| `arcane spark` | Start Webhook Build Server |
| `arcane dashboard` | Launch TUI |

---

## 👥 Team & Server Access

**Invite a teammate:**

```bash
arcane team add alice age1alice...
git add .git/arcane && git commit -m "Add Alice" && git push
```

**Authorize a server:**

```bash
arcane deploy gen-key       # On server: generate key
arcane deploy allow age1... # On laptop: authorize it
```

**Revoke instantly:**

```bash
rm .git/arcane/keys/user:alice.age && git commit -am "Remove Alice"
```

---

## 🆚 Competitive Positioning

### We're Building The Best Of:

| Tool        | What We Take                                    |
| ----------- | ----------------------------------------------- |
| **Kamal**   | Blue/Green deploys, health checks, deploy locks |
| **Coolify** | Docker Compose support, service flexibility     |
| **Dokku**   | Simplicity, zero-config feel                    |
| **Vercel**  | One-command deploys, instant rollback           |

### Our Unique Advantages:

1. **Zero Dev Access to Prod Secrets** — Devs work with staging. Only servers decrypt production.
2. **No Server-Side Control Plane** — Nothing to crash, update, or secure on your VPS.
3. **Build Locally, Push Artifacts** — Use your beefy dev machine. $5 VPSs can't compile Rust.
4. **One-Step Custom Domains** — Just add `arcane.domain` label to your Compose file. No UI syncing.
5. **Encrypted Git as Source of Truth** — No secret sprawl across dashboards.
6. **Scales from Solo to Enterprise** — Same tool, just add machine keys.

### What We're NOT Building:

| Feature               | Why Not                                     |
| --------------------- | ------------------------------------------- |
| Web Dashboard         | Security liability. TUI is faster.          |
| One-Click Marketplace | Attracts hobbyists, not pros.               |
| Buildpacks            | Magic that fails. Dockerfiles are explicit. |
| Kubernetes            | Overkill for 99% of projects.               |
| Docker Swarm          | Dead technology.                            |

---

## 📊 Feature Status

| Feature                         | Status    |
| ------------------------------- | --------- |
| Encrypted `.env` via Git Filter | ✅ Stable |
| Team key sharing                | ✅ Stable |
| Server/machine keys             | ✅ Stable |
| Docker image deployment         | ✅ Stable |
| Docker Compose deployment       | ✅ Stable |
| Parallel server group deploys   | ✅ Stable |
| Remote logs & exec              | ✅ Stable |
| Sovereign Terminal (TUI)        | ✅ Stable |
| Background Daemon (Guardian)    | ✅ Stable |
| AI auto-commits                 | ✅ Stable |
| Smart Squash                    | ✅ Stable |
| Semantic Versioning             | ✅ Stable |

---

## 📚 Documentation

| Document                                             | Description                         |
| ---------------------------------------------------- | ----------------------------------- |
| [QUICKSTART.md](QUICKSTART.md)                       | Solo, Team, and Server setup guides |
| [ROADMAP.md](ROADMAP.md)                             | Future plans and phases             |
| [docs/CLI.md](docs/CLI.md)                           | Full command reference              |
| [docs/KEY_ARCHITECTURE.md](docs/KEY_ARCHITECTURE.md) | How envelope encryption works       |
| [docs/DEPLOY.md](docs/DEPLOY.md)                     | Deployment guide (Docker, Compose)  |

| [docs/SPARK.md](docs/SPARK.md) | Self-hosted webhook server guide |
| [docs/COMPETITIVE_ANALYSIS.md](docs/COMPETITIVE_ANALYSIS.md) | How we compare to Kamal, Coolify, etc. |

---

## 📜 License

**Free** for individuals, open source, and companies with fewer than 5 employees.

**Commercial license required** for companies with 5+ employees.  
See [LICENSE](LICENSE) for details.

---

<p align="center">
  <b>Arcane</b> — Your code has a secret keeper.
</p>
