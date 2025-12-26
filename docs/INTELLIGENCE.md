# Sovereign Intelligence (Autonomous AI Commits)

Arcane can **automatically commit your changes** with intelligent, AI-generated messages.
This feature runs locally using [Ollama](https://ollama.com), keeping your code private and your servers safe.

## Logic: Zero-Click Commits

When you run `arcane start`, Arcane acts as an intelligent daemon:

1.  **Watches** for file changes.
2.  **Waits** for a "quiet period" (5 seconds).
3.  **Analyzes** the `git diff`.
4.  **Generates** a concise commit message using the "Senior Engineer" persona.
5.  **Loop Prevention**: Ignores changes that are _only_ version bumps to prevent infinite commit loops.
6.  **Extracts** the message using Zero-Tolerance logic (no chatter, no preambles).
7.  **Commits** the changes automatically.

## Prerequisites

1.  **Install Ollama**: [ollama.com](https://ollama.com)
2.  **Pull Model**: `ollama pull qwen2.5:1.5b` (Recommended) or `llama3`.
3.  **Start Ollama**: `ollama serve`

## Usage

### Start the Intelligence Daemon

Run this in your project root while you code:

```bash
arcane start
```

(You will see logs like `📝 Change detected` -> `✅ Auto-committed: "Refactored user auth logic"`)

## Configuration

Arcane defaults to `Ollama (qwen2.5:1.5b)`.
You can customize this in `~/.arcane/config.toml` (if implemented) or environment variables.

## Safety Architecture

**Server Safe**: This feature is strictly isolated to the `start` command.
The standard `arcane run` command (used on servers) **never** attempts to connect to AI providers or watch files.

---

## Smart Squash (Git History Management)

When you've accumulated many small auto-commits, use **Smart Squash** to clean your history before pushing.

### Strategies

| Mode             | Trigger | Result                                  |
| ---------------- | ------- | --------------------------------------- |
| **Auto-Push ON** | Toggle  | Each commit = Patch                     |
| **Smart Squash** | `s`     | AI groups into Minors + Patches         |
| **Bulk Squash**  | `l`     | All → 1 Major/Minor bump                |
| **Auto-Deploy**  | `d`     | Toggle Deployment to Authorized Servers |

### Usage

1. Accumulate commits with Auto-Commit (keep Auto-Push OFF)
2. Open Dashboard: `arcane dashboard`
3. Press `s` (Smart) or `l` (Bulk)
4. Review the plan → `Enter` to execute, `Esc` to cancel

### Configuration

-   **Bulk Squash Mode**: Defaults to Major bump. Press `m` in Versioning tab to toggle to Minor.
-   A backup branch (`before-squash`) is created automatically.
