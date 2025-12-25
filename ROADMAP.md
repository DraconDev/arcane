# Arcane Roadmap

> **Philosophy**: Build an amazing tool for myself. Licensing means companies pay.

---

## Current State (v0.1.x)

### ✅ Stable

-   **Security Layer**: Envelope encryption, team keys, machine keys, secrets scanning
-   **Git Integration**: Transparent encrypt/decrypt, git filters, shadow branches
-   **TUI Dashboard**: Tabs, graph, settings, identity vault

### ✅ Beta

-   **AI Auto-Commit**: Daemon watches files, generates commit messages
-   **Smart Squash**: AI groups commits into Minors + Patches
-   **Bulk Squash**: All commits → 1 Major/Minor bump (configurable)
-   **Version Bumping**: Auto-detect Cargo.toml/package.json, AI-driven semver

### 🚧 Scaffolded (Not Complete)

-   **Ops Module**: Server config, SSH, deploy functions exist but incomplete

---

## Near Term (v0.2.x)

| Priority | Feature                    | Description                                           |
| -------- | -------------------------- | ----------------------------------------------------- |
| 1        | **Push-to-Deploy**         | `arcane deploy` pushes code + baked secrets to server |
| 2        | **Server Groups**          | Define groups (prod, stage) → deploy to many at once  |
| 3        | **Health Checks**          | Tiny status endpoint for deployed containers          |
| 4        | **Graph Branch Switching** | Toggle between main/current/custom branches           |

---

## Medium Term (v0.3.x)

| Feature                | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| **Build Server Mode**  | `arcane spark` listens for commits, auto-builds/deploys |
| **Rollback Timeline**  | Visual history with one-click revert                    |
| **Multi-Repo Support** | Orchestrate linked projects                             |

---

## Long Term Vision

**Arcane = The Sovereign Developer Platform**

```
Local Dev → Auto-Commit → Smart Squash → Version → Deploy → Monitor
     └── All local. All yours. Zero cloud required.
```

---

## Licensing Strategy

| User                    | License                     |
| ----------------------- | --------------------------- |
| Solo devs               | Free                        |
| Open source             | Free                        |
| Companies < 5 employees | Free                        |
| Companies 5+ employees  | Commercial license required |

---

## Next Actions

-   [ ] Stabilize squash features (test with real repos)
-   [ ] Complete `arcane deploy` command
-   [ ] Define server group config format
-   [ ] Integrate deploy into TUI Ops tab
