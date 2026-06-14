use moonlight_core::{compare::CapturedTarget, Classification};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use moonlight_core::compare::CompareConfig;

#[derive(Debug, Clone)]
pub(crate) struct Case {
    pub(crate) primary: TargetCommand,
    pub(crate) candidate: TargetCommand,
    pub(crate) secondary: Option<TargetCommand>,
    pub(crate) max_body_capture_bytes: usize,
    pub(crate) ignored_json_paths: Vec<String>,
    pub(crate) ignored_headers: Vec<String>,
    pub(crate) ignore_stderr: bool,
}

impl Case {
    pub(crate) fn uses_default_compare_config(&self) -> bool {
        self.ignored_json_paths.is_empty() && self.ignored_headers.is_empty() && !self.ignore_stderr
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TargetCommand {
    Shell(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCase {
    pub(crate) case: Case,
    pub(crate) compare_config: Arc<CompareConfig>,
}

#[derive(Debug, Deserialize)]
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
    pub(crate) ignored_json_paths: Vec<String>,
    #[serde(default)]
    pub(crate) ignored_headers: Vec<String>,
    #[serde(default)]
    pub(crate) ignore_stderr: bool,
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
