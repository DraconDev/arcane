# Arcane vs The Competition

A brutally honest comparison of deployment tools.

---

## The Contenders

| Tool               | Category         | Model                             |
| ------------------ | ---------------- | --------------------------------- |
| **Arcane**         | CLI + TUI        | Build local → SSH push            |
| **Kamal**          | CLI              | Build local → Registry → SSH pull |
| **Coolify**        | Self-hosted PaaS | Git pull → Build on server        |
| **Dokku**          | Self-hosted PaaS | Git push → Buildpacks             |
| **Vercel/Railway** | Cloud PaaS       | Git push → Magic                  |
| **GitHub Actions** | CI/CD            | Workflow YAML → Runners           |

---

## Head-to-Head Comparison

### 1. Secrets Management

| Tool               | How Secrets Work                                | Security Level                         |
| ------------------ | ----------------------------------------------- | -------------------------------------- |
| **Arcane**         | Encrypted in Git, envelope crypto, machine keys | 🔒🔒🔒 (devs never see prod)           |
| **Kamal**          | 1Password/Doppler integration                   | 🔒🔒 (depends on external service)     |
| **Coolify**        | Web form in dashboard                           | 🔒 (attack surface)                    |
| **Dokku**          | `config:set` on server                          | 🔒 (manual, error-prone)               |
| **Vercel**         | Web dashboard                                   | 🔒🔒 (encrypted, but cloud)            |
| **GitHub Actions** | Repository secrets                              | 🔒🔒 (solid, but GitHub owns the keys) |

**Winner: Arcane** — Zero-trust, zero-cloud, devs never touch prod secrets.

---

### 2. Build Location

| Tool               | Where Build Happens         | Why It Matters                              |
| ------------------ | --------------------------- | ------------------------------------------- |
| **Arcane**         | Your laptop or Spark server | $5 VPS can't compile Rust. Your laptop can. |
| **Kamal**          | Your laptop                 | Same advantage                              |
| **Coolify**        | On the server               | ❌ Slow, eats server resources              |
| **Dokku**          | On the server               | ❌ Same problem                             |
| **Vercel**         | Cloud                       | ⚠️ Fast, but you don't control it           |
| **GitHub Actions** | GitHub runners              | ⚠️ 2-5 min cold start                       |

**Winner: Arcane/Kamal** — Build where you have power, deploy where you don't.

---

### 3. Deploy Speed (Cold Start to Running)

| Tool                | Time     | Notes                            |
| ------------------- | -------- | -------------------------------- |
| **Arcane (direct)** | ~30s     | SSH + docker run                 |
| **Arcane (Spark)**  | ~30s     | Webhook → SSH                    |
| **Kamal**           | ~1 min   | Registry push/pull adds overhead |
| **Coolify**         | ~3-5 min | Git clone + build on server      |
| **Dokku**           | ~2-5 min | Git push + buildpack             |
| **Vercel**          | ~1-2 min | Optimized, but variable          |
| **GitHub Actions**  | ~3-5 min | Cold start + checkout + install  |

**Winner: Arcane** — Direct SSH with pre-built artifacts is fastest.

---

### 4. Complexity

| Tool               | Config Files                      | Learning Curve    |
| ------------------ | --------------------------------- | ----------------- |
| **Arcane**         | `servers.toml` + Dockerfile       | Low               |
| **Kamal**          | `deploy.yml` (proprietary format) | Medium            |
| **Coolify**        | Web UI                            | Low (but fragile) |
| **Dokku**          | Procfile + buildpacks             | Medium            |
| **Vercel**         | Zero (magic)                      | Lowest            |
| **GitHub Actions** | Workflow YAML                     | High (YAML hell)  |

**Winner: Vercel** (for simplicity) / **Arcane** (for control)

---

### 5. Infrastructure Required

| Tool               | What You Need                                    |
| ------------------ | ------------------------------------------------ |
| **Arcane**         | Just your server (optional Spark for automation) |
| **Kamal**          | Your server + registry (Docker Hub/etc)          |
| **Coolify**        | Server with control plane installed              |
| **Dokku**          | Server with Dokku installed                      |
| **Vercel**         | Nothing (cloud)                                  |
| **GitHub Actions** | Nothing (uses GitHub runners)                    |

**Winner: Arcane** — Zero server-side agents required.

---

### 6. Cost at Scale (100 deploys/month)

| Tool               | Cost                       |
| ------------------ | -------------------------- |
| **Arcane**         | $0 (just your server cost) |
| **Kamal**          | $0-$10 (registry fees)     |
| **Coolify**        | $0 (self-hosted)           |
| **Dokku**          | $0 (self-hosted)           |
| **Vercel**         | $0-$20+ (free tier limits) |
| **GitHub Actions** | $0 (within 2000 min/mo)    |

**Winner: Tie** — Most are free at this scale.

---

### 7. Enterprise Features

| Feature              | Arcane   | Kamal | Coolify | Dokku | Vercel |
| -------------------- | -------- | ----- | ------- | ----- | ------ |
| Zero-trust secrets   | ✅       | ❌    | ❌      | ❌    | ❌     |
| Audit trail          | ✅ (Git) | ✅    | ⚠️      | ❌    | ✅     |
| Server groups        | ✅       | ✅    | ⚠️      | ❌    | N/A    |
| Blue/Green           | ✅       | ✅    | ✅      | ⚠️    | ✅     |
| Health checks        | ✅       | ✅    | ✅      | ✅    | ✅     |
| Private code support | ✅       | ✅    | ✅      | ✅    | ⚠️     |
| SOC2/HIPAA ready     | ✅       | ⚠️    | ❌      | ❌    | ✅     |

**Winner: Arcane** — Zero-trust + self-hosted = compliance-ready.

---

## The Verdict

### Use Arcane If:

-   You care about security (secrets never leave your infra)
-   You want speed (sub-minute deploys)
-   You hate web dashboards (TUI > browser)
-   You need compliance (SOC2/HIPAA)
-   You deploy to multiple servers

### Use Kamal If:

-   You're in the Rails ecosystem
-   You're okay with registry push/pull overhead
-   You like their config format

### Use Coolify If:

-   You want a web UI (and accept the security tradeoff)
-   You're okay with on-server builds
-   You don't need enterprise features

### Use Vercel If:

-   You're deploying frontend/static sites
-   You don't care about self-hosting
-   You want zero complexity

### Use GitHub Actions If:

-   You want free automation
-   Speed doesn't matter
-   Your code is public anyway

---

## What Arcane Does Differently

1. **Secrets are encrypted IN Git** — Not in a separate dashboard or vault
2. **No server-side control plane** — Nothing to crash, update, or secure
3. **Build where it's fast** — Your laptop or Spark, not your VPS
4. **TUI over Web UI** — Faster, more secure, keyboard-driven
5. **Machine keys** — Servers have their own identity; devs never touch prod secrets

---

## The Bottom Line

> **Arcane is for developers who want enterprise-grade security and speed without enterprise-grade complexity.**

If you're tired of web dashboards that crash, YAML pipelines that break, and secrets scattered across 5 different services — Arcane is your answer.
