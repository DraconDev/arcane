# Sovereign Guardian (Auto-Init Daemon)

The **Auto-Init Daemon** ensures "Secure by Default" behavior for your development environment.

It watches your configured directories (e.g., `~/_Dev`) and automatically secures any new Git repository you create:

1.  **Injects `.gitattributes`**: Adds the `filter=git-arcane` config.
2.  **Initializes Arcane**: Generates the repository's `owner.age` key.

This eliminates the risk of accidentally committing plaintext secrets before configuring Arcane.

## Setup

### 1. Configure

Tell the daemon which root directory to watch. It will watch recursively.

```bash
arcane daemon config --add ~/_Dev
```

### 2. Run (Manual)

To test it, run in a terminal:

```bash
arcane daemon run
```

### 3. Run (Automatic)

For a true "set and forget" experience, run the daemon as a background service on login.

#### Linux (Systemd)

1.  Create the service file: `~/.config/systemd/user/arcane.service`

    ```ini
    [Unit]
    Description=Arcane Sovereign Guardian
    After=network.target

    [Service]
    ExecStart=%h/.cargo/bin/arcane daemon run
    Restart=on-failure
    Environment=RUST_LOG=info

    [Install]
    WantedBy=default.target
    ```

2.  Enable and start:

    ```bash
    systemctl --user enable --now arcane
    ```

3.  Check status:

    ```bash
    systemctl --user status arcane
    ```

## Troubleshooting

### "Failed to watch" Error

If the daemon crashes with this error, you likely hit the Linux `inotify` limit (default is 8192).

**Fix**: Increase the limit permanently.

```bash
echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```
