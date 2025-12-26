# Why Arcane?

## The Problem Everyone Has

Every company with more than one developer faces the same security nightmare:

**Developers have production secrets on their laptops.**

-   `.env.production` files everywhere
-   AWS keys, Stripe keys, database passwords
-   Copied, leaked, taken when they leave
-   "Key rotation" is theater - they already saw the secrets

Most "secret management" solutions don't actually solve this:

| Tool                | Can Developers Read Prod Secrets? |
| ------------------- | --------------------------------- |
| HashiCorp Vault     | ✅ Yes, if they have permissions  |
| AWS Secrets Manager | ✅ Yes, if they have IAM access   |
| Doppler             | ✅ Yes, if they're in the project |
| 1Password           | ✅ Yes, if they're in the vault   |
| SOPS                | ✅ Yes, if they're a recipient    |
| git-crypt           | ✅ Yes, if they have the key      |

---

## Arcane's Answer: Zero Developer Access

With Arcane + Build Server (Spark):

| Entity             | Access to Prod Secrets?                 |
| ------------------ | --------------------------------------- |
| Developer laptops  | ❌ **NO ACCESS**                        |
| Build server       | ✅ Yes (has machine key)                |
| Production servers | ✅ Yes (receives secrets during deploy) |

### How It Works

1. **Secrets are encrypted** in the repo (`.env.production.age`)
2. **Only the build server** has the machine key to decrypt
3. **Developers push code** → Build server deploys
4. **Secrets never exist** on developer machines in any form

### What Developers CAN'T Do

```bash
# Can't decrypt the file
cat .env.production.age  # Encrypted gibberish

# Can't deploy to prod
arcane deploy --env production
# Error: No machine key found. Cannot decrypt secrets.

# Can't use their identity
ARCANE_MACHINE_KEY=$MY_KEY arcane deploy --env production
# Error: Key not authorized for this repo.
```

### What Developers CAN Do

```bash
# Push code
git push origin main

# Work with staging (if you give them staging access)
arcane deploy --env staging  # Works if they have staging key
```

---

## The One Trusted Entity

In the Arcane model, exactly ONE entity has production secrets:

-   **Solo dev**: You (your laptop)
-   **Small team**: You (controlled deploys)
-   **Large team**: Build server only
-   **Enterprise**: Dedicated deploy infrastructure

Everyone else just pushes code. They:

-   Can't see secrets
-   Can't deploy to prod
-   Can't take secrets when they leave
-   Can't accidentally leak what they don't have

---

## Comparison

| Scenario          | Traditional                        | Arcane                           |
| ----------------- | ---------------------------------- | -------------------------------- |
| Developer leaves  | Rotate all secrets (they saw them) | Do nothing (they never had them) |
| Laptop stolen     | Secrets exposed                    | Nothing to expose                |
| Accidental commit | Secrets in git history             | Encrypted, useless without key   |
| Prod access audit | Check every developer              | Check one build server           |

---

## This Isn't Just Encryption

SOPS, git-crypt, and others encrypt secrets. But:

-   **SOPS**: If you're a recipient, you can decrypt. Developers are recipients.
-   **git-crypt**: Same - authorized users can decrypt.
-   **Arcane**: Separates "can work on code" from "can deploy to prod"

The innovation is **machine-specific deploy keys**. A build server can deploy without developers having access.

---

## Summary

> **Arcane is the only tool where developers can contribute code to a production app without ever having access to production secrets.**

No other tool does this. Not Vault. Not AWS. Not SOPS. Not any of them.

---

## The Holy Grail: Security + Speed

Most tools make you choose:

| Approach                  | Security                              | Speed                        | Revocation           |
| ------------------------- | ------------------------------------- | ---------------------------- | -------------------- |
| **Traditional**           | ❌ Devs have secrets                  | ✅ Fast (but insecure)       | ❌ Rotate everything |
| **Enterprise "Security"** | ⚠️ Complex policies, still accessible | ❌ Slow (approval workflows) | ⚠️ Audit + rotate    |
| **Arcane**                | ✅ Zero access                        | ✅ Push → Deploy             | ✅ Delete one file   |

**The trade others make:**

-   "Security" = slow approval workflows + devs still have access
-   "Speed" = everyone has keys + pray they don't leak

**Arcane's position:**

-   **Security** = mathematically impossible without the key
-   **Speed** = push and it deploys
-   **Revocation** = delete a `.age` file, instant lockout

No compromise. Both at once.

---

## Instant Revocation

When someone leaves or a key is compromised:

**Traditional:**

```
1. Identify all secrets they had access to
2. Generate new secrets for each service
3. Update all deployments
4. Coordinate downtime
5. Hope you didn't miss any
```

**Arcane:**

```bash
rm .git/arcane/keys/machine:compromised.age
git commit -m "Revoke access"
git push
# Done. They can't decrypt anything anymore.
```

No secret rotation. No service restarts. No downtime. They never had the actual secrets - just a key to unlock them. Remove the key, game over.

---

## Arcane vs The Alternatives

### Coolify / CapRover / Dokku

These tools install a **massive control plane on your server**:

| Issue                      | What Happens                                           |
| -------------------------- | ------------------------------------------------------ |
| **Building ON the server** | Your $5 VPS trying to compile Rust/Go. Good luck.      |
| **Resource overhead**      | The control plane eats RAM your app needs              |
| **Port conflicts**         | "Wait, is Traefik on 80 or is Nginx?"                  |
| **Secret management**      | Web UI where you type secrets. Hope no one's watching. |
| **Updates**                | "Coolify update broke my deploys" - every month        |
| **Crashes**                | Control plane goes down = can't deploy anything        |

**Arcane:**

-   Builds **locally** (your beefy dev machine)
-   Pushes **pre-built images** (server just runs them)
-   **Zero overhead** (no control plane, just Docker)
-   **No port conflicts** (Caddy handles routing cleanly)
-   **Secrets encrypted in git** (not typed into a web UI)
-   **Nothing to update on server** (it's just Docker)

### Vercel / Railway / Render

Cloud platforms that do everything for you:

| Issue                | What Happens                                   |
| -------------------- | ---------------------------------------------- |
| **Vendor lock-in**   | Your app only works on their platform          |
| **Pricing**          | Free tier lures you, then $20/mo per service   |
| **Cold starts**      | Serverless = slow first request                |
| **Limited control**  | Can't SSH in, can't see what's happening       |
| **Data sovereignty** | Your data on their servers, their jurisdiction |

**Arcane:**

-   **Your servers** (any VPS, any provider, any country)
-   **Fixed cost** ($5/mo VPS runs multiple apps)
-   **Always warm** (containers run 24/7)
-   **Full SSH access** (debug anything)
-   **Your data stays yours**

### Kubernetes

The enterprise answer to everything:

| Issue                     | What Happens                                 |
| ------------------------- | -------------------------------------------- |
| **Complexity**            | YAML files longer than your app code         |
| **Learning curve**        | Months to understand pods, services, ingress |
| **Resource requirements** | 3-node cluster minimum for HA                |
| **Overkill**              | You have 2 apps. K8s has 47 concepts.        |

**Arcane:**

-   **One command**: `arcane deploy myapp`
-   **Learning curve**: 5 minutes
-   **Resources**: One $5 VPS is enough
-   **Concepts**: Build. Push. Run.

### GitHub Actions / GitLab CI

Pipeline-based CI/CD:

| Issue               | What Happens                                           |
| ------------------- | ------------------------------------------------------ |
| **YAML debugging**  | "Why did the pipeline fail?" (1 hour later)            |
| **Secret sprawl**   | Secrets in GitHub, secrets in AWS, secrets in Vault... |
| **Slow feedback**   | Push, wait 5 min, see it failed, push again            |
| **Minutes billing** | Free tier runs out, now you're paying                  |

**Arcane:**

-   **No YAML** (it's a CLI tool)
-   **Secrets in one place** (encrypted in repo)
-   **Instant feedback** (build locally, see errors immediately)
-   **Free forever** (runs on your machine)

---

## Solo Workflow: Embarrassingly Simple

For solo developers, Arcane is so simple it feels like cheating:

```bash
# Setup (once, on your laptop)
arcane identity new
arcane init

# Daily work - literally just code
# ... edit files ...
# Daemon auto-commits with AI messages
# Auto-push sends to git

# Deploy - one command
arcane deploy myapp
# Done. Live in seconds.
```

### What You DON'T Do

**No server setup:**

-   No SSH in to configure environment variables
-   No `.env` files to manually create
-   No "did I set the right value?" anxiety
-   Secrets are baked into the deploy automatically

**No configuration hell:**

-   No YAML files
-   No Docker Compose to maintain separately
-   No CI/CD pipeline to debug
-   No "works on my machine" issues

**No waiting:**

-   Build locally (fast)
-   Compress and push (seconds)
-   Container starts (instant)
-   Zero-downtime swap (seamless)

### The Timeline

| What                 | Traditional                 | Arcane            |
| -------------------- | --------------------------- | ----------------- |
| First deploy setup   | Hours (CI/CD, secrets, env) | 5 minutes         |
| Each deploy          | Minutes (pipeline runs)     | ~10 seconds       |
| Add a secret         | Update CI, redeploy         | Edit file, deploy |
| Debug deploy failure | Check 5 different systems   | Check one log     |

### The Feeling

You're writing code in 2024 with the simplicity of 2005 `scp` deployments, but with:

-   Enterprise-grade encryption
-   Zero-downtime deploys
-   AI-generated commit messages
-   Automatic secret injection

It feels like you're getting away with something.

---

## Scale As You Grow

The same tool scales without changing your workflow:

| Stage           | Who Deploys            | Setup                      |
| --------------- | ---------------------- | -------------------------- |
| **Solo**        | You from your laptop   | Just use it                |
| **Small team**  | Any dev with a key     | `arcane team invite`       |
| **Medium team** | Only leads deploy      | Give keys to leads only    |
| **Large team**  | Build server only      | Set up Arcane Spark        |
| **Enterprise**  | Multiple build servers | Stage/Prod Spark instances |

You don't migrate to a new tool. You just add machine keys.
