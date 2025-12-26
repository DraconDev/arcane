# Arcane Roadmap

> **Philosophy**: Build an amazing tool for solo devs and small teams. Zero cloud dependencies. You own everything.

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

## Current State (v0.1.x)

### ✅ Stable

-   **Security Layer**: Envelope encryption, team keys, machine keys, secrets scanning
-   **Git Integration**: Transparent encrypt/decrypt, git filters, shadow branches
-   **TUI Dashboard**: Tabs, graph, settings, identity vault
-   **Desktop Notifications**: Security alerts with click-to-open

### ✅ Beta

-   **AI Auto-Commit**: Daemon watches files, generates commit messages
-   **Smart Squash**: AI groups commits into Minors + Patches
-   **Push-to-Deploy**: `arcane deploy` pushes code + baked secrets to server
-   **Garage Mode**: Build locally → smoke test → push image

---

## Phase 1: Stage/Prod Environments

### The Problem

Currently deploying to one server at a time. Real apps need staging vs production.

### Solution

**Environment files (layered):**

```
config/envs/
├── base.env         # Shared defaults (API URLs, feature flags)
├── staging.env      # Staging overrides + secrets (encrypted)
└── production.env   # Prod overrides + secrets (encrypted)
```

**Server registry:**

```toml
# config/servers.toml
[servers.micro1]
host = "micro1.dracon.uk"
env = "staging"

[servers.oracle]
host = "oracle.dracon.uk"
env = "production"
```

**Commands:**

```bash
arcane deploy citadel --env staging    # Deploy to all staging servers
arcane deploy citadel --env production # Deploy to all prod servers
arcane deploy citadel                  # Default = staging (safe default)
```

**Safety:**

-   `--env production` prompts: "Deploy to PRODUCTION? [y/N]"
-   Or flag: `arcane deploy citadel --env production --yes`

---

## Phase 2: Docker Compose Support

### The Problem

Multi-container apps need compose, not just single Dockerfile.

### Solution

**Auto-detect:**

-   If `docker-compose.yml` exists → compose mode
-   Else → Dockerfile mode

**Compose Flow:**

1. Build images locally (or use pre-built)
2. Copy compose file to server
3. Push images to server (or pull from registry)
4. `docker compose up -d` on server

**Environment integration:**

-   Compose uses env files from Phase 1
-   `docker compose --env-file staging.env up -d`

---

## Phase 3: Server Groups

### The Problem

Deploying to 10 servers one by one is tedious.

### Solution

```toml
# config/servers.toml
[groups.prod]
servers = ["oracle", "micro2", "micro3"]
env = "production"

[groups.stage]
servers = ["micro1"]
env = "staging"
```

**Commands:**

```bash
arcane deploy citadel --group prod   # All prod servers
arcane deploy citadel --group stage  # All stage servers
```

**Rolling Deploy:**

-   Default: Sequential (one at a time)
-   `--parallel`: Simultaneous (faster, riskier)

---

## Phase 4: Health Checks

### The Problem

Deploy succeeds but app crashes on startup.

### Solution

```toml
# In project config or servers.toml
[deploy.health]
url = "/health"
port = 3000
timeout = 30
```

**Flow:**

1. Deploy completes
2. Wait up to 30s for `http://container:3000/health` to return 200
3. If fails: Log error, optionally rollback

---

## Future: Arcane Spark (Build Server)

**What it is:** A daemon that listens for GitHub webhooks and auto-deploys.

**Security Model: ✅ ALREADY COMPLETE**

The machine key infrastructure is ready. See [TEAM_WORKFLOW.md](docs/TEAM_WORKFLOW.md#build-servers--ci-arcane-spark):

-   `arcane deploy gen-key` - Generate machine identity for build server
-   `arcane deploy allow <pubkey>` - Authorize the build server
-   `ARCANE_MACHINE_KEY` env var - Build server uses this to decrypt

**What's missing:**

-   Webhook listener (HTTP server for GitHub/GitLab push events)
-   Git polling mode (alternative to webhooks)
-   Build queue (handle concurrent pushes)
-   Status notifications (Discord/Slack on deploy)

**When to build it:**

-   When you have a dedicated build server
-   When team members shouldn't deploy directly
-   When you want push-to-deploy automation

---

## Priority Order

| Priority | Feature                 | Reason                    |
| -------- | ----------------------- | ------------------------- |
| 1        | Stage/Prod Environments | Needed for any real app   |
| 2        | Compose Support         | Multi-container is common |
| 3        | Server Groups           | Quality of life           |
| 4        | Health Checks           | Deployment confidence     |
| 5        | Arcane Spark            | Only for teams            |

---

## Open Questions

-   [ ] Should compose pull images from registry or push locally like Garage Mode?
-   [ ] How to handle compose volumes? (Persistent data on remote)
-   [ ] Should we support `docker swarm` or just `docker compose`?
-   [ ] Rollback strategy: keep last N images? Automatic on health fail?

---

## Licensing

| User                    | License    |
| ----------------------- | ---------- |
| Solo devs               | Free       |
| Open source             | Free       |
| Companies < 5 employees | Free       |
| Companies 5+ employees  | Commercial |
