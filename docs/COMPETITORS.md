# Arcane vs Competitors

A comprehensive comparison of secrets management solutions.

---

## Quick Comparison

| Feature              | Arcane                 | Infisical     | Doppler   | SOPS           | Git-Crypt   |
| -------------------- | ---------------------- | ------------- | --------- | -------------- | ----------- |
| **Architecture**     | Git-native (no server) | Server/SaaS   | SaaS only | File-based     | File-based  |
| **Self-hosted**      | Yes (default)          | Yes (complex) | No        | Yes            | Yes         |
| **Cloud required**   | No                     | Optional      | Yes       | Optional (KMS) | No          |
| **Team sharing**     | Native                 | Native        | Native    | Manual         | Manual      |
| **Revocation**       | Delete file            | Dashboard     | Dashboard | Re-encrypt     | Re-encrypt  |
| **Per-project keys** | Yes                    | Yes           | Yes       | Manual         | No          |
| **Audit logs**       | Git history            | Built-in      | Built-in  | Git history    | Git history |
| **Pricing**          | Free                   | Freemium      | Freemium  | Free           | Free        |
| **Setup time**       | 2 min                  | 10+ min       | 5 min     | 5 min          | 2 min       |

---

## Detailed Breakdown

### Infisical

**What it is**: Open-source secrets management platform with dashboard, CLI, and SDK.

**Architecture**: Self-hosted or cloud. Requires running a server (Docker/Kubernetes).

| Pros                                           | Cons                           |
| ---------------------------------------------- | ------------------------------ |
| Beautiful dashboard UI                         | Requires server infrastructure |
| Native integrations (Kubernetes, Vercel, etc.) | More complex self-hosting      |
| Versioning and audit logs                      | Overkill for solo devs         |
| Secrets rotation                               | Another service to maintain    |

**Best for**: Teams with DevOps capacity, enterprises needing compliance features.

**Arcane advantage**: No server needed. Secrets live in Git. Zero infrastructure.

---

### Doppler

**What it is**: SaaS-only secrets management with CLI sync.

**Architecture**: Cloud-only. Your secrets live on their servers.

| Pros                 | Cons                       |
| -------------------- | -------------------------- |
| Very polished UX     | No self-hosted option      |
| Easy team onboarding | Vendor lock-in             |
| Environment sync     | Secrets leave your control |
| Many integrations    | Paid for most features     |

**Best for**: Teams that trust SaaS, want zero setup.

**Arcane advantage**: Sovereign. Secrets never leave your infrastructure. No vendor lock-in.

---

### SOPS (Mozilla)

**What it is**: File-based encryption with multi-backend support (AWS KMS, GCP, PGP, age).

**Architecture**: CLI tool, no server. Encrypts files in-place.

| Pros                    | Cons                               |
| ----------------------- | ---------------------------------- |
| Battle-tested (Mozilla) | Manual key management              |
| Multi-cloud KMS support | No Git filter (visible ciphertext) |
| Flexible file formats   | Complex for team sharing           |
| age support             | Steeper learning curve             |

**Best for**: Teams already using AWS/GCP, need KMS integration.

**Arcane advantage**: Automatic Git filter (transparent encryption). Simpler team sharing.

---

### Git-Crypt

**What it is**: GPG-based transparent encryption for Git.

**Architecture**: Git filter, uses GPG keys.

| Pros               | Cons                          |
| ------------------ | ----------------------------- |
| Simple and proven  | GPG key management is painful |
| Transparent to Git | All repos use same key        |
| No server needed   | No per-repo key isolation     |
| Open source        | Revocation requires re-keying |

**Best for**: Solo devs already comfortable with GPG.

**Arcane advantage**: Per-repo keys (better isolation), age instead of GPG (simpler), easier team/server authorization.

---

### HashiCorp Vault

**What it is**: Enterprise secrets management platform.

**Architecture**: Self-hosted server or HCP (cloud).

| Pros                  | Cons                       |
| --------------------- | -------------------------- |
| Industry standard     | Very complex to operate    |
| Dynamic secrets       | Overkill for most projects |
| Full audit/compliance | Requires dedicated team    |
| Secret rotation       | High operational overhead  |

**Best for**: Large enterprises with dedicated security teams.

**Arcane advantage**: No infrastructure. Solo devs can use it in 2 minutes.

---

### dotenv-vault

**What it is**: Encrypted .env management with cloud sync.

**Architecture**: SaaS with CLI sync.

| Pros                   | Cons                            |
| ---------------------- | ------------------------------- |
| Familiar .env workflow | Cloud-based (secrets leave you) |
| Easy setup             | Limited team features           |
| Environment sync       | Relatively new                  |

**Best for**: Node.js teams wanting quick .env encryption.

**Arcane advantage**: Fully local (no cloud). Works with any stack, not just Node.

---

## Positioning Matrix

```
                    Simple ────────────────── Complex
                        │                        │
    Self-Hosted    Arcane │ Git-Crypt    SOPS    │ Vault
                        │                        │
                   ─────┼────────────────────────┼─────
                        │                        │
    Cloud/SaaS          │ dotenv-vault  Doppler  │ Infisical
                        │                        │
                    Simple ────────────────── Complex
```

## 🤖 Developer Experience & Tooling

### Source of Truth: Where Do Your Secrets Live?

| Solution      | Source of Truth       | Version Control? | Offline Access? |
| ------------- | --------------------- | ---------------- | --------------- |
| **Arcane**    | Git (`.env` file)     | ✅ Yes           | ✅ Yes          |
| **Infisical** | Dashboard/Server      | Via Infisical UI | ⚠️ Need server  |
| **Doppler**   | Cloud dashboard       | Via Doppler UI   | ❌ No           |
| **SOPS**      | Git (encrypted files) | ✅ Yes           | ✅ Yes          |
| **Git-Crypt** | Git (encrypted files) | ✅ Yes           | ✅ Yes          |
| **Vault**     | Vault server          | Via Vault audit  | ⚠️ Need server  |

**Infisical and Doppler CAN be single sources of truth** - they just live outside your codebase. This is a tradeoff:

-   **Dashboard-based**: Easier for non-developers, nice UI, centralized management
-   **Git-based (Arcane)**: Config-as-code, portable, works offline, version controlled with your code

### AI Assistant Compatibility

For developers using AI coding assistants (Cursor, Copilot, Claude):

| Solution            | AI Sees Config? | Notes                              |
| ------------------- | --------------- | ---------------------------------- |
| **Arcane**          | ✅ Yes          | `.env` exists locally as plaintext |
| **Git-Crypt**       | ✅ Yes          | Same reason                        |
| **SOPS**            | ⚠️ Partial      | Sees encrypted blobs               |
| **Dashboard-based** | ❌ No           | Secrets not in codebase            |

If you work heavily with AI assistants, Git-native solutions (Arcane, Git-Crypt, SOPS) have an advantage because the config files exist in your repo.

### The Dashboard Tax (DX Failure Modes)

Dashboard-based tools introduce friction that leads to shortcuts:

**Workflow Comparison:**

| Action             | Dashboard-Based (Infisical/Doppler)                                          | Git-Based (Arcane)     |
| ------------------ | ---------------------------------------------------------------------------- | ---------------------- |
| Add secret         | 1. Open dashboard 2. Navigate to project 3. Add key 4. Wait for sync 5. Test | 1. Edit `.env` 2. Test |
| Test locally       | Same as above, or hardcode and risk committing                               | Just works             |
| Clean up test keys | Remember to go back and delete                                               | Delete line, commit    |

**Common Failure Modes:**

1. **Hardcoding to avoid friction**

    ```javascript
    // TODO: Move to Infisical later
    const API_KEY = "sk_test_hardcoded...";
    // Spoiler: "later" never comes
    ```

2. **Dashboard sprawl**

    ```
    OLD_API_KEY_2021
    TEST_STRIPE_KEY_JOHNS_LAPTOP
    TEMP_DELETE_ME_LATER
    PROD_KEY_BACKUP_MAYBE
    ```

    Nobody cleans this up. Ever.

3. **Typos and formatting errors**

    - Accidental space: `"supersecret "` vs `"supersecret"`
    - Equals in value: `password=abc=123` parsed incorrectly
    - No tooling to catch these in a web form

4. **No AI/tooling visibility**
    - AI can't catch `STRIPE_KEY` vs `STRIPE_SECRET_KEY` mismatch
    - Linters can't validate against dashboard values
    - No autocomplete for secret names

**Arcane's Design Principle**: The lazy path IS the secure path. Editing `.env` locally is faster than any dashboard, so developers naturally do the right thing.

---

## Arcane's Unique Position

**"Git-native secrets for sovereign developers"**

1. **No server**: Unlike Infisical/Vault, nothing to host
2. **No cloud**: Unlike Doppler/dotenv-vault, secrets stay local
3. **Per-repo keys**: Unlike Git-Crypt, isolated blast radius
4. **Modern crypto**: age instead of GPG
5. **Transparent**: Git filter = you never see ciphertext locally
6. **Config-as-code**: Secrets versioned alongside your code

---

## When to Use What

| Scenario                     | Recommendation     |
| ---------------------------- | ------------------ |
| Solo dev, multiple projects  | **Arcane**         |
| Small team, no DevOps        | **Arcane**         |
| Team with Kubernetes         | Infisical          |
| Enterprise compliance        | Vault or Infisical |
| "Just want it to work" cloud | Doppler            |
| Already using AWS KMS        | SOPS               |
| Legacy GPG workflow          | Git-Crypt          |
