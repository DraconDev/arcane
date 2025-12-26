# Arcane vs Coolify: Feature Analysis

This document defines what we **Keep**, **Adapt**, or **Kill** from the Coolify feature set.

> **Goal:** Build a tool that feels 10x lighter and more secure than Coolify, without missing critical capabilities.

---

## 1. Application Deployment

| Coolify Feature                  | Arcane Stance             | Reasoning                                                                                               |
| -------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------- |
| **Git integration (pull based)** | **Kill / Defer to Spark** | Building on the server is slow and resource intensive. We push pre-built artifacts.                     |
| **Dockerfile / Buildpacks**      | **Adapt (Repo-based)**    | We support Dockerfiles. We do NOT support magical buildpacks that guess your stack. Explicit is better. |
| **Private Registry Support**     | **Keep**                  | Essential. Arcane needs to push to private registries seamlessly.                                       |
| **Rollbacks**                    | **Keep (Adapt)**          | Coolify keeps old containers. We should too (keep last N versions).                                     |
| **Preview Deployments (PRs)**    | **Defer**                 | Requires a build server (Spark). Hard to do from laptop (who triggers it?).                             |

## 2. Databases & Services

| Coolify Feature        | Arcane Stance       | Reasoning                                                                                                 |
| ---------------------- | ------------------- | --------------------------------------------------------------------------------------------------------- |
| **One-Click DBs**      | **Adapt (Compose)** | Don't build a UI for this. Support `arcane deploy init --template postgres`. Use standard Docker Compose. |
| **Backups (S3/Local)** | **Adapt (Sidecar)** | Don't build backup logic into the daemon. Use a backup container sidecar (e.g. `postgres-backup`).        |
| **Service Templates**  | **Kill**            | "One click WordPress" attracts the wrong users. We focus on devs deploying _their_ code.                  |

## 3. Networking & Routing

| Coolify Feature        | Arcane Stance     | Reasoning                                                                            |
| ---------------------- | ----------------- | ------------------------------------------------------------------------------------ |
| **Traefik Proxy**      | **Adapt (Caddy)** | Traefik is complex. Caddy is simpler for zero-config HTTPS. We already use it.       |
| **Custom Domains**     | **Keep**          | Essential. `config.toml` should map domains to containers.                           |
| **TCP/UDP Ports**      | **Keep**          | Standard Docker port mapping.                                                        |
| **Path-based routing** | **Kill/Limit**    | `/api` -> Container A is complex. Subdomains (`api.app.com`) are cleaner and easier. |

## 4. UI & Management

| Coolify Feature     | Arcane Stance    | Reasoning                                                                     |
| ------------------- | ---------------- | ----------------------------------------------------------------------------- |
| **Web Dashboard**   | **Kill**         | A TUI + CLI is faster. A Web UI requires auth, security updates, and hosting. |
| **Team Management** | **Adapt (Keys)** | No "login". Identity is handled by Crypto Keys (AGE). Much more secure.       |
| **API**             | **Kill**         | The CLI _is_ the API. Automation happens via scripts wrapping the CLI.        |

## 5. Security

| Coolify Feature           | Arcane Stance         | Reasoning                                                                                    |
| ------------------------- | --------------------- | -------------------------------------------------------------------------------------------- |
| **Environment Variables** | **Adapt (Encrypted)** | Coolify stores them in DB (plaintext-ish). We bake them encrypted into the repo. Zero trust. |
| **SSH Key Management**    | **Keep**              | We use specific deploy keys.                                                                 |
| **Firewall Management**   | **Kill**              | Out of scope. User configures UFW/Security Groups once.                                      |

---

## Critical Gaps to Close (Phase 1 & 2)

To genuinely compete with the _useful_ parts of Coolify, Arcane needs:

1.  **Compose Support:** Coolify handles multi-container apps well. We need `docker-compose.yml` support immediately.
2.  **Persistent Volumes:** Coolify manages volumes clearly. We need a way to say "this data survives redeploys".
3.  **Logs:** `coolify logs` is a button. `arcane logs` needs to stream remote logs via SSH.

## The Verdict

**We are NOT building:**

-   A web control plane
-   A CI/CD runner on the production server
-   A "marketplace" of one-click apps

**We ARE building:**

-   A secure transport for local builds -> remote servers
-   A way to manage secrets without a web UI
-   A way to orchestrate Docker Compose remotely
