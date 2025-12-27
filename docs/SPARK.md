# Arcane Spark ⚡

> **Self-Hosted, Push-to-Deploy Build Server.**

Arcane Spark is a lightweight webhook daemon that listens for GitHub push events and triggers `arcane deploy` automatically. It sits on your server (or a build box) and eliminates the need for external CI/CD services like GitHub Actions or CircleCI for simple deployment workflows.

## 🚀 Features

-   **Zero Config**: Works out of the box with standard GitHub webhooks.
-   **Security**: Verifies HMAC signatures (optional but recommended) and whitelists repositories.
-   **Debounce**: Intelligent 10s debounce to batch rapid commits.
-   **Latest-Wins**: Automatically ignores older builds if a new commit arrives during the wait period.
-   **Auto-Ingress**: Automatically injects Traefik labels into `compose.yml` deployments for instant routing.
-   **Status Reporting**: Reports build status (pending/success/failure) back to the GitHub Commit UI.

---

## 🛠️ Setup

### 1. Configuration (`spark.toml`)

Create a `spark.toml` file in the directory where you run Arcane:

```toml
[repos.my-project]
name = "my-project"
url = "https://github.com/me/my-project.git"
url_secret = "my-secret-token" # Optional: Matches GitHub Webhook Secret
```

### 2. Run the Server

```bash
# Start listening on port 7777 (default)
arcane spark
```

### 3. Usage with GitHub

1.  Go to your GitHub Repo -> Settings -> Webhooks -> Add webhook.
2.  **Payload URL**: `http://your-server-ip:7777/webhook`
3.  **Content type**: `application/json`
4.  **Secret**: (Same as `url_secret` in toml)
5.  **Events**: Just "Push" events.

---

## 📦 How It Works

1.  **Webhook Received**: Spark receives a JSON payload from GitHub.
2.  **Verify**: Checks if the repo is in `spark.toml` and (optionally) verifies the HMAC signature.
3.  **Clone/Pull**: Updates a local mirror of the repository in `~/.arcane/spark/repos/`.
4.  **Deploy**: Runs `arcane deploy` targeting the **local server** (or configured target).
    -   If `compose.yml` exists, it runs with `--compose` and `--auto-ingress`.
    -   If not, it attempts a single image build (Garage Mode).
5.  **Report**: Updates GitHub commit status to "✅ Success" or "❌ Failure".

---

## 🌐 Auto-Ingress

When deploying Docker Compose projects via Spark, it automatically passes the `--auto-ingress` flag. This leverages Arcane's core ability to inject Traefik V2 labels on the fly:

-   **Host Rule**: `Host(<repo-name>.dracon.uk)`
-   **TLS**: `certresolver=letsencrypt`
-   **Networking**: Connects to `traefik-public`

This means you can drop a standard `compose.yml` into your repo, push to GitHub, and have a fully secured, HTTPS-enabled endpoint in seconds.
