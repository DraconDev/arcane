#!/bin/bash
# ⚔️ Arcane Sovereign Server Setup
# Usage: ssh root@<server> "bash -s" < scripts/setup_server.sh
# 
# Installs:
# 1. Docker (The Runtime)
# 2. Caddy (The Router/Load Balancer)
# 3. Zstd (The Warp Drive)
# 4. /var/lock/arcane.deploy (The Locking Mechanism)

set -e

echo "🔮 Initiating Sovereign Server Setup..."

# 1. Update & Prereqs
export DEBIAN_FRONTEND=noninteractive
apt-get update && apt-get upgrade -y
apt-get install -y curl wget git zstd sudo

# 2. Install Docker
if ! command -v docker &> /dev/null; then
    echo "🐳 Installing Docker..."
    curl -fsSL https://get.docker.com | sh
    systemctl enable --now docker
else
    echo "✅ Docker already installed."
fi

# 3. Install Caddy
if ! command -v caddy &> /dev/null; then
    echo "🌐 Installing Caddy..."
    apt-get install -y debian-keyring debian-archive-keyring apt-transport-https
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
    apt-get update
    apt-get install -y caddy
    systemctl enable --now caddy
else
    echo "✅ Caddy already installed."
fi

# 4. Configure Locking Directory
mkdir -p /var/lock
chmod 777 /var/lock # Allow Arcane to mkdir inside (if running as non-root user, adjust this!)

# 5. Setup User (Optional - if you want a dedicated 'deploy' user)
# Not strictly enforced by script, but recommended.

echo "✨ Server Provisioned!"
echo "   - Runtime: Docker ✅"
echo "   - Router:  Caddy ✅"
echo "   - Push:    Zstd ✅"
echo ""
echo "You are ready to deploy."
