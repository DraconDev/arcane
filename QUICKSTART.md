# Arcane Quickstart

Get your secrets encrypted in 2 minutes.

---

## 🚀 Solo Setup (One Developer)

### Step 1: Install

```bash
cargo install --path .
```

### Step 2: Create Identity (Once)

```bash
arcane identity new
```

### Step 3: Global Config (Once)

```bash
# Make ALL your repos auto-encrypt .env files
git config --global filter.git-arcane.clean "arcane clean %f"
git config --global filter.git-arcane.smudge "arcane smudge"
git config --global core.attributesfile ~/.gitattributes
echo "*.env filter=git-arcane diff=git-arcane" >> ~/.gitattributes
```

### Step 4: Use It

```bash
# In any repo, just work normally
echo "API_KEY=secret123" > .env
git add .env
git commit -m "Add secrets"
```

That's it. Your `.env` is encrypted in Git, plaintext locally.

---

## 👥 Team Setup (Multiple Developers)

### Adding a Teammate

**Alice (new teammate) does:**

```bash
# Show her public key
arcane identity show
# Outputs: age1alicepublickey...
# She sends this to you
```

**You (owner) do:**

```bash
arcane team add alice age1alicepublickey...
git add .git/arcane
git commit -m "Add Alice to team"
git push
```

**Alice can now:**

```bash
git pull
cat .env  # Works! Decrypts automatically
```

---

## 🚀 Server Setup (Deployment)

Two actors: **You** (local) and **The Server** (remote, e.g., Coolify).

### Step 1: Generate Server Identity (On Your Machine)

```bash
arcane deploy gen-key
```

**Output:**

```
🤖 Generated Machine Identity

Public Key (Authorize this):
age1abc123...

Private Key (Set as ARCANE_MACHINE_KEY):
AGE-SECRET-KEY-1xyz789...
```

### Step 2: Authorize the Server (On Your Machine)

```bash
# Tell YOUR repo that this server is allowed
arcane deploy allow age1abc123...

# Commit the authorization
git add .git/arcane
git commit -m "Authorize production server"
git push
```

### Step 3: Configure the Server

#### Option A: Your Own Project (Arcane Pre-Installed)

If Arcane is installed as a binary on your server:

1. **Add Environment Variable:**

    - `ARCANE_MACHINE_KEY=AGE-SECRET-KEY-1xyz789...`

2. **Update Start Command:**
    - Wrap your command: `arcane run -- npm start`

#### Option B: The `secrets-demo` Example (Arcane Built from Source)

The `examples/secrets-demo` Dockerfile builds Arcane from source. For Coolify:

| Setting                  | Value                                          |
| ------------------------ | ---------------------------------------------- |
| **Base Directory**       | `/` (repo root)                                |
| **Dockerfile Location**  | `examples/secrets-demo/Dockerfile`             |
| **Environment Variable** | `ARCANE_MACHINE_KEY=AGE-SECRET-KEY-1xyz789...` |

The Dockerfile already has `CMD ["arcane", "run", "--", "./secrets-demo"]`, so no start command change needed.

### Step 4: What Happens When Server Starts

```
Server boots
    ↓
Runs: arcane run -- <your app>
    ↓
Arcane sees ARCANE_MACHINE_KEY env var
    ↓
Finds .git/arcane/keys/machine:abc123.age
    ↓
Decrypts it → Gets repo.key
    ↓
Decrypts .env → Secrets in MEMORY (never on disk)
    ↓
Launches your app (with secrets as env vars)
```

**Result:** Your app runs with secrets, but the disk only has ciphertext.

---

## 📋 Notable Commands

| Command                         | What It Does                       |
| ------------------------------- | ---------------------------------- |
| `arcane identity show`          | Show your public key               |
| `arcane identity new`           | Generate master identity           |
| `arcane deploy gen-key`         | Create identity for a server       |
| `arcane deploy allow <key>`     | Grant server access to this repo   |
| `arcane team add <alias> <key>` | Add a teammate                     |
| `arcane run -- <cmd>`           | Run command with secrets decrypted |
| `arcane scan <file>`            | Check file for leaked secrets      |
| `arcane shadow list`            | View automatic backups             |
| `arcane dashboard`              | Open visual dashboard              |

---

## 🔑 Key Locations

| File                             | Purpose                         |
| -------------------------------- | ------------------------------- |
| `~/.arcane/identity.age`         | Your master key (BACK THIS UP!) |
| `.git/arcane/keys/owner.age`     | Repo key wrapped for you        |
| `.git/arcane/keys/machine:*.age` | Repo key wrapped for servers    |
| `.git/arcane/keys/user:*.age`    | Repo key wrapped for teammates  |

---

## ❓ FAQ

**Q: Do I need to run `arcane init`?**
No. It runs automatically on first encrypt.

**Q: What if I lose my master key?**
If a teammate or server still has access, they can re-invite you. Otherwise, you still have plaintext locally—just re-encrypt.

**Q: Can teammates see my private key?**
No. You exchange public keys only. Private keys never leave their owner's machine.

---

## 📚 More Docs WIP

-   [KEY_ARCHITECTURE.md](docs/KEY_ARCHITECTURE.md) — How the encryption works
-   [CLI.md](docs/CLI.md) — Full command reference
-   [DEPLOYMENT.md](DEPLOYMENT.md) — Server deployment guide
