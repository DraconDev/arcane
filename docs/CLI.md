# Arcane CLI Reference

This guide details every command available in the `arcane` CLI.

## 🔐 Setup & Security

### `arcane setup`

**Usage**: `arcane setup`
**Purpose**: Configure global git filters (run once after install).
**Details**:

-   Registers `git-arcane` filter in global gitconfig.
-   Also registers `git-seal` filter for backward compatibility with legacy repos.
-   **Run this once** after installing or updating arcane.

### `arcane init`

**Usage**: `arcane init`
**Purpose**: Creates a unique encryption key for the current repository.
**Details**:

-   Generates a random `repo.key`.
-   Encrypts it with your personal Master Identity (`~/.arcane/identity.age`).
-   Saves it to `.git/arcane/keys/`.
-   **Why?**: Required so that secrets in this repo are encrypted with a key unique to _this_ project, not your global master key.

### `arcane scan <path>`

**Usage**: `arcane scan src/`
**Purpose**: Scans files for accidental secret leaks.
**Details**:

-   Uses regex patterns to find AWS keys, Stripe keys, Private Keys, etc.
-   Returns a list of potential violations.
-   **Why?**: Catch leaks _before_ you commit.

### `arcane clean` / `arcane smudge`

**Usage**: (Automatic) Called by Git.
**Purpose**: The "Git Filter" plumbing.

-   `clean`: Encrypts file content (on `git add`).
-   `smudge`: Decrypts file content (on `git checkout`).
    **Why?**: Enables "Transparent Encryption". You see plaintext, Git stores ciphertext.

---

## 🚀 Deployment (Zero-Trust)

### `arcane deploy gen-key`

**Usage**: `arcane deploy gen-key`
**Purpose**: Generate a new identity for a server (e.g., Coolify, VPS).
**Details**:

-   Outputs a Public Key (`age1...`) and a Private Key (`AGE-SECRET-KEY...`).
-   **Does NOT** save anything to disk. You must copy-paste the keys.
-   **Why?**: Servers need their own identity so you can revoke them later without rotating everyone else's keys.

### `arcane deploy allow <public_key>`

**Usage**: `arcane deploy allow age1...`
**Purpose**: Authorize a specific server to access this repo's secrets.
**Details**:

-   Takes the Repo Key (decrypted with your identity).
-   Re-encrypts it for the Server's Public Key.
-   Saves it to `.git/arcane/keys/machine:<hash>.age`.
-   **Why?**: Grant access to one specific server.

### `arcane run -- <command>`

**Usage**: `arcane run -- npm start`
**Purpose**: Run a command with decrypted secrets injected into the environment.
**Details**:

-   Detects the `ARCANE_MACHINE_KEY` environment variable.
-   Decrypts the `.env` file **in memory**.
-   Spawns `<command>` with the secrets as env vars.
-   **Why?**: "Zero-Trust Runtime". Secrets are never written to disk on the server.

---

## 🆔 Identity Management

### `arcane identity new`

**Usage**: `arcane identity new`
**Purpose**: Generate your personal Master Identity.
**Details**:

-   Creates `~/.arcane/identity.age` (your private key).
-   Run this once per machine.
-   **Security**: Never share this file. Back it up safely.

### `arcane identity show`

**Usage**: `arcane identity show`
**Purpose**: Display your Public Key.
**Details**:

-   Outputs the `age1...` public key derived from your identity.
-   **Share this**: Send this key to teammates so they can add you to repos.

---

## 👥 Teams & Collaboration

### `arcane team add <alias> <public_key>`

**Usage**: `arcane team add alice age1...`
**Purpose**: Grant a teammate access to the current repository.
**Details**:

-   Takes the Repo Key (decrypted with your identity).
-   Re-encrypts it for the Teammate's Public Key.
-   Saves it to `.git/arcane/keys/user:<alias>.age`.
-   **Why?**: Zero-trust sharing. You never share your private key or the repo key directly.

### `arcane team list`

**Usage**: `arcane team list`
**Purpose**: See who has access to this repository.

### `arcane team invite` / `create` (Legacy)

Higher-level team abstractions are available but `team add` is the preferred direct method for most users.

---

## 🧠 Sovereign Intelligence (Development)

### `arcane start <path>`

**Purpose**: Start the background daemon.
**Details**:

-   Watches files for changes.
-   Automatically creates "Shadow Commits" (Backups).
-   (WIP) Generates AI commit messages.

### `arcane timeline`

**Purpose**: See a linear history of your work (Shadow Branches).

### `arcane dashboard`

**Purpose**: Launch the Sovereign Terminal (TUI) for managing repos, settings, and identity.
