# Arcane Deployment Guide 🚀

> "The direct path is often the most sovereign."

Arcane provides a powerful, agentless deployment system that encrypts secrets locally and pushes everything through SSH tunnels. No cloud registry, no CD pipeline, no Kubernetes. Just you and your servers.

---

## 🏗️ Core Philosophies

1.  **Garage Mode (Push-to-Deploy)**: We build locally, test locally, and stream binaries/images directly to the server. Secrets are decrypted in RAM only.
2.  **Environment Isolation**: Separate credentials for Staging and Production, handled seamlessly via `config/envs/`.
3.  **Zero Trust**: Servers only see the secrets they need, at the moment of deployment.

---

## 🛠️ Usage

### 1. Single Image Deployment (Garage Mode)

Ideal for simple microservices or monoliths. Supports Zero Downtime.

```bash
# Basic Deploy (Staging default)
arcane deploy --target micro1 --app chimera

# Production Deploy (Prompts for confirmation)
arcane deploy --target micro1 --app chimera --env production
```

**Workflow:**

1.  **Build**: `docker build` locally.
2.  **Smoke Test**: Runs transient container to verify boot.
3.  **Warp Drive**: Streams image via `zstd | ssh | docker load`.
4.  **Encrypt/Inject**: Decrypts `.env` and passes vars to container securely.
5.  **Swap**: Hot-swaps the container.

### 2. Docker Compose Deployment (Stack Mode)

For complex stacks (e.g. App + Redis + Sidecar).

```bash
arcane deploy --target micro1 --compose docker-compose.yaml --env production
```

**Workflow:**

1.  **Generate**: Decrypts env vars to a temporary `.env` on remote.
2.  **Upload**: SCPs plain `docker-compose.yaml` and the generated `.env`.
3.  **Execute**: `docker compose up -d --remove-orphans` (Rolling Update).

### 3. Server Groups (Mass Deployment)

Deploy to clusters defined in specific groups.

```bash
# In ~/.arcane/servers.toml:
# [[groups]]
# name = "web-cluster"
# servers = ["web1", "web2", "web3"]

# Deploy to all concurrently (Max 4 parallel)
arcane deploy --target web-cluster --parallel
```

### 4. Simulation (Dry Run)

Validate configuration and keys without touching the server.

```bash
arcane deploy --target micro1 --dry-run
```

Output:

```
[DRY RUN] Decryption successful. Loaded 12 variables.
[DRY RUN] Would hold lock.
[DRY RUN] Would build image...
```

---

## 🌍 Environment Management

Arcane uses a layered configuration system in `project_root/config/envs/`:

-   `base.env`: Shared defaults (e.g. `PORT=3000`). Commit as plaintext.
-   `staging.env`: Staging secrets. **Encrypted** by Arcane.
-   `production.env`: Production secrets. **Encrypted** by Arcane.

Use `arcane init` to set up encryption keys.

---

## 🚦 Zero Downtime Strategies

### Standard (Rename Swap)

Default for single images.

1.  Start `new_container`.
2.  Wait for health check.
3.  Stop `old_container`.
4.  If fail: Kill `new`, keep `old` running.

### Blue/Green (Caddy)

Requires `--ports 8001,8002`.

1.  Deploy to inactive color (Green).
2.  Verify health.
3.  Update Caddy upstream to Green.
4.  Kill Blue.

---

## 📡 Observability

Arcane provides direct SSH-based observability tools to inspect your running deployments without leaving your terminal.

### Remote Logs

Stream logs from any container on any managed server.

```bash
# Stream logs (follow)
arcane logs micro1 -f

# Show last 100 lines
arcane logs micro1 -n 100

# Specify app/container name (Default is 'app')
arcane logs micro1 --app redis -f
```

### Interactive Shell

Open a secure, interactive shell session inside a running container.

```bash
# Bash session
arcane exec micro1 -- /bin/bash

# Run a one-off command (e.g., Rails console/migration)
arcane exec micro1 -- rails db:migrate
```

### Dry Run

All observability commands support `--dry-run` to see exactly what SSH command would be executed.

```bash
arcane exec micro1 --dry-run -- rm -rf /
# Output: [DRY RUN] Would SSH to ... and run: 'docker exec -it app rm -rf /'
```

---

## 🧩 Troubleshooting

-   **"Upload is slow"**: Check `.dockerignore`. Exclude `target/`, `node_modules/`, and `.git/`.
-   **"SSH Error"**: Ensure your SSH agent has the key loaded (`ssh-add ~/.ssh/id_ed25519`).
-   **"Deployment Locked"**: Arcane places a lock directory on the server. If a deploy crashes, you may need to manually run `rmdir /var/lock/arcane.deploy` on the server.
