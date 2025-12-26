# Arcane Competitive Analysis & Feature Strategy

This document defines what we **Keep**, **Adapt**, or **Kill** from the entire ecosystem of deployment tools.

> **Goal:** Build a tool that is simpler than Kamal, more secure than Coolify, and more flexible than Vercel.

---

## Part 1: The Competitors

### 1.1 Coolify (The "All-in-One" Self-Hosted PaaS)

**What it is:** Open-source Heroku/Vercel alternative. Installs a control plane on your server with a web UI.

| Feature                           | Arcane Stance       | Reasoning                                                                                                    |
| --------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Git Pull Deployment**           | **Kill**            | Building on server is slow and resource-intensive. A $5 VPS can't compile Rust. We push pre-built artifacts. |
| **Web Dashboard**                 | **Kill**            | Security liability. Needs auth, updates, hosting. Our TUI/CLI is faster and more secure.                     |
| **Private Registry Support**      | **Keep**            | Essential for closed-source corporate work.                                                                  |
| **Docker Compose Support**        | **Keep**            | Their best feature. We need this (Phase 2).                                                                  |
| **Automatic HTTPS (via Traefik)** | **Adapt (Caddy)**   | Traefik is complex. Caddy is simpler with automatic HTTPS.                                                   |
| **Kill Switch / Stop All**        | **Keep**            | Need `arcane stop` to halt everything on a server quickly.                                                   |
| **One-Click App Templates**       | **Kill**            | "One click WordPress" attracts wrong users. We focus on devs deploying _their_ code.                         |
| **Backups (DB to S3)**            | **Adapt (Sidecar)** | Don't build backup logic in. Use a sidecar container (e.g., `postgres-backup`).                              |
| **Service Status Dashboard**      | **Adapt (TUI)**     | We show this in the TUI Ops tab, not a web page.                                                             |
| **Rollback UI**                   | **Adapt (CLI)**     | `arcane rollback citadel` instead of a button.                                                               |
| **Resource Monitoring**           | **Defer**           | Nice to have. Could SSH + `docker stats`. Not critical.                                                      |
| **SSH Terminal in Browser**       | **Kill**            | Use your own terminal. We're not building a web shell.                                                       |
| **Team Permissions**              | **Adapt (Keys)**    | No user/pass. Identity via AGE crypto keys. Much more secure.                                                |

---

### 1.2 Kamal (The Closest Cousin - by 37signals/Basecamp)

**What it is:** "Deploy anywhere" tool from the makers of Rails/Basecamp. Uses Traefik + SSH. Very similar philosophy to Arcane.

| Feature                        | Arcane Stance       | Reasoning                                                                         |
| ------------------------------ | ------------------- | --------------------------------------------------------------------------------- |
| **Traefik Proxy**              | **Adapt (Caddy)**   | Kamal wraps Traefik. Caddy is simpler, auto-HTTPS without docker socket mounting. |
| **Blue/Green Deploy**          | **Keep (Critical)** | Spin up new → wait for health → switch traffic → kill old. **We need this.**      |
| **Health Checks (HTTP)**       | **Keep**            | Wait for `/health` to return 200 before switching.                                |
| **Deploy Locks**               | **Keep**            | Lock file on server prevents two devs deploying at once.                          |
| **"Accessories" (DBs, Redis)** | **Adapt (Compose)** | Kamal uses weird config format. We use standard `docker-compose.yaml`.            |
| **Multi-Server Deploys**       | **Keep**            | Deploy to N servers in sequence or parallel. Phase 3.                             |
| **Asset Bridge (for Rails)**   | **Kill**            | Rails-specific. We're framework-agnostic.                                         |
| **Secrets via 1Password/etc**  | **Kill**            | We have our own encrypted secrets. Better.                                        |
| **Docker Image Registry Push** | **Keep**            | Build locally → push to registry → pull on server.                                |
| **Rollback**                   | **Keep**            | Keep last N images, instant rollback.                                             |
| **`kamal audit`**              | **Adapt**           | Show who deployed what when. Git history + deploy logs.                           |
| **`kamal app logs`**           | **Keep**            | Stream logs from containers. Phase 4.                                             |
| **`kamal app exec`**           | **Keep**            | Run commands in containers remotely. `arcane exec citadel -- rails c`.            |

---

### 1.3 Dokku (The OG - Mini Heroku)

**What it is:** Install on server, `git push dokku main` to deploy. Uses Buildpacks.

| Feature                       | Arcane Stance | Reasoning                                                                              |
| ----------------------------- | ------------- | -------------------------------------------------------------------------------------- |
| **Buildpacks (Heroku-style)** | **Kill**      | "Magic" detection fails. Explicit Dockerfiles are reliable.                            |
| **Plugins (Redis, Postgres)** | **Kill**      | Plugin ecosystem = maintenance nightmare. Use Docker images.                           |
| **Git Push to Deploy**        | **Adapt**     | We use git for versioning, but `arcane deploy` command as the action. Better feedback. |
| **Process Types (Procfile)**  | **Kill**      | Docker is our process model. No Procfile parsing.                                      |
| **Config:set**                | **Adapt**     | We encrypt env vars in git, not set them on server.                                    |
| **Zero-Downtime (Checks)**    | **Keep**      | Dokku does this well. We need it.                                                      |
| **SSL via Let's Encrypt**     | **Keep**      | Caddy does this automatically.                                                         |
| **Custom Domains**            | **Keep**      | Map domains to containers.                                                             |
| **Network Isolation**         | **Keep**      | Containers on same network can talk; external can't.                                   |

---

### 1.4 CapRover (The Swarm GUI)

**What it is:** Web UI for Docker Swarm. One-click apps.

| Feature                         | Arcane Stance       | Reasoning                                                                                 |
| ------------------------------- | ------------------- | ----------------------------------------------------------------------------------------- |
| **Docker Swarm**                | **Kill**            | Swarm is dead. Compose is standard. K8s is overkill.                                      |
| **One-Click Apps Marketplace**  | **Kill**            | We don't want to maintain app definitions.                                                |
| **Webhooks for CI**             | **Adapt (Spark)**   | They rely on webhooks for everything. We reserve this for the Build Server (Spark) model. |
| **Wildcard SSL**                | **Keep**            | Caddy supports this. `*.app.com` certs.                                                   |
| **Force HTTPS**                 | **Keep**            | Automatic with Caddy.                                                                     |
| **Persistent Apps (Databases)** | **Adapt (Compose)** | Define in compose, we handle volumes.                                                     |

---

### 1.5 Vercel / Railway / Render (The Cloud PaaS)

**What it is:** Fully managed platforms. "Just push code."

| Feature                       | Arcane Stance     | Reasoning                                           |
| ----------------------------- | ----------------- | --------------------------------------------------- |
| **Zero Config Deploy**        | **Adapt**         | We're close. `arcane deploy` is one command.        |
| **Preview Deployments (PRs)** | **Defer (Spark)** | Build server would create `pr-123.staging.app.com`. |
| **Serverless Functions**      | **Kill**          | Not our model. We run containers.                   |
| **Edge Functions**            | **Kill**          | Use Cloudflare if you need edge.                    |
| **Instant Rollback**          | **Keep**          | Keep N versions, instant switch.                    |
| **Environment Variables UI**  | **Kill**          | Encrypted files in git > web UI.                    |
| **GitHub Integration**        | **Adapt (Spark)** | Webhooks trigger build server, not magic.           |
| **Custom Domains**            | **Keep**          | Essential.                                          |
| **Analytics Dashboard**       | **Defer**         | Nice to have. Use Plausible/Umami as a container.   |
| **Automatic Scaling**         | **Kill**          | Not our problem. Provision enough server.           |
| **Cold Starts**               | **N/A**           | We don't have cold starts. Containers run 24/7.     |

---

### 1.6 Kubernetes (The Enterprise Beast)

**What it is:** Container orchestration at scale. Complex.

| Feature                        | Arcane Stance     | Reasoning                                 |
| ------------------------------ | ----------------- | ----------------------------------------- |
| **Pods/Deployments**           | **Kill**          | Overkill for 99% of projects.             |
| **Services/Ingress**           | **Adapt (Caddy)** | Caddy replaces this simply.               |
| **ConfigMaps/Secrets**         | **Adapt**         | We encrypt secrets in git. Better.        |
| **Rolling Updates**            | **Keep**          | We need this (Phase 2).                   |
| **Health/Liveness Probes**     | **Keep**          | Phase 4.                                  |
| **Horizontal Pod Autoscaling** | **Kill**          | Not our model.                            |
| **Namespaces (Multi-tenant)**  | **Defer**         | Could do via server groups. Not critical. |
| **Helm Charts**                | **Kill**          | Too complex. Docker Compose is enough.    |
| **kubectl apply**              | **Adapt**         | `arcane deploy` is our equivalent.        |

---

## Part 2: Feature Ideas Extracted

### Deployment Features (Priority: High)

| Feature                        | Source    | Priority    | Notes                            |
| ------------------------------ | --------- | ----------- | -------------------------------- |
| **Docker Compose Support**     | Coolify   | ✅ Done     | Multi-container apps             |
| **Blue/Green Deploy**          | Kamal     | ✅ Done     | Zero-downtime switching          |
| **Health Checks (HTTP)**       | Kamal/K8s | ✅ Done     | Wait for `/health` before switch |
| **Deploy Locks**               | Kamal     | ✅ Done     | Prevent collision                |
| **Rollback (Keep N Versions)** | All       | **Phase 2** | `arcane rollback` (Pending)      |
| **Multi-Server Deploy**        | Kamal     | ✅ Done     | Server groups                    |
| **Preview Deployments**        | Vercel    | **Spark**   | Requires build server            |

### Secret Management (Priority: High - DONE)

| Feature               | Source        | Our Status    |
| --------------------- | ------------- | ------------- |
| **Encrypted at Rest** | SOPS          | ✅ Done (AGE) |
| **Machine Keys**      | Arcane-unique | ✅ Done       |
| **Zero Dev Access**   | Arcane-unique | ✅ Done       |
| **Team Sharing**      | Vault         | ✅ Done       |

### Networking (Priority: Medium)

| Feature             | Source       | Priority        | Notes         |
| ------------------- | ------------ | --------------- | ------------- |
| **Automatic HTTPS** | All          | ✅ Done (Caddy) | Let's Encrypt |
| **Wildcard Certs**  | CapRover     | **Low**         | `*.app.com`   |
| **Custom Domains**  | All          | **Phase 1**     | Map in config |
| **gRPC Support**    | Chimera need | ✅ Done (Caddy) | `h2c://`      |
| **WebSockets**      | Common       | ✅ Done (Caddy) | Automatic     |

### Observability (Priority: Medium)

| Feature            | Source     | Priority  | Notes                         |
| ------------------ | ---------- | --------- | ----------------------------- |
| **Remote Logs**    | Kamal      | ✅ Done   | `arcane logs citadel`         |
| **Container Exec** | Kamal      | ✅ Done   | `arcane exec citadel -- bash` |
| **Resource Stats** | Coolify    | **Low**   | SSH + `docker stats`          |
| **Deploy History** | All        | ✅ Done   | Git history (`arcane log`)    |
| **Audit Trail**    | Enterprise | **Defer** | Build server logs it          |

### Developer Experience (Priority: High)

| Feature                | Source        | Priority    | Notes                     |
| ---------------------- | ------------- | ----------- | ------------------------- |
| **One-Command Deploy** | All           | ✅ Done     | `arcane deploy`           |
| **TUI Dashboard**      | Arcane-unique | ✅ Done     | Live status               |
| **AI Commit Messages** | Arcane-unique | ✅ Done     | Daemon feature            |
| **Security Scanner**   | Arcane-unique | ✅ Done     | Pre-commit                |
| **Config Validation**  | Kamal         | **Phase 1** | `arcane validate`         |
| **Dry Run**            | Kamal         | **Phase 2** | `arcane deploy --dry-run` |

### Team & Scale (Priority: Medium)

| Feature                    | Source        | Priority    | Notes                             |
| -------------------------- | ------------- | ----------- | --------------------------------- |
| **Server Groups**          | Kamal         | ✅ Done     | `--group prod`                    |
| **Environment Separation** | All           | ✅ Done     | `staging.env` vs `production.env` |
| **Build Server (Spark)**   | Arcane-unique | **Phase 5** | Webhook listener                  |
| **Parallel Deploys**       | Kamal         | ✅ Done     | `--parallel` flag                 |
| **Sequential (Rolling)**   | Kamal         | ✅ Done     | Default for safety                |

---

## Part 3: What We're NOT Building

| Feature                    | Why Not                                     |
| -------------------------- | ------------------------------------------- |
| **Web Dashboard**          | Security liability. TUI is better.          |
| **One-Click Marketplace**  | Attracts hobbyists, not pros.               |
| **Buildpacks**             | Magic that fails. Dockerfiles are explicit. |
| **Docker Swarm**           | Dead technology.                            |
| **Kubernetes**             | Overkill for 99% of projects.               |
| **Edge/Serverless**        | Different model. Use Cloudflare Workers.    |
| **Database Management UI** | Use pgAdmin/Adminer as a container.         |
| **Plugin System**          | Maintenance nightmare.                      |
| **GitHub OAuth**           | SSH keys + AGE is more secure.              |

---

## Part 4: The Final Verdict

### We're Building The Best Of:

| Tool                  | What We Take                                                  |
| --------------------- | ------------------------------------------------------------- |
| **Kamal**             | Blue/Green deploys, health checks, deploy locks, multi-server |
| **Coolify**           | Docker Compose support, service flexibility                   |
| **Dokku**             | Simplicity, zero-config feel                                  |
| **Vercel**            | One-command deploys, instant rollback                         |
| **Arcane (Original)** | Encrypted secrets, machine keys, zero-trust security, TUI     |

### Our Unique Advantages:

1. **Zero Dev Access to Prod Secrets** - No one else does this.
2. **No Server-Side Control Plane** - Nothing to crash, update, or secure.
3. **Build Locally, Push Artifacts** - Use your beefy dev machine.
4. **Encrypted Git as Single Source of Truth** - No secret sprawl.
5. **Scales from Solo to Enterprise** - Same tool, just add machine keys.
