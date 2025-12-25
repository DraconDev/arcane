# ⚔️ Arcane Ops: The Sovereign Cloud Orchestrator

> **Vision**: Transform `arcane` from a Secrets Manager into a full "Hacker's PaaS" that replaces Coolify, Portainer, and manually managing SSH keys.
> **Philosophy**: "One Binary. Zero Bloat. Maximum Power."

---

## 1. The Manifesto

**The Problem:**

-   **Coolify/CapRover**: Great tools, but built on heavy dynamic languages (PHP/Node). They consume 1-2GB RAM just to be idle. They rely on complex databases (Postgres/Redis) that can fail.
-   **Dokku**: CLI only. Hard to visualize the state of your fleet.
-   **Portainer**: Just a Docker GUI. Lacks "Application" context.

**The Solution: Arcane Ops**
A **Rust-native, TUI-first, Agentic** orchestrator baked directly into the `arcane` binary.

-   **Agentic**: It doesn't just list containers; it understands _Deployments_.
-   **Sovereign**: No external SaaS. No telemetry. You own the binary, you own the fleet.
-   **Secretless Runtime**: The target server is "dumb". It needs NO keys, NO arcane binary, and NO state. Secrets are injected into RAM during deployment.

---

## 2. Architecture: "The Super-Binary"

We avoid creating a separate `arcane-ops` binary. Instead, we use **Cargo Features** to include Ops capabilities in the main `arcane` tool.

### Feature Flags

| Feature   | Description              | Dependencies  | Use Case                  |
| :-------- | :----------------------- | :------------ | :------------------------ |
| `default` | Full TUI + Ops + Secrets | `ratatui`     | Developer Laptop          |
| `minimal` | CLI Only (Secrets + Git) | `age`, `git2` | CI/CD Pipelines & Servers |

---

## 3. The Deployment Workflow (Enterprise v0.2.0)

Arcane Ops supports multiple deployment strategies via the same engine.

### Core Features

1.  **Distributed Locking**: Before any operation, Arcane acquires a lock (`/var/lock/arcane.deploy`) on the remote server. This ensures atomic, team-safe deployments.
2.  **The "Warp Drive" (Zstd Push)**: We do NOT use a Docker Registry.
    -   Your local machine streams the image: `docker save | zstd -T0 | ssh <host> 'zstd -d | docker load'`.
    -   This is typically faster than pushing to a registry and pulling back down, and keeps your data strictly P2P.

### Strategy A: Smart Swap (Hybrid / Default)

Best for rapid dev loops or simple apps. Minimal downtime (seconds).

1.  **Push**: Image streamed to server.
2.  **Rename**: `app` -> `app_old`.
3.  **Start**: New `app` container starts.
4.  **Health Check**: Arcane polls status for 5s.
    -   **Success**: `app_old` is removed.
    -   **Failure**: `app` is killed, `app_old` is restored. (Boomerang Rollback)

### Strategy B: Zero Downtime (Blue/Green)

Best for production APIs. Zero dropped connections. Requires **Caddy** on the server.
Trigger: `arcane push --ports 8001,8002`

1.  **Toggle**: Arcane detects if Blue (8001) or Green (8002) is running.
2.  **Deploy**: Pushes code to the _inactive_ slot (e.g., Green).
3.  **Verify**: Health check on Green.
4.  **Swap**: Arcane executes `sed -i` on `/etc/caddy/Caddyfile` to swap the upstream port and reloads Caddy.
5.  **Kill**: The old Blue container is removed.

---

## 4. Runtime Decryption & The "Secretless Server"

In all cases, **The Server is Stateless**.

1.  **No `arcane` binary** is needed on the remote server.
2.  **No `~/.arcane/keys`** are needed on the remote server.
3.  **No `.env` files** exist on the remote server disk.

Secrets are decrypted locally (by the Laptop or CI Runner) and injected directly into the container's environment variables (RAM) via the SSH process.

> **Benefit**: If your server is compromised, there are no keys to steal from the disk. The secrets exist only inside the running container's memory.

---

## 5. Technical Feature Roadmap

### Completed (v0.2.0)

-   [x] **Fleet Visibility**: TUI Ops Tab, Real-time stats.
-   [x] **Remote Control**: SSH wrappers, Remote Docker socket usage.
-   [x] **Direct Push**: "Warp Drive" Zstd pipelines.
-   [x] **Resilience**: Boomerang Rollback, Distributed Locking.
-   [x] **Enterprise**: Zero Downtime (Caddy Blue/Green).

### Next Steps (v0.3.0)

-   [ ] **Agentic AI**: Log stream analysis and auto-healing.
-   [ ] **Multi-Server**: Deploy to staging AND production from same TUI command.

---

## 6. The Business Case: "The Sovereign License"

**How we get paid without selling data or forcing SaaS:**

We sell **Cryptographic License Keys** (Ed25519 Signed).

-   **Offline Validation**: The binary verifies the signature locally. No "license server" ping. No telemetry.
-   **The "WinRAR" Model**: You can use it, but "Pro" features strictly enforced.

### The Pricing Model: "Honor System & Corporate Compliance"

**Philosophy**: We do not gatekeep features. The "Free" version is identical to the "Pro" version.
We rely on the fact that **Companies cannot legally use unlicensed software**.

See `MONETIZATION_STRATEGY.md` for full details.
