# Arcane Roadmap

> **Philosophy**: Zero cloud dependencies. You own everything. Scales from solo dev laptop to enterprise build servers.

---

## What Arcane Is NOT

-   **Not Kubernetes** - No pods, services, ingress, operators
-   **Not Coolify/Vercel** - No hosted management UI
-   **Not CI/CD** - No YAML pipelines, no GitHub Actions dependency

## What Arcane IS

-   **Local-first** - Build on your machine, push to your servers
-   **Direct SSH** - No agents to install on servers
-   **Baked secrets** - Encrypted at rest, decrypted during deploy
-   **Simple mental model** - `arcane deploy` = done

---

## Current State (v0.1.36)

### ✅ Stable - Core

| Feature                         | Status  |
| ------------------------------- | ------- |
| Envelope encryption (AGE)       | ✅ Done |
| Team key sharing                | ✅ Done |
| Machine/server keys             | ✅ Done |
| Secret scanning & blocking      | ✅ Done |
| Git filters (encrypt on commit) | ✅ Done |
| `arcane run` (runtime decrypt)  | ✅ Done |

### ✅ Stable - Deployment

| Feature                             | Status  |
| ----------------------------------- | ------- |
| Single image deploy                 | ✅ Done |
| Docker Compose deploy               | ✅ Done |
| Directory context upload (tar-pipe) | ✅ Done |
| Blue/Green with Caddy               | ✅ Done |
| Deploy locks (per-server)           | ✅ Done |
| `--dry-run` flag                    | ✅ Done |
| Server groups                       | ✅ Done |
| `--parallel` flag                   | ✅ Done |
| Environment injection (`--env`)     | ✅ Done |
| Remote logs (`arcane logs`)         | ✅ Done |
| Remote exec (`arcane exec`)         | ✅ Done |

### ✅ Stable - TUI & AI

| Feature                       | Status  |
| ----------------------------- | ------- |
| Sovereign Terminal (all tabs) | ✅ Done |
| Desktop notifications         | ✅ Done |
| AI auto-commit (Ollama)       | ✅ Done |
| Smart Squash                  | ✅ Done |
| Semantic versioning           | ✅ Done |
| Daemon loading indicator      | ✅ Done |

---

## 🔲 Remaining Features

### Priority 1: Core Gaps

| Feature           | Description                                  | Effort |
| ----------------- | -------------------------------------------- | ------ |
| `arcane rollback` | Keep last N images, instant revert           | Medium |
| `arcane validate` | Pre-deploy config check (DNS, SSH, env vars) | Medium |
| `arcane stop`     | Kill switch for all containers on a server   | Low    |
| `arcane status`   | Show what's running on which port            | Low    |

### Priority 2: Build Automation

| Feature                        | Description                                  | Effort |
| ------------------------------ | -------------------------------------------- | ------ |
| **GitHub Actions Integration** | Use GitHub as build server (no Spark needed) | Low    |
| Arcane Spark (self-hosted)     | Webhook listener for push-to-deploy          | Medium |
| Git polling mode               | Alternative to webhooks                      | Low    |
| Status notifications           | Discord/Slack on deploy                      | Low    |

### Priority 3: Networking

| Feature                  | Description               | Effort |
| ------------------------ | ------------------------- | ------ |
| Custom domains in config | Map domains to containers | Medium |
| Wildcard certs           | `*.app.com` via Caddy     | Low    |

---

## 💡 New Ideas (from do.md)

### 1. GitHub as Build Server

Instead of building Arcane Spark, leverage GitHub Actions:

```yaml
# .github/workflows/deploy.yml
on:
    push:
        branches: [main]
jobs:
    deploy:
        runs-on: ubuntu-latest
        env:
            ARCANE_MACHINE_KEY: ${{ secrets.ARCANE_MACHINE_KEY }}
        steps:
            - uses: actions/checkout@v4
            - run: cargo install --git https://github.com/DraconDev/arcane
            - run: arcane deploy --target production --env production
```

**Benefits:**

-   GitHub does the building (beefy runners)
-   Arcane only handles the push + secrets
-   No Spark server to maintain

### 2. `arcane status` Command

Show what's running on each server:

```bash
$ arcane status micro1
micro1 (132.145.59.238)
├── app-api        :8080  (healthy)
├── app-frontend   :80    (healthy)
├── app-grafana    :3000  (healthy)
├── postgres       -      (healthy)
└── redis          -      (healthy)
```

---

## Open Questions

-   [ ] Should compose pull images from registry or push locally like Garage Mode?
-   [ ] How to handle compose volumes? (Persistent data on remote)
-   [ ] Rollback strategy: keep last N images? Automatic on health fail?

---

## What We're NOT Building

| Feature               | Why Not                                     |
| --------------------- | ------------------------------------------- |
| Web Dashboard         | Security liability. TUI is faster.          |
| One-Click Marketplace | Attracts hobbyists, not pros.               |
| Buildpacks            | Magic that fails. Dockerfiles are explicit. |
| Kubernetes            | Overkill for 99% of projects.               |
| Docker Swarm          | Dead technology.                            |
