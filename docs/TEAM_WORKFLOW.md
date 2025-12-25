# Team Workflow Guide

How to share encrypted secrets with teammates using Arcane.

---

## Overview

Arcane uses **public key cryptography** for team sharing:

-   Each teammate has their own **private key** (secret)
-   They share their **public key** (safe to share)
-   You encrypt the repo key for their public key
-   They use their private key to decrypt

**No passwords are exchanged. No cloud accounts needed.**

---

## Adding a Teammate

### Prerequisites

Both you and your teammate need:

1. Arcane installed (`cargo install --path .`)
2. A master identity (`~/.arcane/identity.age`)

### Step 1: Teammate Gets Their Public Key

**Teammate runs:**

```bash
arcane identity show
```

**Output:**

```
🔑 Your Arcane Identity

Public Key (share this with teammates):
age1alicepublickey1234567890abcdef...

Identity File: /home/alice/.arcane/identity.age
```

They send this public key to you (safe to share via Slack, email, etc.).

### Step 2: You Invite Them

**You run:**

```bash
# Create a team and invite them
arcane team invite devs age1alicepublickey...

# Commit and push the authorization
git add .git/arcane
git commit -m "Add Alice to team"
git push
```

### Step 3: Teammate Pulls and Decrypts

**Teammate runs:**

```bash
git pull
cat .env  # Works! Decrypted automatically
```

---

## Removing a Teammate

```bash
# Find their key file
ls .git/arcane/keys/
# → user:alice.age

# Delete it
rm .git/arcane/keys/user:alice.age
git add -u .git/arcane
git commit -m "Revoke Alice's access"
git push
```

Alice immediately loses access. No re-encryption needed.

---

## Testing the Flow Locally

You can simulate a teammate without another person:

### Step 1: Generate a Fake Teammate Identity

```bash
# Create Alice's identity
age-keygen -o /tmp/alice.age
# Note the public key from output: age1alice...
```

### Step 2: Invite Fake Teammate

```bash
arcane team invite testers age1alice...
git add .git/arcane
git commit -m "Add test teammate"
```

### Step 3: Test Decryption as That Teammate

```bash
# Extract Alice's private key
ALICE_KEY=$(grep AGE-SECRET-KEY /tmp/alice.age)

# Test if Alice can decrypt
ARCANE_MACHINE_KEY=$ALICE_KEY arcane run -- echo "Alice can decrypt!"
```

### Step 4: Cleanup

```bash
rm .git/arcane/keys/user:*.age  # Remove test key
rm /tmp/alice.age               # Remove fake identity
git add -u && git commit -m "Remove test teammate"
```

---

## How It Works

```
You (Owner)                          Alice (Teammate)
─────────────                        ────────────────
Has: owner.age                       Generates: alice_identity.age
     (repo.key encrypted for you)    Shares: age1alice... (public)

You create: user:alice.age
     (repo.key encrypted for Alice)

Alice pulls → decrypts user:alice.age → gets repo.key → decrypts .env
```

Both have independent ways to unlock the **same repo.key**. Neither sees the other's private key.

---

## Security Notes

| What                   | Where                    | Safe to Share?          |
| ---------------------- | ------------------------ | ----------------------- |
| Your private key       | `~/.arcane/identity.age` | ❌ NEVER                |
| Your public key        | Derived from above       | ✅ Yes                  |
| Teammate's `.age` file | `.git/arcane/keys/`      | ✅ Yes (it's encrypted) |
| Repo key               | Inside `.age` files      | Protected by encryption |

---

## FAQ

**Q: Can a teammate see other teammates' private keys?**
No. Each `.age` file contains the repo key encrypted for ONE specific person. Alice can't read Bob's `.age` file.

**Q: What if a teammate leaves?**
Delete their `.age` file and push. Instant revocation.

**Q: Do I need to re-encrypt secrets when adding/removing people?**
No. The secrets themselves don't change. You just add/remove the "envelopes" (`.age` files) that protect the repo key.
