# Sovereign Deployment (Garage Mode) 🚀

> "The direct path is often the most sovereign."

Arcane implements a deployment strategy known as **Garage Mode**. Unlike modern DevOps pipelines that rely on complex chains of third-party services (GitHub Actions -> Docker Hub -> Kubernetes), Arcane believes in **Direct Action**.

## The Philosophy: Caveman or Genius?

It feels primitive because it uses raw tools (`ssh`, `tar`, `zstd`, `docker`).
It is genius because it removes all "Rent-Seeking" middlemen.

**The Pipeline:**

1.  **You** (Laptop) -> **Tunnel** (SSH) -> **Production** (VPS)

**Benefits:**

-   **Zero Rent**: No CI minutes, no private registry fees.
-   **Zero Leakage**: Code and secrets never leave your encrypted tunnel.
-   **Zero Downtime**: If GitHub goes down, you can still ship.

---

## 🏗️ The Garage Workflow

When you run `arcane deploy`, the following pipeline executes entirely on your machine (and the target):

### 1. Local Build

Arcane builds your Docker image locally using your `Dockerfile`.

```bash
docker build -t <app>:latest .
```

### 2. Smoke Test (The "Ignition Check")

Before uploading, Arcane runs a transient container for 3 seconds to ensure it doesn't crash on boot.

-   If it dies? **Abort**. (Bad code never leaves your laptop).
-   If it lives? **Proceed**.

### 3. Ram Injection (Secrets)

Arcane decrypts your local `.env` file (or uses the plaintext one if you are in development) and injects the secrets directly into the simple deployment command.

-   **Security Note**: Secrets are NEVER baked into the image. They exist only in the RAM of the running container.

### 4. Warp Drive (The Transport)

We don't "push" to a registry. We stream the image directly to the server.

```bash
docker save <image> | zstd -T0 -3 | ssh <host> 'zstd -d | docker load'
```

-   **Speed**: Zstd compression makes this comparable to (or faster than) a registry pull, especially for updates (Docker layers).
-   **Note**: Ensure your `.dockerignore` excludes `target/` and `.git/` to keep uploads fast!

### 5. Smart Swap (Zero Downtime)

Once the image is on the server, Arcane performs a **Hot Swap**:

#### Standard Mode (Renaming)

1.  Rename existing container `app` -> `app_old`.
2.  Start new container `app` (with new secrets).
3.  Health Check (HTTP/Docker Status).
4.  If Healthy: Kill `app_old`.
5.  If Failed: Kill `app`, Rename `app_old` -> `app`. (Instant Rollback).

#### Enterprise Mode (Caddy / Blue-Green)

If you provide `--ports 8001,8002`, Arcane orchestrates a Blue/Green deploy with Caddy.

1.  Determine which color is active (Blue).
2.  Deploy to Green.
3.  Verify Health.
4.  Update Caddyfile (`sed -i ...`) to point to Green.
5.  Reload Caddy (Traffic swaps instantly).
6.  Kill Blue.

---

## Usage

### 1. Prerequisites

-   A `Dockerfile` in your repo root.
-   A server configured in `~/.arcane/servers.toml`.
-   (Optional) An `.env` file in `config/envs/<server_name>.env`.

### 2. Deploy

```bash
# Deploy 'chimera' app to 'micro1' server
arcane deploy -t micro1 -a chimera
```

### 3. Deploy with Zero Downtime (Blue/Green)

```bash
# Deploy 'chimera' using ports 8001 and 8002 for traffic swapping
arcane deploy -t micro1 -a chimera --ports 8001,8002
```

---

## Troubleshooting

-   **"Upload is slow"**: Check your `.dockerignore`. You usually want to ignore `target/` and `.git/`.
-   **"Decryption failed"**: If your `.env` file is plain text, Arcane will auto-detect and use it (logging a warning).
-   **"Smoke Test Failed"**: Check `docker logs` output. Your container likely crashed immediately (missing env var?).
