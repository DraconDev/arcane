# Arcane Competitive Analysis & Strategy

This document defines what we **Keep**, **Adapt**, or **Kill** from the entire ecosystem of deployment tools.

> **Goal:** Build a tool that is simpler than Kamal, more secure than Coolify, and more flexible than Vercel.

---

## 1. Coolify (The "All-in-One")

| Feature                 | Arcane Stance | Reasoning                                             |
| ----------------------- | ------------- | ----------------------------------------------------- |
| **Git Pull Deployment** | **Kill**      | Building on server is slow. We push artifacts.        |
| **Web Dashboard**       | **Kill**      | Security risk. Needs auth/updates. CLI/TUI is faster. |
| **Private Registry**    | **Keep**      | Essential for closed-source corporate work.           |
| **Kill Switch**         | **Keep**      | Need a way to stop everything fast (`arcane stop`).   |

## 2. Kamal (The Closest Cousin)

Kamal (by 37signals) is our closest philosophical rival: "Deploy anywhere, no PaaS".

| Kamal Feature           | Arcane Stance       | Reasoning                                                                                                                       |
| ----------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Traefik Proxy**       | **Adapt (Caddy)**   | Kamal essentially wrappers Traefik. We chose Caddy for simpler config and automatic HTTPS without docker-socket mounting hacks. |
| **Blue/Green Deploy**   | **Keep**            | Kamal spins up new container -> waits for health -> switches traffic -> kills old. **We need this.**                            |
| **Deploy Locks**        | **Keep**            | Prevents two devs pushing at once. We implement this via a lock file on the server.                                             |
| **"Accessories" (DBs)** | **Adapt (Compose)** | Kamal defines DBs in config. We prefer standard `docker-compose.yaml` so you can run it locally too.                            |

## 3. Dokku (The OG)

| Dokku Feature           | Arcane Stance | Reasoning                                                                                                     |
| ----------------------- | ------------- | ------------------------------------------------------------------------------------------------------------- |
| **Buildpacks (Heroku)** | **Kill**      | "Magic" detection fails often. Explicit Dockerfiles are reliable.                                             |
| **Plugins (Redis/etc)** | **Kill**      | Maintaining plugins is hard. Just use a Docker image.                                                         |
| **Git Push to Deploy**  | **Adapt**     | We use git for versioning, but `arcane deploy` command for the action. Gives better feedback than a git hook. |

## 4. CapRover (The Swarm GUI)

| CapRover Feature   | Arcane Stance     | Reasoning                                                                                 |
| ------------------ | ----------------- | ----------------------------------------------------------------------------------------- |
| **Docker Swarm**   | **Kill**          | Swarm is dead. Compose is standard. K8s is overkill.                                      |
| **One-Click Apps** | **Kill**          | Attracts hobbyists, not pros. We focus on custom code.                                    |
| **Webhooks**       | **Adapt (Spark)** | They rely on webhooks for everything. We reserve this for the Build Server (Spark) model. |

---

## 5. Deployment Capabilities Matrix

What capabilities do we need to match the "Best in Class"?

| Capability         | Coolify   | Kamal        | Arcane (Target)                     |
| ------------------ | --------- | ------------ | ----------------------------------- |
| **Zero Downtime**  | Yes       | Yes          | **Must Have** (via Caddy + Rolling) |
| **Health Checks**  | Yes       | Yes          | **Phase 4**                         |
| **Multi-Server**   | Yes       | Yes          | **Phase 3** (Groups)                |
| **Secret Mgmt**    | DB/File   | Env File     | **Encrypted Git** (Best in class)   |
| **Access Control** | User/Pass | SSH Key      | **Machine Keys** (Best in class)    |
| **Observability**  | Native UI | Logs command | **Logs command** (Phase 4)          |

---

## Critical Gaps to Close (The Plan)

To beat Kamal and Coolify, we need:

1.  **Docker Compose Support (Phase 2):** Kamal struggles here (uses "accessories"). Coolify does it well. We must support `docker-compose.yml` natively.
2.  **Rolling/Zero-Downtime Logic:** Kamal's strongest feature. We need to verify our Caddy config creates zero-downtime swaps.
3.  **Deploy Locks:** We need to ensure two people don't deploy to `citadel` at the exact same moment.

## The Verdict

**We are building:**

-   **Kamal's** reliability (Health checks, rolling deploys)
-   **Coolify's** flexibility (Compose support)
-   **Arcane's** security (Zero-trust secrets, Zero-config servers)
