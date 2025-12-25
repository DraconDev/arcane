# Build Stage
FROM rust:latest AS builder

WORKDIR /usr/src/arcane
COPY . .

# Build Arcane (Root workspace)
RUN cargo install --path . --root /usr/src/arcane/dist

# Build Demo App
WORKDIR /usr/src/arcane/examples/secrets-demo
RUN cargo build --release

# Runtime Stage
FROM ubuntu:24.04

# Install basic deps (SSL needed for many Rust apps)
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries
COPY --from=builder /usr/src/arcane/dist/bin/arcane /usr/local/bin/arcane
COPY --from=builder /usr/src/arcane/examples/secrets-demo/target/release/secrets-demo /app/secrets-demo

# Copy encrypted .env (auto-encrypted by Git filter in .gitattributes)
COPY --from=builder /usr/src/arcane/examples/secrets-demo/.env .env

# Copy Arcane keys (committed in arcane-keys/ for demo purposes)
# In production, these would be in .git/arcane/ (not tracked by git itself)
RUN mkdir -p .git/arcane
COPY --from=builder /usr/src/arcane/examples/secrets-demo/arcane-keys /app/.git/arcane

# Set executable perms
RUN chmod +x /usr/local/bin/arcane

# Expose port
EXPOSE 5122

# Run with Arcane Wrapper
CMD ["arcane", "run", "--", "./secrets-demo"]
