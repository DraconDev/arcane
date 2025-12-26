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
