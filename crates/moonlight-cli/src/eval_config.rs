use anyhow::{bail, Context};
use moonlight_core::config::{
    normalize_timeout, DEFAULT_MAX_BODY_CAPTURE_BYTES, DEFAULT_TARGET_TIMEOUT_MS,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Clone)]
pub(crate) struct ProjectEvalConfig {
    pub(crate) project: ProjectSection,
    pub(crate) eval: EvalSection,
    pub(crate) checks: Vec<CheckConfig>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectSection {
    pub(crate) name: String,
    pub(crate) repo: PathBuf,
    pub(crate) baseline_ref: String,
}

#[derive(Debug, Clone)]
pub(crate) struct EvalSection {
    pub(crate) work_dir: PathBuf,
    pub(crate) keep_worktrees: KeepWorktrees,
    pub(crate) jobs: usize,
    pub(crate) target_timeout_ms: u64,
    pub(crate) max_body_capture_bytes: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct CheckConfig {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) command: CheckCommand,
    pub(crate) cwd: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) ignore_stdout: bool,
    pub(crate) ignore_stderr: bool,
    pub(crate) normalize_stdout_patterns: Vec<String>,
    pub(crate) normalize_stderr_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum CheckCommand {
    Shell(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeepWorktrees {
    Never,
    Failed,
    Always,
}

impl FromStr for KeepWorktrees {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "never" => Ok(Self::Never),
            "failed" => Ok(Self::Failed),
            "always" => Ok(Self::Always),
            other => bail!("invalid keep_worktrees {other:?}; use never, failed, or always"),
        }
    }
}

impl ProjectEvalConfig {
    pub(crate) fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let raw: RawProjectEvalConfig =
            toml::from_str(&content).with_context(|| format!("invalid {}", path.display()))?;
        raw.validate()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectEvalConfig {
    project: RawProjectSection,
    #[serde(default)]
    eval: RawEvalSection,
    checks: Vec<RawCheckConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectSection {
    name: String,
    #[serde(default = "default_repo")]
    repo: PathBuf,
    baseline_ref: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEvalSection {
    work_dir: Option<PathBuf>,
    keep_worktrees: Option<String>,
    jobs: Option<usize>,
    target_timeout_ms: Option<u64>,
    max_body_capture_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCheckConfig {
    id: String,
    name: Option<String>,
    command: Option<String>,
    argv: Option<Vec<String>>,
    cwd: Option<PathBuf>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    timeout_ms: Option<u64>,
    #[serde(default)]
    ignore_stdout: bool,
    #[serde(default)]
    ignore_stderr: bool,
    #[serde(default)]
    normalize_stdout_patterns: Vec<String>,
    #[serde(default)]
    normalize_stderr_patterns: Vec<String>,
}

impl RawProjectEvalConfig {
    fn validate(self) -> anyhow::Result<ProjectEvalConfig> {
        if self.project.name.trim().is_empty() {
            bail!("[project].name must not be empty");
        }
        if self.project.baseline_ref.trim().is_empty() {
            bail!("[project].baseline_ref must not be empty");
        }
        if self.checks.is_empty() {
            bail!("at least one [[checks]] entry is required");
        }

        let checks = self
            .checks
            .into_iter()
            .map(RawCheckConfig::validate)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ProjectEvalConfig {
            project: ProjectSection {
                name: self.project.name,
                repo: self.project.repo,
                baseline_ref: self.project.baseline_ref,
            },
            eval: EvalSection {
                work_dir: self
                    .eval
                    .work_dir
                    .unwrap_or_else(|| PathBuf::from(".moonlight/evals")),
                keep_worktrees: self
                    .eval
                    .keep_worktrees
                    .as_deref()
                    .unwrap_or("failed")
                    .parse()?,
                jobs: self.eval.jobs.unwrap_or(1).max(1),
                target_timeout_ms: self
                    .eval
                    .target_timeout_ms
                    .map(normalize_timeout)
                    .unwrap_or(DEFAULT_TARGET_TIMEOUT_MS),
                max_body_capture_bytes: self
                    .eval
                    .max_body_capture_bytes
                    .unwrap_or(DEFAULT_MAX_BODY_CAPTURE_BYTES),
            },
            checks,
        })
    }
}

impl RawCheckConfig {
    fn validate(self) -> anyhow::Result<CheckConfig> {
        if self.id.trim().is_empty() {
            bail!("check id must not be empty");
        }
        let command = match (self.command, self.argv) {
            (Some(_), Some(_)) => bail!(
                "check {} must provide exactly one of command or argv",
                self.id
            ),
            (Some(command), None) => {
                if command.trim().is_empty() {
                    bail!("check {} command must not be empty", self.id);
                }
                CheckCommand::Shell(command)
            }
            (None, Some(argv)) => {
                if argv.is_empty() {
                    bail!("check {} argv must not be empty", self.id);
                }
                if argv[0].trim().is_empty() {
                    bail!("check {} argv command must not be empty", self.id);
                }
                CheckCommand::Argv(argv)
            }
            (None, None) => bail!("check {} must provide command or argv", self.id),
        };

        Ok(CheckConfig {
            id: self.id,
            name: self.name.filter(|value| !value.trim().is_empty()),
            command,
            cwd: self.cwd.unwrap_or_else(|| PathBuf::from(".")),
            env: self.env,
            timeout_ms: self.timeout_ms.map(normalize_timeout),
            ignore_stdout: self.ignore_stdout,
            ignore_stderr: self.ignore_stderr,
            normalize_stdout_patterns: self.normalize_stdout_patterns,
            normalize_stderr_patterns: self.normalize_stderr_patterns,
        })
    }
}

fn default_repo() -> PathBuf {
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_eval_config() {
        let config: RawProjectEvalConfig = toml::from_str(
            r#"
            [project]
            name = "demo"
            baseline_ref = "main"

            [[checks]]
            id = "test"
            command = "cargo test"
            "#,
        )
        .unwrap();

        let config = config.validate().unwrap();
        assert_eq!(config.project.name, "demo");
        assert_eq!(config.eval.keep_worktrees, KeepWorktrees::Failed);
        assert_eq!(config.checks[0].id, "test");
    }

    #[test]
    fn rejects_check_with_both_command_forms() {
        let config: RawProjectEvalConfig = toml::from_str(
            r#"
            [project]
            name = "demo"
            baseline_ref = "main"

            [[checks]]
            id = "test"
            command = "cargo test"
            argv = ["cargo", "test"]
            "#,
        )
        .unwrap();

        assert!(config.validate().is_err());
    }
}
