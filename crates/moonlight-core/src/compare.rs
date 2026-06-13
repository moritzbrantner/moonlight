use crate::{
    BodyCapture, Classification, ComparisonSummary, DiffEntry, DiffKind, TargetObservation,
};
use bytes::Bytes;
use http::HeaderMap;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone)]
pub struct CapturedTarget {
    pub observation: TargetObservation,
    pub body_bytes: Bytes,
    pub stderr_bytes: Bytes,
}

#[derive(Debug, Clone)]
pub struct CompareConfig {
    pub ignored_json_paths: HashSet<String>,
    pub ignored_headers: HashSet<String>,
    pub ignore_stderr: bool,
}

impl CompareConfig {
    pub fn new(
        ignored_json_paths: &[String],
        ignored_headers: &[String],
        ignore_stderr: bool,
    ) -> Self {
        Self {
            ignored_json_paths: ignored_json_paths.iter().cloned().collect(),
            ignored_headers: ignored_headers
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            ignore_stderr,
        }
    }
}

pub fn capture_body(body: &[u8], max_bytes: usize) -> BodyCapture {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let preview_bytes = &body[..body.len().min(max_bytes)];
    BodyCapture {
        size_bytes: body.len(),
        sha256: hex::encode(hasher.finalize()),
        preview: String::from_utf8_lossy(preview_bytes).to_string(),
        truncated: body.len() > max_bytes,
    }
}

pub fn capture_headers(headers: &HeaderMap, redact_headers: &[String]) -> BTreeMap<String, String> {
    let redact: HashSet<String> = redact_headers
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();
    let mut captured = BTreeMap::new();
    for (name, value) in headers {
        let key = name.as_str().to_ascii_lowercase();
        if is_hop_by_hop_header(&key) {
            continue;
        }
        let header_value = if redact.contains(&key) {
            "[redacted]".to_string()
        } else {
            value.to_str().unwrap_or("[non-utf8]").to_string()
        };
        captured.insert(key, header_value);
    }
    captured
}

pub fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
    )
}

pub fn compare_targets(
    primary: &CapturedTarget,
    candidate: &CapturedTarget,
    secondary: Option<&CapturedTarget>,
    config: &CompareConfig,
) -> ComparisonSummary {
    let raw_candidate_diffs = diff_pair(primary, candidate, TargetRole::Candidate, config);
    let reference_noise = secondary
        .map(|secondary| diff_pair(primary, secondary, TargetRole::Secondary, config))
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

#[derive(Debug, Clone, Copy)]
enum TargetRole {
    Candidate,
    Secondary,
}

impl TargetRole {
    fn label(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Secondary => "secondary",
        }
    }

    fn values(self, value: Option<String>) -> (Option<String>, Option<String>) {
        match self {
            Self::Candidate => (value, None),
            Self::Secondary => (None, value),
        }
    }
}

fn diff_pair(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
) -> Vec<DiffEntry> {
    let mut diffs = Vec::new();
    diff_target_errors(primary, other, role, &mut diffs);
    diff_status(primary, other, role, &mut diffs);
    diff_headers(primary, other, role, config, &mut diffs);
    diff_bodies(primary, other, role, config, &mut diffs);
    diff_stderr(primary, other, role, config, &mut diffs);
    diffs
}

fn diff_target_errors(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    diffs: &mut Vec<DiffEntry>,
) {
    if primary.observation.error != other.observation.error {
        let (candidate, secondary) = role.values(other.observation.error.clone());
        diffs.push(DiffEntry {
            kind: DiffKind::TargetError,
            path: "$target_error".to_string(),
            primary: primary.observation.error.clone(),
            candidate,
            secondary,
            message: format!("primary target error differs from {}", role.label()),
        });
    }
}

fn diff_status(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    diffs: &mut Vec<DiffEntry>,
) {
    if primary.observation.status != other.observation.status {
        let (candidate, secondary) =
            role.values(other.observation.status.map(|value| value.to_string()));
        diffs.push(DiffEntry {
            kind: DiffKind::Status,
            path: "$status".to_string(),
            primary: primary.observation.status.map(|value| value.to_string()),
            candidate,
            secondary,
            message: format!("primary status differs from {}", role.label()),
        });
    }
}

fn diff_headers(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    let keys: BTreeSet<String> = primary
        .observation
        .headers
        .keys()
        .chain(other.observation.headers.keys())
        .filter(|name| !config.ignored_headers.contains(*name))
        .cloned()
        .collect();

    for key in keys {
        let primary_value = primary.observation.headers.get(&key).cloned();
        let other_value = other.observation.headers.get(&key).cloned();
        if primary_value != other_value {
            let (candidate, secondary) = role.values(other_value);
            diffs.push(DiffEntry {
                kind: DiffKind::Header,
                path: format!("$.headers.{key}"),
                primary: primary_value,
                candidate,
                secondary,
                message: format!("primary header {key} differs from {}", role.label()),
            });
        }
    }
}

fn diff_bodies(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    if primary.body_bytes == other.body_bytes {
        return;
    }

    let primary_json = serde_json::from_slice::<Value>(&primary.body_bytes);
    let other_json = serde_json::from_slice::<Value>(&other.body_bytes);

    match (primary_json, other_json) {
        (Ok(primary_json), Ok(other_json)) => {
            diff_json("$", &primary_json, &other_json, role, config, diffs);
        }
        _ => {
            let primary_text = normalize_text(&primary.body_bytes);
            let other_text = normalize_text(&other.body_bytes);
            if primary_text != other_text {
                let (candidate, secondary) = role.values(Some(other_text));
                diffs.push(DiffEntry {
                    kind: DiffKind::Body,
                    path: "$body".to_string(),
                    primary: Some(primary_text),
                    candidate,
                    secondary,
                    message: format!("primary body differs from {}", role.label()),
                });
            }
        }
    }
}

fn diff_stderr(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    if config.ignore_stderr {
        return;
    }
    if primary.stderr_bytes == other.stderr_bytes {
        return;
    }

    let primary_text = normalize_text(&primary.stderr_bytes);
    let other_text = normalize_text(&other.stderr_bytes);
    if primary_text != other_text {
        let (candidate, secondary) = role.values(Some(other_text));
        diffs.push(DiffEntry {
            kind: DiffKind::Stderr,
            path: "$stderr".to_string(),
            primary: Some(primary_text),
            candidate,
            secondary,
            message: format!("primary stderr differs from {}", role.label()),
        });
    }
}

fn diff_json(
    path: &str,
    primary: &Value,
    other: &Value,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    if config.ignored_json_paths.contains(path) {
        return;
    }

    match (primary, other) {
        (Value::Object(primary_map), Value::Object(other_map)) => {
            let keys: BTreeSet<String> = primary_map
                .keys()
                .chain(other_map.keys())
                .cloned()
                .collect();
            for key in keys {
                let child_path = if path == "$" {
                    format!("$.{key}")
                } else {
                    format!("{path}.{key}")
                };
                match (primary_map.get(&key), other_map.get(&key)) {
                    (Some(primary_value), Some(other_value)) => {
                        diff_json(&child_path, primary_value, other_value, role, config, diffs);
                    }
                    (primary_value, other_value) => {
                        push_json_diff(child_path, primary_value, other_value, role, diffs);
                    }
                }
            }
        }
        (Value::Array(primary_items), Value::Array(other_items)) => {
            let max_len = primary_items.len().max(other_items.len());
            for index in 0..max_len {
                let child_path = format!("{path}[{index}]");
                match (primary_items.get(index), other_items.get(index)) {
                    (Some(primary_value), Some(other_value)) => {
                        diff_json(&child_path, primary_value, other_value, role, config, diffs);
                    }
                    (primary_value, other_value) => {
                        push_json_diff(child_path, primary_value, other_value, role, diffs);
                    }
                }
            }
        }
        _ if primary == other => {}
        _ => push_json_diff(path.to_string(), Some(primary), Some(other), role, diffs),
    }
}

fn push_json_diff(
    path: String,
    primary: Option<&Value>,
    other: Option<&Value>,
    role: TargetRole,
    diffs: &mut Vec<DiffEntry>,
) {
    let other_value = other.map(json_preview);
    let (candidate, secondary) = role.values(other_value);
    diffs.push(DiffEntry {
        kind: DiffKind::Body,
        path: path.clone(),
        primary: primary.map(json_preview),
        candidate,
        secondary,
        message: format!("primary body value {path} differs from {}", role.label()),
    });
}

fn json_preview(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<json>".to_string())
}

fn normalize_text(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CompareConfig {
        CompareConfig::new(
            &[
                "$.timestamp".into(),
                "$.requestId".into(),
                "$.traceId".into(),
                "$.id".into(),
            ],
            &[
                "date".into(),
                "server".into(),
                "set-cookie".into(),
                "x-request-id".into(),
                "traceparent".into(),
            ],
            false,
        )
    }

    fn target(status: u16, headers: &[(&str, &str)], body: &str) -> CapturedTarget {
        target_with_stderr(status, headers, body, "")
    }

    fn target_with_stderr(
        status: u16,
        headers: &[(&str, &str)],
        body: &str,
        stderr: &str,
    ) -> CapturedTarget {
        CapturedTarget {
            observation: TargetObservation {
                status: Some(status),
                headers: headers
                    .iter()
                    .map(|(key, value)| (key.to_ascii_lowercase(), value.to_string()))
                    .collect(),
                body: capture_body(body.as_bytes(), 1024),
                stderr: Some(capture_body(stderr.as_bytes(), 1024)),
                latency_ms: 1,
                error: None,
            },
            body_bytes: Bytes::copy_from_slice(body.as_bytes()),
            stderr_bytes: Bytes::copy_from_slice(stderr.as_bytes()),
        }
    }

    fn target_error(message: &str) -> CapturedTarget {
        CapturedTarget {
            observation: TargetObservation {
                status: None,
                headers: BTreeMap::new(),
                body: capture_body(&[], 1024),
                stderr: None,
                latency_ms: 1,
                error: Some(message.to_string()),
            },
            body_bytes: Bytes::new(),
            stderr_bytes: Bytes::new(),
        }
    }

    #[test]
    fn identical_targets_match() {
        let primary = target(
            200,
            &[("content-type", "application/json")],
            r#"{"ok":true}"#,
        );
        let candidate = target(
            200,
            &[("content-type", "application/json")],
            r#"{"ok":true}"#,
        );
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::Match);
        assert!(result.raw_candidate_diffs.is_empty());
    }

    #[test]
    fn identical_non_json_bodies_match() {
        let primary = target(200, &[], "plain text");
        let candidate = target(200, &[], "plain text");
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::Match);
        assert!(result.raw_candidate_diffs.is_empty());
    }

    #[test]
    fn candidate_difference_without_secondary_is_suspicious() {
        let primary = target(200, &[], r#"{"ok":true,"value":1}"#);
        let candidate = target(200, &[], r#"{"ok":true,"value":2}"#);
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::SuspiciousDifference);
        assert_eq!(result.noise_filtered_diffs[0].path, "$.value");
    }

    #[test]
    fn candidate_matching_primary_on_noisy_path_is_reference_noise() {
        let primary = target(200, &[], r#"{"region":"a","value":1}"#);
        let candidate = target(200, &[], r#"{"region":"a","value":1}"#);
        let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
        let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
        assert_eq!(result.classification, Classification::ReferenceNoise);
        assert_eq!(result.reference_noise[0].path, "$.region");
    }

    #[test]
    fn candidate_matching_secondary_on_noisy_path_is_reference_noise() {
        let primary = target(200, &[], r#"{"region":"a","value":1}"#);
        let candidate = target(200, &[], r#"{"region":"b","value":1}"#);
        let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
        let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
        assert_eq!(result.classification, Classification::ReferenceNoise);
        assert!(result.noise_filtered_diffs.is_empty());
    }

    #[test]
    fn candidate_different_from_both_references_is_suspicious_with_noise() {
        let primary = target(200, &[], r#"{"region":"a","value":1}"#);
        let candidate = target(200, &[], r#"{"region":"c","value":1}"#);
        let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
        let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
        assert_eq!(result.classification, Classification::SuspiciousWithNoise);
        assert_eq!(result.noise_filtered_diffs[0].path, "$.region");
    }

    #[test]
    fn status_noise_uses_candidate_must_match_reference_rule() {
        let primary = target(200, &[], r#"{"ok":true}"#);
        let candidate = target(500, &[], r#"{"ok":true}"#);
        let secondary = target(404, &[], r#"{"ok":true}"#);
        let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
        assert_eq!(result.classification, Classification::SuspiciousWithNoise);
        assert_eq!(result.noise_filtered_diffs[0].kind, DiffKind::Status);
    }

    #[test]
    fn header_noise_filters_when_candidate_matches_secondary() {
        let primary = target(200, &[("x-region", "a")], "ok");
        let candidate = target(200, &[("x-region", "b")], "ok");
        let secondary = target(200, &[("x-region", "b")], "ok");
        let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
        assert_eq!(result.classification, Classification::ReferenceNoise);
    }

    #[test]
    fn stderr_differences_are_compared_by_default() {
        let primary = target_with_stderr(0, &[], "ok", "primary");
        let candidate = target_with_stderr(0, &[], "ok", "candidate");
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::SuspiciousDifference);
        assert_eq!(result.noise_filtered_diffs[0].kind, DiffKind::Stderr);
    }

    #[test]
    fn stderr_can_be_ignored() {
        let primary = target_with_stderr(0, &[], "ok", "primary");
        let candidate = target_with_stderr(0, &[], "ok", "candidate");
        let config = CompareConfig::new(&[], &[], true);
        let result = compare_targets(&primary, &candidate, None, &config);
        assert_eq!(result.classification, Classification::Match);
    }

    #[test]
    fn target_errors_are_top_level_errors() {
        let primary = target(200, &[], "ok");
        let candidate = target_error("candidate failed");
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::TargetError);
    }

    #[test]
    fn ignored_json_fields_do_not_diff() {
        let primary = target(200, &[], r#"{"id":"a","value":1}"#);
        let candidate = target(200, &[], r#"{"id":"b","value":1}"#);
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::Match);
    }

    #[test]
    fn ignored_headers_do_not_diff() {
        let primary = target(200, &[("date", "one"), ("x-mode", "a")], "ok");
        let candidate = target(200, &[("date", "two"), ("x-mode", "a")], "ok");
        let result = compare_targets(&primary, &candidate, None, &config());
        assert_eq!(result.classification, Classification::Match);
    }
}
