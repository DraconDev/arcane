use crate::ai_service::SquashPlan;
use anyhow::{Context, Result};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct RebaseManager;

impl RebaseManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute_plan(
        &self,
        repo_path: &Path,
        plan: &SquashPlan,
        base_sha: &str,
    ) -> Result<()> {
        // 1. Generate the Todo List Content
        // The plan has groups. e.g. group 1: [A, B, C] -> pick A, squash B, squash C.
        // We need to map hashes to actions.
        // BUT `git rebase -i` gives us a list, and we edit it.
        // Or we can just overwrite the todo list if we know the hashes.

        let mut todo_lines = Vec::new();

        // We need to iterate groups in REVERSE order if the plan is newest-first?
        // Usually `git rebase -i` presents OLDEST first.
        // Our AI plan likely gave us groups in some order.
        // Let's assume the plan groups are in Newest -> Oldest order (check AI prompt).
        // AI prompt said "Commits (Newest First)".
        // So groups are likely Newest First.
        // `git rebase -i` wants Oldest First.
        // So we should reverse the groups.

        let mut groups = plan.groups.clone();
        groups.reverse(); // Now Oldest First

        for group in groups {
            // Group: [Hash1, Hash2, Hash3] (Assuming these are also sorted? Newest or Oldest?)
            // If the prompt gave "Newest First" list, checking group contents...
            // "Contiguous commits... 1 group".
            // If Hash1 is newer than Hash2, and grouped together...
            // We need to ensure we pick the OLDEST of the group as 'pick' (or 'reword')
            // and the others as 'squash' or 'fixup'.

            // We don't strictly know the order inside the group from the struct alone without checking dates,
            // but let's assume valid ordering or we fetch dates?
            // Safer: Just trust the list provided by AI match the input list (Newest First).
            // So in [H_new, H_mid, H_old], H_old is the base.

            let mut commits = group.commits.clone();
            // If input was Newest First, then H_old is at the end.
            // We want to process Oldest First.
            commits.reverse();

            if let Some(first) = commits.first() {
                // First commit (Oldest) gets "reword" (pick + edit message)
                // using the target message.
                // Actually, to set the message automatically, we can use "reset" or "label" tricks?
                // Or just 'pick' then 'squash' others?
                // To apply the NEW message, we can use `reword`.
                // BUT `reword` opens an editor.
                // We want to avoid editors.
                // Only way to avoid editor for message change is `git commit --amend -m` inside the rebase or similar.
                // OR:
                // pick H_old
                // fixup H_mid
                // fixup H_new
                // exec git commit --amend -m "New Group Message"

                todo_lines.push(format!("pick {}", first));

                for fixup in commits.iter().skip(1) {
                    todo_lines.push(format!("fixup {}", fixup));
                }

                // Set the new message
                todo_lines.push(format!(
                    "exec git commit --amend -m \"{}\"",
                    group.target_message
                ));
            }
        }

        let todo_content = todo_lines.join("\n");

        // 2. Create the Fake Editor Script
        // This script will be called by git with the path to the todo file.
        // It simply overwrites that file with our `todo_content`.

        let script_path = repo_path.join(".git/arcane_rebase_editor.sh");
        let script_content = format!(
            "#!/bin/sh\necho \"{}\" > $1",
            todo_content.replace("\"", "\\\"").replace("$", "\\$")
        );

        tokio::fs::write(&script_path, script_content).await?;

        let mut perms = std::fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)?;

        // 3. Run Git Rebase
        let output = Command::new("git")
            .current_dir(repo_path)
            .env("GIT_SEQUENCE_EDITOR", &script_path)
            .env("GIT_EDITOR", "true") // For any commit --amend that might pop up (though exec shouldn't)
            .args(&["rebase", "-i", base_sha])
            .output()
            .await?;

        // Cleanup
        let _ = tokio::fs::remove_file(script_path).await;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Verify if we are stuck in rebase
            let rebase_dir = repo_path.join(".git/rebase-merge");
            if rebase_dir.exists() {
                Command::new("git")
                    .current_dir(repo_path)
                    .args(&["rebase", "--abort"])
                    .output()
                    .await?;
            }
            return Err(anyhow::anyhow!("Rebase failed: {}", stderr));
        }

        Ok(())
    }
}
