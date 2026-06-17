mod capture;
mod diff;
mod json_path;

use crate::{target::CapturedTarget, Classification, ComparisonSummary, DiffEntry};
use std::collections::HashSet;

pub use capture::{
    capture_body, capture_body_with_redaction_patterns, capture_body_with_redactions,
    capture_headers, is_hop_by_hop_header,
};

#[derive(Debug, Clone)]
pub struct CompareConfig {
    pub ignore_json_paths: HashSet<String>,
    pub ignore_json_path_patterns: Vec<String>,
    pub redact_json_paths: HashSet<String>,
    pub redact_json_path_patterns: Vec<String>,
    pub ignore_headers: HashSet<String>,
    pub ignore_stderr: bool,
}

impl CompareConfig {
    pub fn new(
        ignore_json_paths: &[String],
        ignore_headers: &[String],
        ignore_stderr: bool,
    ) -> Self {
        Self::new_with_redactions(ignore_json_paths, &[], ignore_headers, ignore_stderr)
    }

    pub fn new_with_redactions(
        ignore_json_paths: &[String],
        redact_json_paths: &[String],
        ignore_headers: &[String],
        ignore_stderr: bool,
    ) -> Self {
        Self::new_with_patterns(
            ignore_json_paths,
            &[],
            redact_json_paths,
            &[],
            ignore_headers,
            ignore_stderr,
        )
    }

    pub fn new_with_patterns(
        ignore_json_paths: &[String],
        ignore_json_path_patterns: &[String],
        redact_json_paths: &[String],
        redact_json_path_patterns: &[String],
        ignore_headers: &[String],
        ignore_stderr: bool,
    ) -> Self {
        Self {
            ignore_json_paths: ignore_json_paths.iter().cloned().collect(),
            ignore_json_path_patterns: ignore_json_path_patterns.to_vec(),
            redact_json_paths: redact_json_paths.iter().cloned().collect(),
            redact_json_path_patterns: redact_json_path_patterns.to_vec(),
            ignore_headers: ignore_headers
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            ignore_stderr,
        }
    }
}

pub fn compare_targets(
    primary: &CapturedTarget,
    candidate: &CapturedTarget,
    secondary: Option<&CapturedTarget>,
    config: &CompareConfig,
) -> ComparisonSummary {
    let raw_candidate_diffs =
        diff::diff_pair(primary, candidate, diff::TargetRole::Candidate, config);
    let reference_noise = secondary
        .map(|secondary| diff::diff_pair(primary, secondary, diff::TargetRole::Secondary, config))
        .unwrap_or_default();
    let noise_filtered_diffs = filter_candidate_diffs(&raw_candidate_diffs, &reference_noise);

    let target_error = primary.observation.error.is_some()
        || candidate.observation.error.is_some()
        || secondary
            .and_then(|target| target.observation.error.as_ref())
            .is_some();

    let classification = if target_error {
        Classification::TargetError
    } else if raw_candidate_diffs.is_empty() && reference_noise.is_empty() {
        Classification::Match
    } else if noise_filtered_diffs.is_empty() {
        Classification::ReferenceNoise
    } else if !reference_noise.is_empty() {
        Classification::SuspiciousWithNoise
    } else {
        Classification::SuspiciousDifference
    };

    ComparisonSummary {
        classification,
        raw_diff_summary: summarize("candidate", &raw_candidate_diffs),
        noise_summary: summarize("reference noise", &reference_noise),
        raw_candidate_diffs,
        reference_noise,
        noise_filtered_diffs,
    }
}

fn summarize(label: &str, diffs: &[DiffEntry]) -> String {
    if diffs.is_empty() {
        format!("no {label} diffs")
    } else {
        format!("{label}: {} diff(s)", diffs.len())
    }
}

fn filter_candidate_diffs(
    candidate_diffs: &[DiffEntry],
    reference_noise: &[DiffEntry],
) -> Vec<DiffEntry> {
    candidate_diffs
        .iter()
        .filter(|candidate_diff| {
            let Some(reference_diff) = reference_noise.iter().find(|reference_diff| {
                reference_diff.kind == candidate_diff.kind
                    && reference_diff.path == candidate_diff.path
            }) else {
                return true;
            };

            candidate_diff.candidate != candidate_diff.primary
                && candidate_diff.candidate != reference_diff.secondary
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
