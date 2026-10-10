mod capture;
mod diff;
mod json_path;

use crate::{target::CapturedTarget, Classification, ComparisonSummary, DiffEntry, DiffKind};
use std::collections::{HashMap, HashSet};

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
    // Normalize once at the public boundary, including directly modified public
    // config fields. Evidence lookup stays constant-time for every visited node.
    let mut normalized = config.clone();
    normalized.ignore_json_paths = config
        .ignore_json_paths
        .iter()
        .filter_map(|path| json_path::canonical_exact_path(path))
        .collect();
    normalized.redact_json_paths = config
        .redact_json_paths
        .iter()
        .filter_map(|path| json_path::canonical_exact_path(path))
        .collect();
    let config = &normalized;
    let candidate_pairs = diff::diff_pair(primary, candidate, diff::TargetRole::Candidate, config);
    let reference_pairs = secondary
        .map(|secondary| diff::diff_pair(primary, secondary, diff::TargetRole::Secondary, config))
        .unwrap_or_default();
    let noise_filtered_diffs = filter_candidate_diffs(&candidate_pairs, &reference_pairs);

    let target_error = primary.observation.error.is_some()
        || candidate.observation.error.is_some()
        || secondary
            .and_then(|target| target.observation.error.as_ref())
            .is_some();

    let classification = if target_error {
        Classification::TargetError
    } else if candidate_pairs.is_empty() && reference_pairs.is_empty() {
        Classification::Match
    } else if noise_filtered_diffs.is_empty() {
        Classification::ReferenceNoise
    } else if !reference_pairs.is_empty() {
        Classification::SuspiciousWithNoise
    } else {
        Classification::SuspiciousDifference
    };

    let raw_candidate_diffs = evidence_entries(candidate_pairs.iter());
    let reference_noise = evidence_entries(reference_pairs.iter());

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
    candidate_diffs: &[diff::PairDiff],
    reference_noise: &[diff::PairDiff],
) -> Vec<DiffEntry> {
    let reference_index: HashMap<(DiffKind, String), Option<String>> = reference_noise
        .iter()
        .map(|reference_diff| {
            (
                (
                    reference_diff.entry.kind.clone(),
                    reference_diff.semantic_path.clone(),
                ),
                reference_diff.semantic_other.clone(),
            )
        })
        .collect();

    evidence_entries(candidate_diffs.iter().filter(|candidate_diff| {
        candidate_diff.semantic_other != candidate_diff.semantic_primary
            && reference_index
                .get(&(
                    candidate_diff.entry.kind.clone(),
                    candidate_diff.semantic_path.clone(),
                ))
                .is_none_or(|secondary| secondary != &candidate_diff.semantic_other)
    }))
}

fn evidence_entries<'a>(diffs: impl IntoIterator<Item = &'a diff::PairDiff>) -> Vec<DiffEntry> {
    let mut seen = HashSet::new();
    diffs
        .into_iter()
        .filter(|diff| seen.insert((diff.entry.kind.clone(), diff.entry.path.clone())))
        .map(|diff| diff.entry.clone())
        .collect()
}

#[cfg(test)]
mod tests;
