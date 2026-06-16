use crate::{DiffEntry, DiffKind, TargetObservation};
use bytes::Bytes;
use serde_json::Value;
use std::collections::BTreeSet;

use super::CompareConfig;

#[derive(Debug, Clone)]
pub struct CapturedTarget {
    pub observation: TargetObservation,
    pub body_bytes: Bytes,
    pub stderr_bytes: Bytes,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TargetRole {
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

pub(super) fn diff_pair(
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
        .filter(|name| !config.ignore_headers.contains(*name))
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
    if config.ignore_stderr || primary.stderr_bytes == other.stderr_bytes {
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
    if config.ignore_json_paths.contains(path)
        || matches_any_json_path_pattern(path, &config.ignore_json_path_patterns)
    {
        return;
    }
    if config.redact_json_paths.contains(path)
        || matches_any_json_path_pattern(path, &config.redact_json_path_patterns)
    {
        if primary != other {
            push_redacted_json_diff(path.to_string(), role, diffs);
        }
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

fn push_redacted_json_diff(path: String, role: TargetRole, diffs: &mut Vec<DiffEntry>) {
    let redacted = Some("\"[redacted]\"".to_string());
    let (candidate, secondary) = role.values(redacted.clone());
    diffs.push(DiffEntry {
        kind: DiffKind::Body,
        path: path.clone(),
        primary: redacted,
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

fn matches_any_json_path_pattern(path: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| json_path_pattern_matches(path, pattern))
}

fn json_path_pattern_matches(path: &str, pattern: &str) -> bool {
    let path_tokens = tokenize_json_path(path);
    let pattern_tokens = tokenize_json_path(pattern);
    if path_tokens.len() != pattern_tokens.len() {
        return false;
    }

    path_tokens
        .iter()
        .zip(pattern_tokens.iter())
        .all(|(path, pattern)| match pattern.as_str() {
            "*" => !path.starts_with('['),
            "[*]" => path.starts_with('[') && path.ends_with(']'),
            _ => path == pattern,
        })
}

fn tokenize_json_path(path: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '$' if tokens.is_empty() && current.is_empty() => tokens.push("$".to_string()),
            '.' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '[' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                current.push(ch);
                for ch in chars.by_ref() {
                    current.push(ch);
                    if ch == ']' {
                        break;
                    }
                }
                tokens.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
