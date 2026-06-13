use bytes::Bytes;
use http::HeaderMap;
use serde_json::Value;
use sha2::{Digest, Sha256};
use shadowdiff_types::{
    BackendCapture, BodyCapture, Classification, ComparisonSummary, DiffEntry, DiffKind,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone)]
pub struct CapturedBackend {
    pub capture: BackendCapture,
    pub body_bytes: Bytes,
}

#[derive(Debug, Clone)]
pub struct CompareConfig {
    pub ignored_json_paths: HashSet<String>,
    pub ignored_headers: HashSet<String>,
}

impl CompareConfig {
    pub fn new(ignored_json_paths: &[String], ignored_headers: &[String]) -> Self {
        Self {
            ignored_json_paths: ignored_json_paths.iter().cloned().collect(),
            ignored_headers: ignored_headers
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
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

pub fn compare_backends(
    primary: &CapturedBackend,
    candidate: Option<&CapturedBackend>,
    secondary: Option<&CapturedBackend>,
    config: &CompareConfig,
) -> ComparisonSummary {
    let raw_candidate_diffs = candidate
        .map(|candidate| diff_pair(primary, candidate, None, "candidate", config))
        .unwrap_or_default();
    let reference_noise = secondary
        .map(|secondary| diff_pair(primary, secondary, Some("secondary"), "secondary", config))
        .unwrap_or_default();
    let noise_paths: HashSet<(DiffKind, String)> = reference_noise
        .iter()
        .map(|entry| (entry.kind.clone(), entry.path.clone()))
        .collect();
    let noise_filtered_diffs: Vec<DiffEntry> = raw_candidate_diffs
        .iter()
        .filter(|entry| !noise_paths.contains(&(entry.kind.clone(), entry.path.clone())))
        .cloned()
        .collect();

    let backend_error = primary.capture.error.is_some()
        || candidate
            .and_then(|backend| backend.capture.error.as_ref())
            .is_some()
        || secondary
            .and_then(|backend| backend.capture.error.as_ref())
            .is_some();

    let classification = if backend_error {
        Classification::BackendError
    } else if raw_candidate_diffs.is_empty() && reference_noise.is_empty() {
        Classification::Match
    } else if noise_filtered_diffs.is_empty() {
        Classification::Noise
    } else if !reference_noise.is_empty() {
        Classification::CandidateDiffWithNoise
    } else {
        Classification::CandidateDiff
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

fn diff_pair(
    primary: &CapturedBackend,
    other: &CapturedBackend,
    other_as_secondary: Option<&str>,
    label: &str,
    config: &CompareConfig,
) -> Vec<DiffEntry> {
    let mut diffs = Vec::new();
    if primary.capture.error != other.capture.error {
        diffs.push(DiffEntry {
            kind: DiffKind::BackendError,
            path: "$backend_error".to_string(),
            primary: primary.capture.error.clone(),
            candidate: value_for_label(label, other.capture.error.clone(), other_as_secondary).0,
            secondary: value_for_label(label, other.capture.error.clone(), other_as_secondary).1,
            message: format!("primary backend error differs from {label}"),
        });
    }
    if primary.capture.status != other.capture.status {
        diffs.push(DiffEntry {
            kind: DiffKind::Status,
            path: "$status".to_string(),
            primary: primary.capture.status.map(|value| value.to_string()),
            candidate: value_for_label(
                label,
                other.capture.status.map(|value| value.to_string()),
                other_as_secondary,
            )
            .0,
            secondary: value_for_label(
                label,
                other.capture.status.map(|value| value.to_string()),
                other_as_secondary,
            )
            .1,
            message: format!("primary status differs from {label}"),
        });
    }
    diff_headers(
        primary,
        other,
        other_as_secondary,
        label,
        config,
        &mut diffs,
    );
    diff_bodies(
        primary,
        other,
        other_as_secondary,
        label,
        config,
        &mut diffs,
    );
    diffs
}

fn diff_headers(
    primary: &CapturedBackend,
    other: &CapturedBackend,
    other_as_secondary: Option<&str>,
    label: &str,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    let keys: BTreeSet<String> = primary
        .capture
        .headers
        .keys()
        .chain(other.capture.headers.keys())
        .filter(|name| !config.ignored_headers.contains(*name))
        .cloned()
        .collect();

    for key in keys {
        let primary_value = primary.capture.headers.get(&key).cloned();
        let other_value = other.capture.headers.get(&key).cloned();
        if primary_value != other_value {
            let (candidate, secondary) = value_for_label(label, other_value, other_as_secondary);
            diffs.push(DiffEntry {
                kind: DiffKind::Header,
                path: format!("$.headers.{key}"),
                primary: primary_value,
                candidate,
                secondary,
                message: format!("primary header {key} differs from {label}"),
            });
        }
    }
}

fn diff_bodies(
    primary: &CapturedBackend,
    other: &CapturedBackend,
    other_as_secondary: Option<&str>,
    label: &str,
    config: &CompareConfig,
    diffs: &mut Vec<DiffEntry>,
) {
    let primary_json = serde_json::from_slice::<Value>(&primary.body_bytes);
    let other_json = serde_json::from_slice::<Value>(&other.body_bytes);

    match (primary_json, other_json) {
        (Ok(primary_json), Ok(other_json)) => {
            diff_json(
                "$",
                &primary_json,
                &other_json,
                other_as_secondary,
                label,
                config,
                diffs,
            );
        }
        _ => {
            let primary_text = normalize_text(&primary.body_bytes);
            let other_text = normalize_text(&other.body_bytes);
            if primary_text != other_text {
                let (candidate, secondary) =
                    value_for_label(label, Some(other_text), other_as_secondary);
                diffs.push(DiffEntry {
                    kind: DiffKind::Body,
                    path: "$body".to_string(),
                    primary: Some(primary_text),
                    candidate,
                    secondary,
                    message: format!("primary body differs from {label}"),
                });
            }
        }
    }
}

fn diff_json(
    path: &str,
    primary: &Value,
    other: &Value,
    other_as_secondary: Option<&str>,
    label: &str,
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
                        diff_json(
                            &child_path,
                            primary_value,
                            other_value,
                            other_as_secondary,
                            label,
                            config,
                            diffs,
                        );
                    }
                    (primary_value, other_value) => push_json_diff(
                        child_path,
                        primary_value,
                        other_value,
                        other_as_secondary,
                        label,
                        diffs,
                    ),
                }
            }
        }
        (Value::Array(primary_items), Value::Array(other_items)) => {
            // TODO: support order-insensitive comparison for arrays that model sets.
            let max_len = primary_items.len().max(other_items.len());
            for index in 0..max_len {
                let child_path = format!("{path}[{index}]");
                match (primary_items.get(index), other_items.get(index)) {
                    (Some(primary_value), Some(other_value)) => {
                        diff_json(
                            &child_path,
                            primary_value,
                            other_value,
                            other_as_secondary,
                            label,
                            config,
                            diffs,
                        );
                    }
                    (primary_value, other_value) => push_json_diff(
                        child_path,
                        primary_value,
                        other_value,
                        other_as_secondary,
                        label,
                        diffs,
                    ),
                }
            }
        }
        _ if primary == other => {}
        _ => push_json_diff(
            path.to_string(),
            Some(primary),
            Some(other),
            other_as_secondary,
            label,
            diffs,
        ),
    }
}

fn push_json_diff(
    path: String,
    primary: Option<&Value>,
    other: Option<&Value>,
    other_as_secondary: Option<&str>,
    label: &str,
    diffs: &mut Vec<DiffEntry>,
) {
    let other_value = other.map(json_preview);
    let (candidate, secondary) = value_for_label(label, other_value, other_as_secondary);
    diffs.push(DiffEntry {
        kind: DiffKind::Body,
        path: path.clone(),
        primary: primary.map(json_preview),
        candidate,
        secondary,
        message: format!("primary body value {path} differs from {label}"),
    });
}

fn value_for_label(
    label: &str,
    value: Option<String>,
    other_as_secondary: Option<&str>,
) -> (Option<String>, Option<String>) {
    if other_as_secondary == Some(label) {
        (None, value)
    } else {
        (value, None)
    }
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
        )
    }

    fn backend(status: u16, headers: &[(&str, &str)], body: &str) -> CapturedBackend {
        CapturedBackend {
            capture: BackendCapture {
                status: Some(status),
                headers: headers
                    .iter()
                    .map(|(key, value)| (key.to_ascii_lowercase(), value.to_string()))
                    .collect(),
                body: capture_body(body.as_bytes(), 1024),
                latency_ms: 1,
                error: None,
            },
            body_bytes: Bytes::copy_from_slice(body.as_bytes()),
        }
    }

    #[test]
    fn identical_responses_match() {
        let primary = backend(
            200,
            &[("content-type", "application/json")],
            r#"{"ok":true}"#,
        );
        let candidate = backend(
            200,
            &[("content-type", "application/json")],
            r#"{"ok":true}"#,
        );
        let result = compare_backends(&primary, Some(&candidate), None, &config());
        assert_eq!(result.classification, Classification::Match);
        assert!(result.raw_candidate_diffs.is_empty());
    }

    #[test]
    fn candidate_only_body_difference_is_suspicious() {
        let primary = backend(200, &[], r#"{"ok":true,"value":1}"#);
        let candidate = backend(200, &[], r#"{"ok":true,"value":2}"#);
        let result = compare_backends(&primary, Some(&candidate), None, &config());
        assert_eq!(result.classification, Classification::CandidateDiff);
        assert_eq!(result.noise_filtered_diffs[0].path, "$.value");
    }

    #[test]
    fn primary_secondary_difference_is_noise() {
        let primary = backend(200, &[], r#"{"region":"a","value":1}"#);
        let candidate = backend(200, &[], r#"{"region":"a","value":1}"#);
        let secondary = backend(200, &[], r#"{"region":"b","value":1}"#);
        let result = compare_backends(&primary, Some(&candidate), Some(&secondary), &config());
        assert_eq!(result.classification, Classification::Noise);
        assert_eq!(result.reference_noise[0].path, "$.region");
    }

    #[test]
    fn candidate_difference_in_same_field_as_reference_noise_is_filtered() {
        let primary = backend(200, &[], r#"{"region":"a","value":1}"#);
        let candidate = backend(200, &[], r#"{"region":"c","value":1}"#);
        let secondary = backend(200, &[], r#"{"region":"b","value":1}"#);
        let result = compare_backends(&primary, Some(&candidate), Some(&secondary), &config());
        assert_eq!(result.classification, Classification::Noise);
        assert!(result.noise_filtered_diffs.is_empty());
    }

    #[test]
    fn status_code_difference_is_reported() {
        let primary = backend(200, &[], r#"{"ok":true}"#);
        let candidate = backend(500, &[], r#"{"ok":true}"#);
        let result = compare_backends(&primary, Some(&candidate), None, &config());
        assert_eq!(result.classification, Classification::CandidateDiff);
        assert_eq!(result.noise_filtered_diffs[0].kind, DiffKind::Status);
    }

    #[test]
    fn ignored_json_fields_do_not_diff() {
        let primary = backend(200, &[], r#"{"id":"a","value":1}"#);
        let candidate = backend(200, &[], r#"{"id":"b","value":1}"#);
        let result = compare_backends(&primary, Some(&candidate), None, &config());
        assert_eq!(result.classification, Classification::Match);
    }

    #[test]
    fn ignored_headers_do_not_diff() {
        let primary = backend(200, &[("date", "one"), ("x-mode", "a")], "ok");
        let candidate = backend(200, &[("date", "two"), ("x-mode", "a")], "ok");
        let result = compare_backends(&primary, Some(&candidate), None, &config());
        assert_eq!(result.classification, Classification::Match);
    }
}
