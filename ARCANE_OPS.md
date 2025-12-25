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

### Directory Structure

```rust
arcane/
├── src/
│   ├── main.rs          // Entry point (CLI & TUI router)
│   ├── ops/             // [NEW] The Ops Module
│   │   ├── mod.rs       // Module definition
│   │   ├── config.rs    // servers.toml parser
│   │   ├── shell.rs     // Wrapper for local & remote shell commands
│   │   ├── deploy.rs    // "Direct Push" logic (The Engine)
│   │   └── monitor.rs   // Docker stats parser
│   └── ui/
│       ├── ops_view.rs  // [NEW] The TUI Tab (Key: '4')
```

> **Zero Bloat Strategy**:  
> We will NOT use `russh` or `bollard`.  
> We will wrap the system `ssh` and `docker` binaries.
>
> -   **Why?** SSH configuration is complex (`~/.ssh/config`, ProxyJump, Agents). By using the system binary, we inherit all of that for free.
> -   **Benefit**: 0 extra dependencies. 100% compatibility with your existing workflow.

---

## 3. The Deployment Workflow (Hybrid)

Arcane Ops supports both **Interactive (Dev)** and **Headless (CI)** deployment modes using the same engine.

### Scenario A: The "Sovereign Dev" (Laptop)

1.  **Code**: You commit changes locally.
2.  **Trigger**: You open Arcane TUI (`arcane dashboard`), go to **Ops**, press `[D]eploy`.
3.  **Action**:
    -   Your laptop builds the Docker image (or pulls it).
    -   Your laptop SSHs to the server.
    -   Your laptop injects secrets + code directly into RAM.
    -   **Result**: Zero latency loop. Perfect for rapid dev.

### Scenario B: The "Pusher Server" (CI/CD)

1.  **Code**: You merge PR to `main`.
2.  **Trigger**: GitHub Actions / Jenkins runs `arcane deploy push production`.
3.  **Action**:
    -   CI Runner authenticates via SSH.
    -   CI Runner builds/pulls artifacts.
    -   CI Runner SSHs to the target server.
    -   **Result**: Automated, auditable deployments using the exact same logic.

### Runtime Decryption & The "Secretless Server"

In both cases, **The Server is Stateless**.

1.  **No `arcane` binary** is needed on the remote server.
2.  **No `~/.arcane/keys`** are needed on the remote server.
3.  **No `.env` files** exist on the remote server disk.

Secrets are decrypted locally (by the Laptop or CI Runner) and injected directly into the container's environment variables (RAM) via the SSH process.

> **Benefit**: If your server is compromised, there are no keys to steal from the disk. The secrets exist only inside the running container's memory.

### Runtime Decryption (The Common Denominator)

In both cases, the server receives the `ARCANE_MACHINE_KEY` environment variable purely in memory. The container starts, and `deployment_entrypoint.sh` decrypts the config envs on boot.

> **Note**: The Server is "Dumb". It doesn't know _how_ to deploy. It just receives instructions from an authenticated Pusher (Laptop or CI).

---

## 4. Technical Feature Roadmap

### Phase 1: Fleet Visibility (The Viewer)

**Goal**: Real-time visibility into all containers across local and remote fleets.

#### Core Features

| Feature                 | Description                                                                     |
| :---------------------- | :------------------------------------------------------------------------------ |
| **Local Docker Socket** | Connect to `unix:///var/run/docker.sock` for instant local container visibility |
| **Container Overview**  | Name, Image, Status, Uptime, Port mappings                                      |
| **Resource Monitoring** | Live CPU/RAM/Network stats per container                                        |
| **Log Streaming**       | Real-time `docker logs -f` directly in TUI                                      |
| **Image Management**    | List, pull, prune images                                                        |

#### TUI Layout (Ops Tab)

```
┌─────────────────────────────────────────────────────────────────────┐
│  ARCANE OPS                                    [1]Secrets [2]Ops    │
├─────────────────────────────────────────────────────────────────────┤
│  Fleet: localhost                                    [S]erver ▾     │
├───────────────────────────────────────┬─────────────────────────────┤
│  CONTAINERS                           │  DETAILS: citadel           │
│  ─────────────────────────────────    │  ─────────────────────────  │
│  ▶ citadel        running   12h       │  Image: dracon/citadel:v2   │
│    igt-cloud      running   12h       │  Ports: 8080→8080, 50051    │
│    redis          running   12h       │  CPU: 2.3%  RAM: 128MB      │
│    payment-agent  running   12h       │                             │
│    hermes-agent   running   12h       │  ENV:                       │
│                                       │    ENVIRONMENT=production   │
│                                       │    RUST_LOG=info            │
├───────────────────────────────────────┴─────────────────────────────┤
│  [R]estart  [S]top  [L]ogs  [D]eploy  [Enter] Exec Shell            │
└─────────────────────────────────────────────────────────────────────┘
```

#### Implementation

```rust
// src/ops/docker.rs
pub struct LocalDockerClient {
    docker: bollard::Docker,
}

impl LocalDockerClient {
    pub async fn list_containers(&self) -> Vec<ContainerInfo>;
    pub async fn get_stats(&self, id: &str) -> ContainerStats;
    pub async fn stream_logs(&self, id: &str) -> impl Stream<Item = String>;
    pub async fn restart(&self, id: &str) -> Result<()>;
    pub async fn stop(&self, id: &str) -> Result<()>;
    pub async fn exec(&self, id: &str, cmd: &[&str]) -> Result<String>;
}
```

---

### Phase 2: Remote Fleet Control (The Deployer)

**Goal**: Manage containers on remote servers via SSH. Zero agents required.

#### Core Features

| Feature             | Description                                            |
| :------------------ | :----------------------------------------------------- |
| **Server Registry** | Store servers in `~/.arcane/servers.toml`              |
| **SSH Tunnel**      | Persistent connections via `russh` with key-based auth |
| **Remote Docker**   | Forward Docker socket over SSH tunnel                  |
| **Deploy Trigger**  | One-key deployment: Pull image → Stop old → Start new  |
| **Rollback**        | Automatic previous-image tracking, instant rollback    |
| **Multi-Server**    | Deploy to staging AND production from same TUI         |

#### Server Configuration

```toml
# ~/.arcane/servers.toml
[[servers]]
name = "production"
host = "prod.dracon.uk"
user = "deploy"
key_path = "~/.ssh/id_ed25519"
docker_socket = "/var/run/docker.sock"

[[servers]]
name = "staging"
host = "stage.dracon.uk"
user = "deploy"
key_path = "~/.ssh/id_ed25519"
```

#### Deploy Workflow

```
User presses [D]eploy on "citadel"
         │
         ▼
  ┌─────────────────────┐
  │  Select Target      │
  │  ○ staging          │
  │  ● production       │
  │  ○ both             │
  └─────────────────────┘
         │
         ▼
  ┌─────────────────────┐
  │  Select Image       │
  │  ● :latest          │
  │  ○ :v2.1.0          │
  │  ○ :v2.0.0          │
  └─────────────────────┘
         │
         ▼
         ▼
  SSH → docker pull → docker stop → docker run (with Environment Variables injected)
         │
         ▼
       SUCCESS ✓
```

#### Implementation

```rust
// src/ops/ssh.rs
pub struct SshManager {
    sessions: HashMap<String, SshSession>,
}

impl SshManager {
    pub async fn connect(&mut self, server: &ServerConfig) -> Result<()>;
    pub async fn exec(&self, server: &str, cmd: &str) -> Result<String>;
    pub async fn forward_docker(&self, server: &str) -> RemoteDockerClient;
}
```

---

### Phase 3: Intelligent Operations (The Agent)

**Goal**: AI-powered auto-healing and predictive maintenance.

#### Core Features

| Feature                   | Description                                      |
| :------------------------ | :----------------------------------------------- |
| **Log Analysis**          | Stream logs → AI → Actionable insights           |
| **Anomaly Detection**     | "Container restarted 5 times in 1 hour" alerts   |
| **Auto-Heal Rules**       | Configurable triggers: CPU > 90% → restart       |
| **Crash Analysis**        | Parse panic traces, suggest fixes                |
| **Smart Port Management** | Auto-detect port conflicts, suggest alternatives |

#### Auto-Heal Configuration

```toml
# ~/.arcane/agent.toml
[[rules]]
name = "High CPU Restart"
container = "citadel"
condition = "cpu > 90% for 5m"
action = "restart"
cooldown = "15m"

[[rules]]
name = "Memory Limit"
container = "*"
condition = "memory > 1GB"
action = "alert"
```

#### AI Integration

```rust
// src/ops/agent.rs
pub struct OpsAgent {
    ai_client: AIClient,
    rules: Vec<HealRule>,
}

impl OpsAgent {
    pub async fn analyze_logs(&self, logs: &str) -> AnalysisResult;
    pub async fn suggest_fix(&self, panic: &PanicTrace) -> Option<String>;
    pub async fn check_rules(&self, stats: &ContainerStats) -> Vec<Action>;
}
```

#### Smart Port Feature (User Requested)

When deploying, Arcane Ops will:

1. Scan target host for used ports
2. Detect conflicts with new container
3. Auto-suggest alternative port mappings
4. Update proxy config (Caddy/Nginx) if integrated

---

## 5. The Business Case: "The Sovereign License"

**How we get paid without selling data or forcing SaaS:**

We sell **Cryptographic License Keys** (Ed25519 Signed).

-   **Offline Validation**: The binary verifies the signature locally. No "license server" ping. No telemetry.
-   **The "WinRAR" Model**: You can use it, but "Pro" features strictly enforced.

### The Pricing Model: "Honor System & Corporate Compliance"

**Philosophy**: We do not gatekeep features. The "Free" version is identical to the "Pro" version.
We rely on the fact that **Companies cannot legally use unlicensed software**.

| Feature         | Everyone (Free & Paid)                |
| :-------------- | :------------------------------------ |
| **Servers**     | **Unlimited**                         |
| **Containers**  | **Unlimited**                         |
| **Agentic Ops** | **Unlimited** (Auto-Healing included) |
| **RBAC**        | **Unlimited**                         |

### Who Must Pay? (The Commercial License)

Aligned with `MONETIZATION_STRATEGY.md`.

| Company Size          | Annual License        |
| :-------------------- | :-------------------- |
| **< 5 Employees**     | **$0 (Free Forever)** |
| **5-25 Employees**    | **$2,000/yr**         |
| **26-100 Employees**  | **$5,000/yr**         |
| **101-500 Employees** | **$10,000/yr**        |
| **500+ Employees**    | **$20,000/yr**        |

**The "WinRAR" Factor:**
You can download Arcane, run it on 50 servers, and use the AI Agent. It will just say "Unlicensed Copy" in the corner unless you add a key.

-   **Indie Devs**: Love us for giving them power.
-   **Companies**: Buy the license to stay compliant.

### The "Arcane Intelligence" Add-On

For users who want the "Agent" features (AI log analysis, auto-fix suggestions) but are on the Free tier (or want to support us).

-   **Price**: **$5/month**.
-   **Feature**: Unlocks the `Agent` tab for AI-driven debugging.

### The Revenue Math

-   **100 Small Companies** @ $2,000 = **$200,000 ARR**.
-   **50 Mid Companies** @ $5,000 = **$250,000 ARR**.
-   **20 Large Companies** @ $10,000 = **$200,000 ARR**.
-   **10 Enterprise** @ $20,000 = **$200,000 ARR**.
-   **2,000 IndieDevs** @ $5/mo = **$120,000 ARR**.
-   **Total Potential**: **~$970k ARR** (Conservative estimate).

**Why this works:**

1.  **Sovereignty**: Users pay a premium to _avoid_ SaaS dependencies.
2.  **Trust**: "We can't turn off your software" is a massive selling point in 2025.
3.  **B2B**: Agencies purchase "Pro" immediately to manage client fleets.

---

## 6. Verification Strategy

How do we know it works?

1.  **Local Test**: Run `arcane` -> Ops -> Connect to Local Docker. Verify we see `chimera` running.
2.  **Remote Test**: Spin up a fresh VPS. Add it to Arcane. Verify we can `[R]estart` a container over SSH.
3.  **Deploy Test**: Trigger a full deploy from the TUI and verify `ARCANE_MACHINE_KEY` is passed correctly.
