# GitHub Actions Integration

Deploy automatically when you push to main, using GitHub as your build server.

## Prerequisites

1. You have `servers.toml` configured in your repo
2. You have encrypted environment files in `config/envs/`
3. Your server is authorized via `arcane deploy allow <key>`

## Step 1: Generate a Machine Key

On your local machine:

```bash
arcane deploy gen-key
```

Output:

```
🤖 Generated Machine Identity

Public Key (Authorize this):
age1abc123...

Private Key (Set as ARCANE_MACHINE_KEY):
AGE-SECRET-KEY-1xyz789...
```

## Step 2: Authorize the Key

```bash
arcane deploy allow age1abc123...
git add .git/arcane && git commit -m "Authorize GitHub Actions" && git push
```

## Step 3: Add Secret to GitHub

1. Go to your repo → Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `ARCANE_MACHINE_KEY`
4. Value: `AGE-SECRET-KEY-1xyz789...` (the private key from Step 1)

## Step 4: Create Workflow

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy

on:
    push:
        branches: [main]

jobs:
    deploy:
        runs-on: ubuntu-latest

        steps:
            - uses: actions/checkout@v4

            - name: Install Rust
              uses: dtolnay/rust-action@stable

            - name: Install Arcane
              run: cargo install --git https://github.com/DraconDev/arcane

            - name: Deploy to Production
              env:
                  ARCANE_MACHINE_KEY: ${{ secrets.ARCANE_MACHINE_KEY }}
              run: arcane deploy --target production --env production
```

## How It Works

```
git push origin main
       ↓
GitHub Actions triggers
       ↓
Checkout code (with encrypted secrets)
       ↓
Install Arcane
       ↓
ARCANE_MACHINE_KEY decrypts secrets
       ↓
arcane deploy → SSH to your server
       ↓
Containers running ✅
```

## Comparison: GitHub Actions vs Spark vs Manual

| Factor   | GitHub Actions      | Spark (Self-Hosted)   | Manual         |
| -------- | ------------------- | --------------------- | -------------- |
| Speed    | 🐢 2-5 min          | ⚡ 10-30s             | ⚡ Instant     |
| Privacy  | ⚠️ Shared runners   | ✅ Your infra         | ✅ Your laptop |
| Setup    | Per-repo YAML       | Once, forever         | None           |
| Cost     | Free (2000 min/mo)  | Free (Oracle tier)    | Free           |
| Use Case | Public repos, CI/CD | Private, max velocity | Emergencies    |

## When to Use Each

-   **GitHub Actions**: You want automation without running infrastructure
-   **Spark**: You need speed + privacy for private repos
-   **Manual (`arcane deploy`)**: Crisis response, quick iterations

## Tips

### Skip Deployment on Certain Commits

```yaml
on:
    push:
        branches: [main]
        paths-ignore:
            - "**.md"
            - "docs/**"
```

### Deploy to Staging First

```yaml
jobs:
    deploy-staging:
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v4
            - run: cargo install --git https://github.com/DraconDev/arcane
            - run: arcane deploy --target staging --env staging
              env:
                  ARCANE_MACHINE_KEY: ${{ secrets.ARCANE_MACHINE_KEY }}

    deploy-production:
        needs: deploy-staging
        runs-on: ubuntu-latest
        environment: production # Requires approval
        steps:
            - uses: actions/checkout@v4
            - run: cargo install --git https://github.com/DraconDev/arcane
            - run: arcane deploy --target production --env production
              env:
                  ARCANE_MACHINE_KEY: ${{ secrets.ARCANE_MACHINE_KEY }}
```

### Cache Arcane Installation

```yaml
- uses: actions/cache@v3
  with:
      path: ~/.cargo
      key: ${{ runner.os }}-cargo-arcane

- name: Install Arcane
  run: cargo install --git https://github.com/DraconDev/arcane
```
