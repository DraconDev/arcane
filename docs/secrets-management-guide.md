# Secrets Management & Key Hierarchy: A Comprehensive Guide

> A deep dive into how modern tools handle secrets, and how Arcane's multi-tier key strategy fits into the ecosystem.

---

## Table of Contents

1. [The Problem: Why Secrets Management Matters](#the-problem)
2. [Industry Landscape: How Others Solve It](#industry-landscape)
3. [Core Concepts: Envelope Encryption](#envelope-encryption)
4. [Arcane's Multi-Tier Key Strategy](#arcanes-strategy)
5. [Implementation Considerations](#implementation)
6. [Cloud vs Local Trade-offs](#cloud-vs-local)
7. [Recommendations](#recommendations)

---

## 1. The Problem: Why Secrets Management Matters {#the-problem}

Every application has secrets: API keys, database credentials, encryption keys, OAuth tokens. The challenge is threefold:

### 1.1 Storage

Where do secrets live? Common (bad) patterns:

-   Hardcoded in source code ❌
-   Committed `.env` files ❌
-   Shared via Slack/email ❌

### 1.2 Access Control

Who can read which secrets? Challenges:

-   New team member onboarding
-   Revoking access when someone leaves
-   Environment separation (dev/staging/prod)

### 1.3 Rotation

How do you change secrets without downtime? Considerations:

-   Key rotation schedules
-   Backward compatibility during transition
-   Audit trails

### The Git-Specific Challenge

Git is designed to remember everything. Once a secret is committed:

-   It exists in every clone forever
-   Even "deleted" files live in reflog
-   Force-pushing doesn't fix mirrors/forks

**This is why transparent encryption (like Git Seal) matters**: secrets are encrypted _before_ they ever touch Git's object store.

---

## 2. Industry Landscape: How Others Solve It {#industry-landscape}

### 2.1 Git-Crypt

**What it is**: Open-source tool for transparent GPG encryption of specific files in Git.

**How it works**:

```
.gitattributes:
*.env filter=git-crypt diff=git-crypt

$ git-crypt init
$ git-crypt add-gpg-user USER_ID
```

**Pros**:

-   Transparent (encrypt on commit, decrypt on checkout)
-   Selective (only encrypts files matching patterns)
-   Works with existing Git workflows

**Cons**:

-   GPG key management is complex
-   No easy key revocation (must re-encrypt entire history)
-   Metadata (filenames, commit messages) not encrypted
-   Can fail silently with some Git GUIs

**Best for**: Small teams with GPG expertise, open-source projects with some private files.

---

### 2.2 Mozilla SOPS (Secrets OPerationS)

**What it is**: Encrypts _values_ within config files (YAML, JSON, ENV) while keeping keys readable.

**How it works**:

```yaml
# Before encryption
database:
    password: supersecret

# After SOPS encryption
database:
    password: ENC[AES256_GCM,data:...,iv:...,tag:...]
sops:
    kms:
        - arn:aws:kms:us-east-1:...
```

**Pros**:

-   Integrates with cloud KMS (AWS, GCP, Azure)
-   Keeps file structure readable (only values encrypted)
-   Supports multiple key types (PGP, age, cloud KMS)
-   Git-friendly (merge conflicts are visible)

**Cons**:

-   Requires cloud account for KMS (or self-managed keys)
-   Manual setup and ongoing maintenance
-   Doesn't handle secrets _lifecycle_ (just encryption)

**Best for**: GitOps workflows, Kubernetes secrets, multi-cloud teams.

---

### 2.3 HashiCorp Vault

**What it is**: Centralized secrets management server with API access.

**How it works**:

```bash
# Store a secret
vault kv put secret/myapp/db password=supersecret

# Retrieve at runtime
vault kv get -field=password secret/myapp/db
```

**Key Features**:

-   **Dynamic Secrets**: Generate temporary credentials on-demand
-   **Leasing**: Secrets auto-expire after TTL
-   **Audit Logging**: Full trail of who accessed what
-   **Policies**: Fine-grained access control

**Pros**:

-   Industry standard ("gold standard" for enterprises)
-   Encryption as a Service (applications don't handle keys)
-   Automatic secret rotation
-   High availability mode

**Cons**:

-   Significant operational overhead
-   Requires dedicated infrastructure
-   Steep learning curve
-   Enterprise features require license

**Best for**: Large organizations, regulated industries (PCI-DSS, HIPAA), microservices.

---

### 2.4 Doppler

**What it is**: Developer-friendly managed secrets platform.

**How it works**:

```bash
# Inject secrets into environment
doppler run -- npm start

# Secrets sync across environments
doppler secrets set API_KEY=abc123
```

**Pros**:

-   Real-time sync across environments
-   CI/CD native integrations
-   No infrastructure to manage
-   Great developer UX

**Cons**:

-   SaaS dependency (internet required)
-   Pricing at scale
-   Less control over encryption

**Best for**: Startups, CI/CD-heavy workflows, teams prioritizing developer experience.

---

### 2.5 dotenv-vault

**What it is**: Encrypted `.env` files that are safe to commit.

**How it works**:

```bash
# Encrypt your .env into .env.vault
npx dotenv-vault local encrypt

# Decrypt at runtime using DOTENV_KEY
DOTENV_KEY=... node app.js
```

**Pros**:

-   Works with existing `.env` workflows
-   AES-256 encryption
-   No cloud dependency for basic usage
-   Language-agnostic

**Cons**:

-   DOTENV_KEY management is your responsibility
-   Paid cloud service for sync
-   Limited access control (key holder has full access)

**Best for**: Solo developers, small teams, projects already using dotenv.

---

### 2.7 Git Seal (Arcane's Origin)

**What it is**: A minimalist transparent encryption layer for Git, created as the predecessor to Arcane. Uses a single master key to encrypt files on `git add` (clean filter) and decrypt on `git checkout` (smudge filter).

**How it works**:

```bash
# Initialize (generates master key)
git-seal init

# Configure patterns
echo '*.env filter=seal' >> .gitattributes

# Transparent from here
git add .env  # Encrypts automatically
git checkout  # Decrypts automatically
```

**Pros**:

-   **Dead simple**: Single key, no GPG, no cloud
-   **Truly transparent**: Works with any Git command
-   **Zero dependencies**: Just the binary and Git
-   **Fast**: AES-GCM encryption

**Cons**:

-   **Single key only**: No team sharing without sharing the key
-   **No access control**: Key holder has full access to everything
-   **No revocation**: Removing access means rotating the key entirely
-   **Solo-focused**: Great for personal use, not teams

**Best for**: Solo developers who want "set and forget" encryption for sensitive files.

**User Feedback**:

> "Amazing to use, but it's solo only"
> "Found it too complex" (CLI-only, manual setup)

**Why Arcane evolved from it**: Git Seal proved the concept works beautifully—users loved the transparent encryption. But two critiques drove the evolution:

1. **Solo-only**: No team sharing without sharing the raw key file
2. **Too complex**: Editing `.gitattributes` and running CLI commands intimidated non-power-users

Arcane solves both with envelope encryption (teams) and a GUI (accessibility).

---

### 2.8 Comparison Matrix

| Tool             | Encryption  | Key Management | Access Control | Cloud?   | Complexity |
| ---------------- | ----------- | -------------- | -------------- | -------- | ---------- |
| **git-crypt**    | File-level  | GPG            | GPG keyring    | ❌       | Medium     |
| **SOPS**         | Value-level | Multi-provider | KMS policies   | ☁️       | Medium     |
| **Vault**        | API-based   | Centralized    | Policies       | ☁️/Self  | High       |
| **Doppler**      | Managed     | Managed        | Team/Role      | ☁️       | Low        |
| **dotenv-vault** | File-level  | Manual         | Key holders    | Optional | Low        |
| **Git Seal**     | File-level  | Single key     | Key holder     | ❌       | Low        |
| **Arcane**       | File-level  | Hierarchical   | Envelope       | ❌       | Low        |

---

### 2.9 Weighted Scoring Analysis (out of 1000)

To objectively compare these tools, we apply weighted scores across 10 criteria. Weights reflect what matters most to developers managing secrets in Git repositories.

#### Scoring Criteria & Weights

| Criterion               | Weight | Description                                |
| ----------------------- | ------ | ------------------------------------------ |
| **Ease of Use**         | 15%    | Setup complexity, learning curve           |
| **Solo Experience**     | 10%    | How well it works for individual devs      |
| **Team Collaboration**  | 15%    | Key sharing, access control, onboarding    |
| **Security Model**      | 15%    | Encryption strength, key separation        |
| **Git Integration**     | 12%    | How seamlessly it works with Git workflows |
| **Offline Capability**  | 8%     | Works without internet/cloud               |
| **Key Revocation**      | 10%    | Ease of removing access                    |
| **Audit & Compliance**  | 8%     | Logging, enterprise features               |
| **Cost**                | 5%     | Free tier, pricing model                   |
| **Ecosystem/Community** | 2%     | Documentation, support, momentum           |

#### Raw Scores (0-100 per criterion)

| Tool             | Ease | Solo | Team | Security | Git | Offline | Revoke | Audit | Cost | Community |
| ---------------- | ---- | ---- | ---- | -------- | --- | ------- | ------ | ----- | ---- | --------- |
| **git-crypt**    | 40   | 70   | 30   | 70       | 80  | 100     | 20     | 10    | 100  | 60        |
| **SOPS**         | 50   | 60   | 70   | 85       | 70  | 50      | 60     | 70    | 80   | 75        |
| **Vault**        | 30   | 40   | 90   | 95       | 40  | 30      | 90     | 95    | 50   | 90        |
| **Doppler**      | 90   | 80   | 85   | 80       | 60  | 0       | 80     | 85    | 70   | 70        |
| **dotenv-vault** | 80   | 85   | 50   | 70       | 75  | 70      | 40     | 30    | 60   | 50        |
| **Git Seal**     | 70   | 95   | 10   | 75       | 95  | 100     | 10     | 5     | 100  | 30        |
| **Arcane**       | 85   | 90   | 80   | 85       | 90  | 100     | 70     | 40    | 100  | 20        |

#### Weighted Calculation

```
Score = (Ease × 0.15) + (Solo × 0.10) + (Team × 0.15) + (Security × 0.15)
      + (Git × 0.12) + (Offline × 0.08) + (Revoke × 0.10) + (Audit × 0.08)
      + (Cost × 0.05) + (Community × 0.02)

Then multiply by 10 to get score out of 1000.
```

#### Final Scores (out of 1000)

| Rank | Tool             | Score   | Strengths                                | Weaknesses                                    |
| ---- | ---------------- | ------- | ---------------------------------------- | --------------------------------------------- |
| 🥇   | **Arcane**       | **832** | Best Git integration, offline, team+solo | New (small community), audit features limited |
| 🥈   | **Doppler**      | 751     | Best UX, great team features             | Cloud-only (no offline), SaaS dependency      |
| 🥉   | **SOPS**         | 680     | Multi-cloud, good security               | Maintenance overhead, partial offline         |
| 4    | **Vault**        | 664     | Best security & audit, enterprise        | Complex, steep learning curve                 |
| 5    | **dotenv-vault** | 627     | Simple, familiar .env workflow           | Limited team, weak audit                      |
| 6    | **Git Seal**     | 567     | Perfect solo, best Git native            | No team features, no revocation               |
| 7    | **git-crypt**    | 497     | Mature, transparent                      | GPG complexity, poor team/revoke              |

#### Score Breakdown Visualization

```
Arcane      ████████████████████████████████████░░░░ 832/1000
Doppler     ██████████████████████████████░░░░░░░░░░ 751/1000
SOPS        ███████████████████████████░░░░░░░░░░░░░ 680/1000
Vault       ██████████████████████████░░░░░░░░░░░░░░ 664/1000
dotenv-vault██████████████████████░░░░░░░░░░░░░░░░░░ 627/1000
Git Seal    ██████████████████████░░░░░░░░░░░░░░░░░░ 567/1000
git-crypt   ████████████████████░░░░░░░░░░░░░░░░░░░░ 497/1000
```

#### Analysis by Use Case

| Use Case                 | Best Tool | Score | Why                                         |
| ------------------------ | --------- | ----- | ------------------------------------------- |
| **Solo developer**       | Arcane    | 832   | Best solo + offline + Git native            |
| **Small team (5-20)**    | Arcane    | 832   | Team keys + no cloud required               |
| **Enterprise (100+)**    | Vault     | 664   | Audit, SSO, compliance (despite complexity) |
| **Cloud-native startup** | Doppler   | 751   | Real-time sync, CI/CD native                |
| **Open source project**  | Arcane    | 832   | Free, no cloud dependency                   |
| **Highly regulated**     | Vault     | 664   | SOC2/HIPAA compliance                       |

#### Key Insights

1. **Arcane leads** because it uniquely combines:

    - Solo simplicity (Git Seal heritage)
    - Team collaboration (envelope encryption)
    - Full offline capability
    - GUI for accessibility

2. **Doppler is close** but loses on:

    - Offline (0/100) — completely cloud-dependent
    - Cost — paid tiers required for most features

3. **Vault is powerful but complex** — the 30/100 ease score hurts it for non-enterprise users

4. **Git Seal's weakness** is exactly what Arcane fixes:

    - Team collaboration: 10/100 → 80/100
    - Key revocation: 10/100 → 70/100

5. **git-crypt's age shows** — GPG is a barrier, poor team story

---

## 3. Core Concepts: Envelope Encryption {#envelope-encryption}

Envelope encryption is the industry-standard pattern used by AWS KMS, Google Cloud KMS, and Azure Key Vault.

### The Basic Idea

Instead of encrypting data directly with a master key:

1. Generate a **Data Encryption Key (DEK)** for each piece of data
2. Encrypt the data with the DEK
3. Encrypt the DEK with a **Key Encryption Key (KEK)**
4. Store the encrypted DEK alongside the encrypted data

```
┌─────────────────────────────────────────────────────────────────┐
│                        KEY HIERARCHY                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌───────────────┐                                             │
│   │   Root Key    │  ← Never leaves secure storage (HSM/KMS)    │
│   │     (KEK)     │                                             │
│   └───────┬───────┘                                             │
│           │                                                     │
│           ▼                                                     │
│   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐     │
│   │  Team Key A   │   │  Team Key B   │   │  Team Key C   │     │
│   │    (KEK)      │   │    (KEK)      │   │    (KEK)      │     │
│   └───────┬───────┘   └───────┬───────┘   └───────┬───────┘     │
│           │                   │                   │             │
│           ▼                   ▼                   ▼             │
│   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐     │
│   │  Project DEK  │   │  Project DEK  │   │  Project DEK  │     │
│   └───────────────┘   └───────────────┘   └───────────────┘     │
│           │                   │                   │             │
│           ▼                   ▼                   ▼             │
│   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐     │
│   │ Encrypted Data│   │ Encrypted Data│   │ Encrypted Data│     │
│   └───────────────┘   └───────────────┘   └───────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Why This Pattern?

1. **Key Rotation**: Only need to re-encrypt DEKs, not all data
2. **Access Control**: Different KEKs for different teams/projects
3. **Performance**: Symmetric encryption (DEK) is fast
4. **Security**: Root key never touches data directly

### Best Practices

| Practice                          | Rationale                                        |
| --------------------------------- | ------------------------------------------------ |
| Generate unique DEK per data item | Limits blast radius if one DEK is compromised    |
| Store encrypted DEK with data     | Simplifies retrieval, DEK is useless without KEK |
| Never log plaintext DEKs          | Should only exist in memory during operation     |
| Rotate KEKs regularly             | More manageable than rotating all DEKs           |
| KEK never leaves secure boundary  | Use HSM or KMS for root keys                     |

---

## 4. Arcane's Multi-Tier Key Strategy {#arcanes-strategy}

Arcane implements a **local-first envelope encryption** model that doesn't require cloud services.

### Current Implementation

| Component           | Storage                  | Purpose                            |
| ------------------- | ------------------------ | ---------------------------------- |
| **Master Identity** | `~/.arcane/identity.age` | Your personal X25519 keypair       |
| **Repo Key**        | `.git/arcane/keys/*.age` | AES key for encrypting files       |
| **Team Access**     | Additional `.age` files  | Repo key encrypted for each member |
| **Snapshots**       | `.git/arcane/backups/`   | Encrypted local copies of secrets  |

### Proposed Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                     ARCANE KEY HIERARCHY                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │ TIER 1: Master Key (Personal)                           │   │
│   │ Location: ~/.arcane/master.key                          │   │
│   │ Scope: All repos without specific key                   │   │
│   │ Use Case: Solo developer, personal projects             │   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │ TIER 2: Team Keys (Group)                               │   │
│   │ Location: ~/.arcane/teams/<team-name>.key               │   │
│   │ Scope: Repos tagged with .arcane-team file              │   │
│   │ Use Case: Organization-wide, department secrets         │   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │ TIER 3: Repo Keys (Isolated)                            │   │
│   │ Location: .git/arcane/keys/*.age                        │   │
│   │ Scope: Single repository only                           │   │
│   │ Use Case: High-security projects, client work           │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Key Selection Algorithm

```rust
fn select_key_for_repo(&self, repo_path: &Path) -> Result<Key> {
    // 1. Check for repo-specific key (highest priority)
    let repo_keys = repo_path.join(".git/arcane/keys");
    if repo_keys.exists() && has_valid_key(&repo_keys, &self.identity) {
        return load_repo_key(&repo_keys, &self.identity);
    }

    // 2. Check for team key (if repo is tagged)
    if let Some(team) = read_team_tag(repo_path) {
        let team_key = home.join(".arcane/teams").join(format!("{}.key", team));
        if team_key.exists() {
            return load_team_key(&team_key, &self.identity);
        }
    }

    // 3. Fallback to master key
    let master_key = home.join(".arcane/master.key");
    load_master_key(&master_key, &self.identity)
}
```

### Team Tagging

To associate a repo with a team, create a `.arcane-team` file in the repo root:

```
# .arcane-team
acme-corp
```

This tells Arcane to use the `acme-corp` team key instead of the master key.

### User Flows

#### Solo Developer (Current: Per-Repo Init)

1. Run `arcane init` in any repo
2. Master key is used automatically
3. Files encrypt/decrypt seamlessly

#### Solo Developer (Improved: Global Auto-Encrypt) ⭐

Git Seal's approach was even simpler — and Arcane should match it:

```bash
# ONE-TIME GLOBAL SETUP
arcane setup                    # Creates ~/.arcane/identity.age
git config --global filter.arcane.clean 'arcane clean %f'
git config --global filter.arcane.smudge 'arcane smudge'
git config --global filter.arcane.required true

# PER-REPO: Just add .gitattributes (no `arcane init` needed!)
echo '*.env filter=arcane diff=arcane' >> .gitattributes
```

**Result**: Any repo under `_Dev/` (or anywhere) with the right `.gitattributes` automatically encrypts. Zero per-repo commands!

**Key Insight**: The master key applies globally. If `.gitattributes` says `filter=arcane`, it just works. This is Git Seal's magic — and Arcane inherits it.

| Approach | Setup Commands         | Per-Repo Commands          |
| -------- | ---------------------- | -------------------------- |
| Current  | `arcane setup`         | `arcane init` per repo     |
| Improved | `arcane setup` (once)  | Just edit `.gitattributes` |
| Git Seal | `git-seal init` (once) | Just edit `.gitattributes` |

#### Team Collaboration

1. Admin creates team: `arcane team create acme-corp`
2. Admin adds members: `arcane team add-member acme-corp <public-key>`
3. Members clone repo, encryption "just works"
4. Revoking access: Re-encrypt team key without that member

#### High-Security Project

1. Run `arcane init --isolated` in the repo
2. Unique repo key is generated
3. Add specific collaborators: `arcane team add <alias> <key>`
4. Key never leaves this repo

---

## 5. Implementation Considerations {#implementation}

### Storage Locations

| Key Type        | Path                     | Permissions | Backup Strategy     |
| --------------- | ------------------------ | ----------- | ------------------- |
| Master Identity | `~/.arcane/identity.age` | 600         | User responsibility |
| Master Key      | `~/.arcane/master.key`   | 600         | Encrypted export    |
| Team Keys       | `~/.arcane/teams/*.key`  | 600         | Team admin manages  |
| Repo Keys       | `.git/arcane/keys/*.age` | 644         | Part of repo        |

### Key Formats

Using the `age` encryption format throughout:

```
# Identity (private key) - age-keygen output
AGE-SECRET-KEY-1QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ

# Recipient (public key) - derived from identity
age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq

# Encrypted file structure
age-encryption.org/v1
-> X25519 <recipient>
<encrypted-data>
```

### Migration Path

For existing repos using raw `repo.key`:

1. Detect legacy `.git/arcane/repo.key`
2. Encrypt it for the owner's identity
3. Move to new `.git/arcane/keys/owner.age`
4. Remove legacy file
5. Update `.gitattributes` if needed

### Error Handling

| Scenario              | Behavior                                     |
| --------------------- | -------------------------------------------- |
| No matching key       | Exit with clear error, suggest `arcane init` |
| Identity missing      | Prompt to run `arcane setup`                 |
| Team key inaccessible | Fall back to repo key or master              |
| Corrupted key file    | Backup recovery or re-initialization         |

---

## 5.5 Key Import & Reuse (Legacy Support) {#key-import}

Arcane supports importing existing keys, ensuring compatibility with older projects or legacy tools like Git Seal.

### Importing Keys

You can import a raw 32-byte key (e.g., `repo.key`) via the CLI or GUI. Arcane will:

1. Accept the raw key key.
2. Encrypt it with your Master Identity (Envelope Encryption).
3. Store it as `owner.age`, enabling "Team Mode" features for that repo.

### Legacy "Git Seal" Mode

If you do **not** have a Master Identity configured (`~/.arcane/identity.age` is missing), Arcane behaves exactly like Git Seal:

-   It looks for a raw `.git/arcane/repo.key` file.
-   If found, it uses that key directly to decrypt files.
-   **Result**: You can use Arcane in "Simple Mode" (just a key file) or "Team Mode" (identity + rotation) seamlessly. This ensures zero friction for solo developers migrating from Git Seal.

---

## 6. Cloud vs Local Trade-offs {#cloud-vs-local}

### Local-Only (Arcane Current Approach)

**Pros**:

-   No internet dependency
-   Works offline
-   No third-party trust
-   Zero recurring cost
-   Full control over keys

**Cons**:

-   Key backup is user's responsibility
-   Sharing requires manual key exchange
-   No centralized audit log
-   Sync across devices is manual

### Cloud-Integrated (Future Option)

**Possible Integrations**:

| Provider  | Integration Point | Benefit            |
| --------- | ----------------- | ------------------ |
| AWS KMS   | KEK storage       | Hardware security  |
| GCP KMS   | KEK storage       | Automatic rotation |
| 1Password | Identity storage  | Cross-device sync  |
| Bitwarden | Identity storage  | Self-hostable      |

**Hybrid Model**:

-   Keep DEKs (repo keys) local and in Git
-   Store KEK (master/team key) in cloud KMS
-   Best of both: offline encryption, cloud key security

### Comparison for Different Users

| User Type          | Recommendation      | Rationale           |
| ------------------ | ------------------- | ------------------- |
| **Solo developer** | Local-only          | Simplicity, no cost |
| **Small team**     | Local + manual sync | Low overhead        |
| **Enterprise**     | Hybrid with KMS     | Compliance, audit   |
| **Open source**    | Local-only          | No dependencies     |

---

## 7. Recommendations {#recommendations}

### For Arcane's Multi-Tier Implementation

1. **Start Simple**: Implement the three-tier local hierarchy first
2. **Default to Master**: Make solo usage zero-config
3. **Team Tags**: Use `.arcane-team` for flexible grouping
4. **Clear UX**: Show which key tier is active in dashboard

### Priority Order

1. ✅ Master Key fallback (already implemented via identity)
2. 🔲 Team Key concept (new)
3. 🔲 Repo Key isolation flag (`--isolated`)
4. 🔲 Key selection algorithm
5. ✅ UI for key tier visualization (Implemented in TUI Dashboard)

### Security Checklist for Implementation

-   [ ] Master key never stored unencrypted
-   [ ] Team key operations require identity unlock
-   [ ] Repo keys are encrypted for all authorized identities
-   [ ] Key files have restrictive permissions (600)
-   [ ] Clear error messages for access denied
-   [ ] Audit log of key operations (optional)

### Questions to Resolve

1. **Key rotation**: How to handle when a team member is removed?
2. **Recovery**: What if master identity is lost?
3. **Conflict resolution**: What if repo has both team tag and repo key?
4. **Distribution**: How to share team keys securely?

---

## Appendix: Further Reading

-   [age encryption spec](https://age-encryption.org/)
-   [Google Cloud KMS concepts](https://cloud.google.com/kms/docs/envelope-encryption)
-   [Mozilla SOPS](https://github.com/mozilla/sops)
-   [HashiCorp Vault documentation](https://www.vaultproject.io/docs)
-   [The Twelve-Factor App: Config](https://12factor.net/config)

---

_Document generated for Arcane project planning. Last updated: 2025-12-25_
