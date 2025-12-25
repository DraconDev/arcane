# Arcane Key Architecture

This document explains the cryptographic design of Arcane in detail.

---

## The Key Hierarchy

Arcane uses **three layers** of keys:

```
Layer 1: Master Identity  (Your personal private key)
         ↓ protects
Layer 2: Repo Key         (Random key, unique per project)
         ↓ protects
Layer 3: Secrets          (Your .env files)
```

---

## Layer 1: Master Identity

**Location**: `~/.arcane/identity.age`

This is YOUR personal private key. It:

-   Lives only on YOUR machine
-   Is NEVER uploaded to any repo
-   Is generated once and reused for all your projects

**Example content**:

```
AGE-SECRET-KEY-1CRF9VVC7A7E5Z2K6XWEKU6DKCRJH8VVJ59Y74H7H6SUTHSZY50CSWT695K
```

**Backup**: Store this in your password manager (Bitwarden, 1Password). If you lose it and have no teammates/servers with access, you lose your secrets forever.

---

## Layer 2: Repo Key

**Location**: Exists only inside encrypted files in `.git/arcane/keys/`

This is a **random symmetric key** generated uniquely for each repository. It:

-   Is generated automatically when you first encrypt a file
-   Actually encrypts/decrypts your `.env` files
-   Is NEVER stored in plaintext anywhere

**How it's protected**:
The repo key is wrapped (encrypted) separately for each person/server who needs access:

| File                      | Contains                                       |
| ------------------------- | ---------------------------------------------- |
| `keys/owner.age`          | Repo Key encrypted for YOUR public key         |
| `keys/machine:abc123.age` | Repo Key encrypted for a SERVER's public key   |
| `keys/user:alice.age`     | Repo Key encrypted for a TEAMMATE's public key |

Each `.age` file contains the **same repo key**, just encrypted for a different recipient.

---

## Layer 3: Secrets

**Location**: Your `.env` files (stored as ciphertext in Git)

When you run `git add .env`:

1. Git calls `arcane clean`
2. Arcane loads `owner.age` and decrypts it using your Master Identity → gets `repo.key`
3. Arcane encrypts `.env` content using `repo.key`
4. Ciphertext is saved to Git

When you run `git checkout`:

1. Git calls `arcane smudge`
2. Arcane loads `owner.age` and decrypts it → gets `repo.key`
3. Arcane decrypts the ciphertext → plaintext `.env`
4. You see your secrets

---

## Visual Flow: Encryption

```
┌─────────────────────────────────────────────────────────────────┐
│ YOUR MACHINE                                                    │
│                                                                 │
│  ~/.arcane/identity.age                                         │
│  (Your Private Key)                                             │
│         │                                                       │
│         ▼                                                       │
│  Decrypt owner.age ──► Get repo.key                             │
│                              │                                  │
│                              ▼                                  │
│                       Encrypt .env ──► Ciphertext               │
│                                              │                  │
└──────────────────────────────────────────────┼──────────────────┘
                                               │
                                               ▼
                                         Git Repository
                                     (Only ciphertext stored)
```

---

## Visual Flow: Adding a Server

When you run `arcane deploy allow age1...`:

```
┌─────────────────────────────────────────────────────────────────┐
│ YOUR MACHINE                                                    │
│                                                                 │
│  1. Load owner.age                                              │
│  2. Decrypt with your identity → Get repo.key                   │
│  3. Encrypt repo.key for SERVER's public key                    │
│  4. Save as machine:xyz.age                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Result: Server can now decrypt .env using THEIR private key
        (You never gave them YOUR private key)
```

---

## Visual Flow: Server Decryption

When the server runs `arcane run -- npm start`:

```
┌─────────────────────────────────────────────────────────────────┐
│ SERVER                                                          │
│                                                                 │
│  ARCANE_MACHINE_KEY env var                                     │
│  (Server's Private Key)                                         │
│         │                                                       │
│         ▼                                                       │
│  Find machine:xyz.age (matches this key)                        │
│         │                                                       │
│         ▼                                                       │
│  Decrypt machine:xyz.age ──► Get repo.key                       │
│                                    │                            │
│                                    ▼                            │
│                             Decrypt .env ──► Secrets in memory  │
│                                                    │            │
│                                                    ▼            │
│                                              Launch npm start   │
│                                              (with env vars)    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Not Just Use One Key? (Git Seal Comparison)

### Git Seal Model

```
Master Key ──► Encrypts ──► .env
```

**Problems**:

1. To add a teammate, you must share YOUR master key
2. To revoke access, you must change YOUR key and re-encrypt everything
3. One key for all repos = compromise one, compromise all

### Arcane Model

```
repo.key (random) ──► Encrypts ──► .env
├─ Wrapped for Owner
├─ Wrapped for Server A
├─ Wrapped for Server B
└─ Wrapped for Teammate Alice
```

**Advantages**:

1. Add teammate: Just create a new `.age` file for them
2. Revoke access: Delete their `.age` file (no re-encryption needed)
3. Each repo has unique key = isolated blast radius
4. Your master key NEVER leaves your machine

---

## File Summary

| File            | Location            | Contains                          | Who Has It       |
| --------------- | ------------------- | --------------------------------- | ---------------- |
| `identity.age`  | `~/.arcane/`        | Your private key                  | Only you         |
| `owner.age`     | `.git/arcane/keys/` | Repo key (encrypted for you)      | In repo (safe)   |
| `owner.pub`     | `.git/arcane/keys/` | Your public key                   | In repo (public) |
| `machine:*.age` | `.git/arcane/keys/` | Repo key (encrypted for server)   | In repo (safe)   |
| `user:*.age`    | `.git/arcane/keys/` | Repo key (encrypted for teammate) | In repo (safe)   |

---

## Security Properties

1. **Zero Knowledge**: Git/GitHub never sees plaintext secrets OR your private key
2. **Forward Secrecy**: Revoking access is instant (delete file) and doesn't require re-encryption
3. **Isolation**: Each repo has a unique key; compromise is contained
4. **Recovery**: As long as ONE authorized party exists, secrets can be recovered
