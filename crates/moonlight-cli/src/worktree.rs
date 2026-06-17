use anyhow::{bail, Context};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub(crate) struct EvalWorktrees {
    pub(crate) root: PathBuf,
    pub(crate) primary: PathBuf,
    pub(crate) candidate: PathBuf,
}

impl EvalWorktrees {
    pub(crate) fn paths(work_dir: &Path, eval_id: Uuid) -> Self {
        let root = work_dir.join(eval_id.to_string());
        Self {
            primary: root.join("primary"),
            candidate: root.join("candidate"),
            root,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CandidateSource {
    Ref(String),
    Patch(PathBuf),
}

impl CandidateSource {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Ref(value) => format!("ref {value}"),
            Self::Patch(path) => format!("patch {}", path.display()),
        }
    }
}

pub(crate) fn resolve_repo_root(repo: &Path) -> anyhow::Result<PathBuf> {
    let output = git(repo, ["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output.trim()))
}

pub(crate) fn prepare_worktrees(
    repo_root: &Path,
    work_dir: &Path,
    eval_id: Uuid,
    baseline_ref: &str,
    candidate: &CandidateSource,
) -> anyhow::Result<EvalWorktrees> {
    let paths = EvalWorktrees::paths(work_dir, eval_id);
    if paths.root.exists() {
        bail!(
            "eval work directory already exists: {}",
            paths.root.display()
        );
    }
    fs::create_dir_all(&paths.root)?;

    add_worktree(repo_root, &paths.primary, baseline_ref)?;
    match candidate {
        CandidateSource::Ref(candidate_ref) => {
            add_worktree(repo_root, &paths.candidate, candidate_ref)?;
        }
        CandidateSource::Patch(patch) => {
            let patch = patch
                .canonicalize()
                .with_context(|| format!("failed to resolve patch {}", patch.display()))?;
            add_worktree(repo_root, &paths.candidate, baseline_ref)?;
            git_checked(
                &paths.candidate,
                Command::new("git")
                    .arg("-C")
                    .arg(&paths.candidate)
                    .arg("apply")
                    .arg(&patch),
            )
            .with_context(|| format!("failed to apply patch {}", patch.display()))?;
        }
    }

    Ok(paths)
}

pub(crate) fn cleanup_worktrees(repo_root: &Path, paths: &EvalWorktrees) -> anyhow::Result<()> {
    remove_worktree(repo_root, &paths.primary)?;
    remove_worktree(repo_root, &paths.candidate)?;
    if paths.root.exists() {
        fs::remove_dir_all(&paths.root)
            .with_context(|| format!("failed to remove {}", paths.root.display()))?;
    }
    Ok(())
}

fn add_worktree(repo_root: &Path, path: &Path, reference: &str) -> anyhow::Result<()> {
    git_checked(
        repo_root,
        Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .arg("worktree")
            .arg("add")
            .arg("--detach")
            .arg(path)
            .arg(reference),
    )
    .with_context(|| {
        format!(
            "failed to create worktree {} at {}",
            path.display(),
            reference
        )
    })
}

fn remove_worktree(repo_root: &Path, path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(path)
        .status();
    if status.as_ref().is_err() || !status.as_ref().is_ok_and(|status| status.success()) {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove worktree {}", path.display()))?;
    }
    Ok(())
}

fn git<const N: usize>(repo: &Path, args: [&str; N]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
}

fn git_checked(_repo: &Path, command: &mut Command) -> anyhow::Result<()> {
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_paths_are_eval_scoped() {
        let eval_id = Uuid::nil();
        let paths = EvalWorktrees::paths(Path::new(".moonlight/evals"), eval_id);

        assert_eq!(
            paths.primary,
            PathBuf::from(".moonlight/evals")
                .join(eval_id.to_string())
                .join("primary")
        );
        assert_eq!(paths.candidate, paths.root.join("candidate"));
    }
}
