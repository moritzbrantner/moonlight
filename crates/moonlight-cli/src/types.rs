use crate::config::CliDefaults;
use moonlight_core::{compare::CapturedTarget, Classification};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use moonlight_core::compare::CompareConfig;

#[derive(Debug, Clone)]
pub(crate) struct Case {
    pub(crate) primary: TargetCommand,
    pub(crate) candidate: TargetCommand,
    pub(crate) secondary: Option<TargetCommand>,
    pub(crate) max_body_capture_bytes: usize,
    pub(crate) ignore_json_paths: Vec<String>,
    pub(crate) ignore_json_path_patterns: Vec<String>,
    pub(crate) redact_json_paths: Vec<String>,
    pub(crate) redact_json_path_patterns: Vec<String>,
    pub(crate) ignore_headers: Vec<String>,
    pub(crate) ignore_stderr: bool,
    pub(crate) target_timeout_ms: u64,
}

impl Case {
    pub(crate) fn uses_default_compare_config(&self, defaults: &CliDefaults) -> bool {
        self.ignore_json_paths == defaults.ignore_json_paths
            && self.ignore_json_path_patterns == defaults.ignore_json_path_patterns
            && self.redact_json_paths == defaults.redact_json_paths
            && self.redact_json_path_patterns == defaults.redact_json_path_patterns
            && self.ignore_headers == defaults.ignore_headers
            && self.ignore_stderr == defaults.ignore_stderr
            && self.target_timeout_ms == defaults.target_timeout_ms
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TargetCommand {
    pub(crate) form: CommandForm,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) env: BTreeMap<String, String>,
}

impl TargetCommand {
    pub(crate) fn shell(command: String) -> Self {
        Self {
            form: CommandForm::Shell(command),
            cwd: None,
            env: BTreeMap::new(),
        }
    }

    pub(crate) fn argv(argv: Vec<String>) -> Self {
        Self {
            form: CommandForm::Argv(argv),
            cwd: None,
            env: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CommandForm {
    Shell(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCase {
    pub(crate) case: Case,
    pub(crate) compare_config: Arc<CompareConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BatchCase {
    #[serde(default)]
    pub(crate) primary: Option<String>,
    #[serde(default)]
    pub(crate) candidate: Option<String>,
    #[serde(default)]
    pub(crate) secondary: Option<String>,
    #[serde(default)]
    pub(crate) primary_argv: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) candidate_argv: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) secondary_argv: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) max_body_capture_bytes: Option<usize>,
    #[serde(default)]
    pub(crate) ignore_json_paths: Vec<String>,
    #[serde(default)]
    pub(crate) ignore_json_path_patterns: Vec<String>,
    #[serde(default)]
    pub(crate) redact_json_paths: Vec<String>,
    #[serde(default)]
    pub(crate) redact_json_path_patterns: Vec<String>,
    #[serde(default)]
    pub(crate) ignore_headers: Vec<String>,
    #[serde(default)]
    pub(crate) ignore_stderr: bool,
    #[serde(default)]
    pub(crate) target_timeout_ms: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct CapturedTargets {
    pub(crate) primary: CapturedTarget,
    pub(crate) candidate: CapturedTarget,
    pub(crate) secondary: Option<CapturedTarget>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct BatchSummary {
    pub(crate) total_runs: usize,
    pub(crate) matches: usize,
    pub(crate) suspicious_differences: usize,
    pub(crate) reference_noise: usize,
    pub(crate) suspicious_with_noise: usize,
    pub(crate) target_errors: usize,
    pub(crate) duration_ms: u128,
    pub(crate) jobs: usize,
}

impl BatchSummary {
    pub(crate) fn record(&mut self, classification: &Classification) {
        self.total_runs += 1;
        match classification {
            Classification::Match => self.matches += 1,
            Classification::SuspiciousDifference => self.suspicious_differences += 1,
            Classification::ReferenceNoise => self.reference_noise += 1,
            Classification::SuspiciousWithNoise => self.suspicious_with_noise += 1,
            Classification::TargetError => self.target_errors += 1,
        }
    }
}
