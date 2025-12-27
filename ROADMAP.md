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

## Current State (v0.1.37)

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

### Priority 1: Core Gaps (DONE ✅)

| Feature           | Description                                | Status  |
| ----------------- | ------------------------------------------ | ------- |
| `arcane rollback` | Swap current with backup container         | ✅ Done |
| `arcane validate` | Pre-deploy config check (SSH, Docker, env) | ✅ Done |
| `arcane halt`     | Kill switch for all containers on server   | ✅ Done |
| `arcane ps`       | Show running containers on server          | ✅ Done |

### Priority 2: Build Automation

| Feature                        | Description                                     | Status  |
| ------------------------------ | ----------------------------------------------- | ------- |
| **GitHub Actions Integration** | Document using GitHub as build server           | ✅ Done |
| **Arcane Spark**               | Self-hosted webhook listener for push-to-deploy | ✅ Done |
| Traefik Setup Script           | Auto-discovery reverse proxy                    | ✅ Done |
| Traefik Label Generation       | Auto-generate labels in compose                 | ✅ Done |
| GitHub Status API              | Report deploy pass/fail to commit               | ✅ Done |

### Priority 3: Networking

| Feature                  | Description                            | Status  |
| ------------------------ | -------------------------------------- | ------- |
| Custom domains in config | `arcane.domain` label in Compose       | ✅ Done |
| Wildcard certs           | `*.app.com` via Traefik                | 🔄 Next |
| Auto subdomain routing   | Project name → subdomain (if no label) | ✅ Done |

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

### 2. GitHub API Synergies

Automate the setup process using GitHub API since we likely have keys:

-   `arcane repo init-hook`: Automatically adds the Spark webhook to your GitHub repo.
-   **Status Checks**: Report deployment status back to the PR/Commit on GitHub.
-   **Deployments API**: Create "Deployment" events in GitHub for tracking.

### 3. `arcane status` Command

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
