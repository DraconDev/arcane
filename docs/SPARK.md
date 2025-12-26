# Arcane Spark Guide ⚡

**Arcane Spark** is a self-hosted build server and webhook listener. It allows you to trigger deployments automatically when you push code to GitHub, keeping your codebase verifying signatures and executing deploys from your own infrastructure.

---

## 🚀 Quick Start

### 1. Prerequisite

You need Arcane installed on a server that has:

1.  **Public IP** (to receive webhooks).
2.  **SSH Access** to your target servers (or be the target server itself).
3.  **Arcane Config** (`servers.toml`, identity key) set up.

### 2. Configuration (`spark.toml`)

Create a `spark.toml` file in your Arcane root (or wherever you run the daemon):

```toml
# spark.toml

# Define which repos to listen for
[[repos]]
name = "arcane"                     # Matches repo name in GitHub URL
url = "https://github.com/DraconDev/arcane"
branch = "main"                     # Branch to deploy
deploy_target = "micro1"            # Server alias from servers.toml
env = "micro1"                      # Environment file (config/envs/micro1.env)
```

### 3. Start Spark

Run the daemon:

```bash
# Using a flag for the secret (good for testing)
arcane spark start --port 7777 --secret "<SECRET>"

# Using an env var (better for production)
export SPARK_WEBHOOK_SECRET="<SECRET>"
arcane spark start --port 7777
```

### 4. Configure GitHub Webhook

1.  Go to your Repository on GitHub.
2.  **Settings** -> **Webhooks** -> **Add webhook**.
3.  **Payload URL**: `http://<YOUR_SERVER_IP>:7777/webhook`
4.  **Content type**: `application/json`
5.  **Secret**: `<SECRET>` (Must match above!)
6.  **Events**: Just "Push" events.
7.  Click **Add webhook**.

---

## 🛠️ Production Deployment (Systemd)

To run Spark as a background service on Linux:

1.  **Create unit file**: `/etc/systemd/system/arcane-spark.service`

```ini
[Unit]
Description=Arcane Spark Webhook Listener
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/arcane
ExecStart=/home/ubuntu/.cargo/bin/arcane spark start --port 7777
Environment="SPARK_WEBHOOK_SECRET=<SECRET>"
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

2.  **Enable and Start**:
    ```bash
    sudo systemctl daemon-reload
    sudo systemctl enable arcane-spark
    sudo systemctl start arcane-spark
    sudo systemctl status arcane-spark
    ```

---

## 🧠 How It Works (Internals)

1.  **Debounce**: Spark waits **10 seconds** after a push before starting a build. If a new push arrives in that window, the timer resets.
2.  **Latest Wins**: If a build is currently running and a new push arrives, Spark **cancels** the current build and queues the new one. This prevents "queue clogging" with obsolete commits.
3.  **Isolation**: Builds for different repositories run in parallel and do not affect each other.
4.  **Security**:
    -   **HMAC-SHA256**: Every request is cryptographically signed by GitHub. Spark drops any request without a valid signature.
    -   **Whitelist**: Spark only acts on repos explicitly defined in `spark.toml`.

---

## 🔍 Troubleshooting

**"Repo 'xyz' not in whitelist, ignoring"**

-   Add the repo to your `spark.toml`.
-   Ensure the `name` matches the GitHub repo name (last part of URL).

**"Invalid webhook signature"**

-   Check that `SPARK_WEBHOOK_SECRET` matches the secret in GitHub settings.

**Deploy fails**

-   Check `arcane deploy` works manually first.
-   Ensure the user running Spark has SSH access to the target.
