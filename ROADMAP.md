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

## Phase 1: Environment Management

### The Problem

Currently deploying to one server at a time. Real apps need staging vs production.

### Solution

**1. Config Validation (Kamal-style):**

-   `arcane validate` command to check config correctness before deploy.
-   Checks: env vars present, DNS resolves, SSH accessible.

**2. Environment files (layered):**

```
config/envs/
├── base.env         # Shared defaults (API URLs, feature flags)
├── staging.env      # Staging overrides + secrets (encrypted)
└── production.env   # Prod overrides + secrets (encrypted)
```

**3. Server Registry:** -> see `servers.toml`

**Commands:**

-   `arcane deploy citadel --env staging`
-   `arcane deploy citadel --env production` (with confirmation prompt)

---

## Phase 2: Docker Compose & Deploy Logic

### The Problem

Multi-container apps (Chimera) and zero-downtime needs.

### Solution

**1. Compose Support (Coolify-style):**

-   Auto-detect `docker-compose.yaml`.
-   Build locally -> Push images/compose file -> Up remotely.
-   **Persistent Volumes:** Explicitly defined to survive redeploys.

**2. Blue/Green Deployment (Kamal-style):**

-   1. Spin up new container (Green)
-   2. Wait for health check (Green is healthy?)
-   3. Switch traffic (Update Caddy/Traefik)
-   4. Kill old container (Blue)
-   _Result: Zero downtime._

**3. Deploy Locks (Kamal-style):**

-   Create `.arcane.lock` on server during deploy.
-   Prevents concurrent deployments from different devs.
-   `arcane lock release` to override.

**4. Dry Run:**

-   `arcane deploy --dry-run` to see exactly what would execute.

---

## Phase 3: Server Groups

### The Problem

Deploying to 10 web servers sequentially takes 10x the time.

### Solution

**1. Parallel Deployment:**

-   `arcane deploy web-cluster` deploys to all members concurrently.
-   Configurable concurrency (e.g. batch size).

**2. Rolling Strategy:**

-   Option to deploy to X% of fleet at a time.
-   Halt on failure to prevent fleet-wide outage.

**3. Group Config:**

-   Defined in `servers.toml`.
-   Can contain specific overrides (future).

---

## Phase 4: Observability (The "Missing Link")

### The Problem

"It crashed, why?" -> currently requires manual SSH.

### Solution

**1. Remote Logs:**

-   `arcane logs citadel --tail` -> Streams `docker logs` from remote via SSH.
-   Supports multi-server log streaming (merged output).

**2. Container Exec:**

-   `arcane exec citadel -- /bin/bash` -> Interactive shell.
-   `arcane exec citadel -- rails db:migrate` -> One-off commands.

**3. Health Checks:**

-   (Moved from Phase 2) - HTTP probes to ensure deployment success.

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

## Enterprise Considerations (The "Dragons")

For large enterprises, Arcane consciously avoids these features (use specialized tools if you need them):

| Feature           | Enterprise Need                        | Arcane's Stance                                       |
| ----------------- | -------------------------------------- | ----------------------------------------------------- |
| **Observability** | Centralized logging (ELK/Splunk)       | SSH in + `docker logs`. Or run a logging container.   |
| **Migrations**    | Zero-downtime DB schema changes        | You manage migrations manually or in startup scripts. |
| **Networking**    | Service Mesh (Istio), complex policies | Standard Docker networking. Keep it simple.           |
| **Audit**         | Immutable deploy logs                  | Git is your audit log. Build server logs deploys.     |

We optimize for **99% of teams**, not the 1% who need Istio.
