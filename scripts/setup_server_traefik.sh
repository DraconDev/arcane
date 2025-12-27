#!/bin/bash
# ⚔️ Arcane Sovereign Server Setup (Traefik Edition)
# Usage: ssh root@<server> "bash -s" < scripts/setup_server_traefik.sh
# 
# Installs:
# 1. Docker (The Runtime)
# 2. Traefik (Auto-discovery Reverse Proxy)
# 3. Zstd (The Warp Drive)
# 4. /var/lock/arcane.deploy (The Locking Mechanism)
#
# Why Traefik over Caddy?
# - Docker label-based routing (no config files to update per deploy)
# - Automatic service discovery
# - Built-in dashboard
# - Better for multi-service architectures

set -e

echo "🔮 Initiating Sovereign Server Setup (Traefik Edition)..."

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

# 3. Create Traefik network (all services must join this)
if ! docker network inspect traefik-public &> /dev/null; then
    echo "🌐 Creating traefik-public network..."
    docker network create traefik-public
else
    echo "✅ traefik-public network exists."
fi

# 4. Create Traefik config directory
mkdir -p /opt/traefik
mkdir -p /opt/traefik/acme

# 5. Write Traefik static config
# We support an optional 'dns-resolver' for Wildcard certs if Cloudflare keys are provided
cat > /opt/traefik/traefik.yml << EOF
# Traefik Static Configuration
api:
  dashboard: true
  insecure: true

entryPoints:
  web:
    address: ":80"
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https
  websecure:
    address: ":443"

providers:
  docker:
    endpoint: "unix:///var/run/docker.sock"
    exposedByDefault: false
    network: traefik-public

certificatesResolvers:
  letsencrypt:
    acme:
      email: ${ADMIN_EMAIL:-admin@dracon.uk}
      storage: /acme/acme.json
      httpChallenge:
        entryPoint: web
EOF

# Add DNS-01 Resolver if Cloudflare is configured
if [[ -n "$CF_API_KEY" ]]; then
    echo "☁️  Configuring Cloudflare DNS-01 Resolver..."
    cat >> /opt/traefik/traefik.yml << EOF
  dns-resolver:
    acme:
      email: ${ADMIN_EMAIL:-admin@dracon.uk}
      storage: /acme/acme.json
      dnsChallenge:
        provider: cloudflare
        resolvers:
          - "1.1.1.1:53"
          - "8.8.8.8:53"
EOF
fi

# 6. Create acme.json with correct permissions
touch /opt/traefik/acme/acme.json
chmod 600 /opt/traefik/acme/acme.json

# 7. Start Traefik container
if docker ps -a --format '{{.Names}}' | grep -q '^traefik$'; then
    echo "🔄 Restarting Traefik..."
    docker rm -f traefik
fi

# Build Docker Run command with optional DNS env vars
TRAEFIK_OPTS=(
    -d
    --name traefik
    --restart unless-stopped
    --network traefik-public
    -p 80:80
    -p 443:443
    -p 8080:8080
    -v /var/run/docker.sock:/var/run/docker.sock:ro
    -v /opt/traefik/traefik.yml:/traefik.yml:ro
    -v /opt/traefik/acme:/acme
)

if [[ -n "$CF_API_EMAIL" ]]; then
    TRAEFIK_OPTS+=(-e "CF_API_EMAIL=$CF_API_EMAIL")
fi
if [[ -n "$CF_API_KEY" ]]; then
    TRAEFIK_OPTS+=(-e "CF_API_KEY=$CF_API_KEY")
fi
if [[ -n "$CF_DNS_API_TOKEN" ]]; then
    TRAEFIK_OPTS+=(-e "CF_DNS_API_TOKEN=$CF_DNS_API_TOKEN")
fi

echo "🚀 Starting Traefik (v3)..."
docker run "${TRAEFIK_OPTS[@]}" traefik:v3.2

# 8. Configure Locking Directory
mkdir -p /var/lock
chmod 777 /var/lock

# 9. Verify
sleep 3
if docker ps | grep -q traefik; then
    echo ""
    echo "✨ Server Provisioned (Traefik Edition)!"
    echo "   - Runtime:   Docker ✅"
    echo "   - Router:    Traefik v3 ✅"
    echo "   - Dashboard: http://<server-ip>:8080 ✅"
    echo "   - Push:      Zstd ✅"
    echo ""
    echo "🏷️  To route a service, add these labels to your compose.yml:"
    echo ""
    echo '  labels:'
    echo '    - "traefik.enable=true"'
    echo '    - "traefik.http.routers.myapp.rule=Host(`myapp.dracon.uk`)"'
    echo '    - "traefik.http.routers.myapp.entrypoints=websecure"'
    echo '    - "traefik.http.routers.myapp.tls.certresolver=letsencrypt"'
    echo '    - "traefik.http.services.myapp.loadbalancer.server.port=3000"'
    echo ""
    echo "  networks:"
    echo "    - traefik-public"
    echo ""
    echo "networks:"
    echo "  traefik-public:"
    echo "    external: true"
    echo ""
    echo "You are ready to deploy."
else
    echo "❌ Traefik failed to start. Check: docker logs traefik"
    exit 1
fi
