use crate::{target::CapturedTarget, DiffEntry, DiffKind};
use serde_json::Value;
use std::collections::BTreeSet;

use super::{json_path, CompareConfig};

#[derive(Debug, Clone)]
pub(super) struct PairDiff {
    pub(super) entry: DiffEntry,
    pub(super) semantic_primary: Option<String>,
    pub(super) semantic_other: Option<String>,
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
) -> Vec<PairDiff> {
    let mut diffs = Vec::new();
    diff_target_errors(primary, other, role, &mut diffs);
    diff_status(primary, other, role, &mut diffs);
    diff_headers(primary, other, role, config, &mut diffs);
    diff_bodies(primary, other, role, config, &mut diffs);
    diff_stderr(primary, other, role, config, &mut diffs);
    diffs
}

fn push_pair_diff(
    entry: DiffEntry,
    semantic_primary: Option<String>,
    semantic_other: Option<String>,
    diffs: &mut Vec<PairDiff>,
) {
    diffs.push(PairDiff {
        entry,
        semantic_primary,
        semantic_other,
    });
}

fn diff_target_errors(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    diffs: &mut Vec<PairDiff>,
) {
    if primary.observation.error != other.observation.error {
        let semantic_primary = primary.observation.error.clone();
        let semantic_other = other.observation.error.clone();
        let (candidate, secondary) = role.values(semantic_other.clone());
        push_pair_diff(
            DiffEntry {
                kind: DiffKind::TargetError,
                path: "$target_error".to_string(),
                primary: semantic_primary.clone(),
                candidate,
                secondary,
                message: format!("primary target error differs from {}", role.label()),
            },
            semantic_primary,
            semantic_other,
            diffs,
        );
    }
}

fn diff_status(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    diffs: &mut Vec<PairDiff>,
) {
    if primary.observation.status != other.observation.status {
        let semantic_primary = primary.observation.status.map(|value| value.to_string());
        let semantic_other = other.observation.status.map(|value| value.to_string());
        let (candidate, secondary) = role.values(semantic_other.clone());
        push_pair_diff(
            DiffEntry {
                kind: DiffKind::Status,
                path: "$status".to_string(),
                primary: semantic_primary.clone(),
                candidate,
                secondary,
                message: format!("primary status differs from {}", role.label()),
            },
            semantic_primary,
            semantic_other,
            diffs,
        );
    }
}

fn diff_headers(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<PairDiff>,
) {
    let keys: BTreeSet<String> = header_names(primary)
        .chain(header_names(other))
        .filter(|name| !config.ignore_headers.contains(*name))
        .collect();

    for key in keys {
        let semantic_primary = semantic_header_value(primary, &key);
        let semantic_other = semantic_header_value(other, &key);
        if semantic_primary == semantic_other {
            continue;
        }

        let primary_value = primary.observation.headers.get(&key).cloned();
        let other_value = other.observation.headers.get(&key).cloned();
        let (candidate, secondary) = role.values(other_value);
        push_pair_diff(
            DiffEntry {
                kind: DiffKind::Header,
                path: format!("$.headers.{key}"),
                primary: primary_value,
                candidate,
                secondary,
                message: format!("primary header {key} differs from {}", role.label()),
            },
            semantic_primary,
            semantic_other,
            diffs,
        );
    }
}

fn header_names(target: &CapturedTarget) -> impl Iterator<Item = String> + '_ {
    let raw = target
        .transport_headers
        .keys()
        .map(|name| name.as_str().to_ascii_lowercase());
    let evidence = target.observation.headers.keys().cloned();
    raw.chain(evidence)
}

fn semantic_header_value(target: &CapturedTarget, key: &str) -> Option<String> {
    let raw_values = target
        .transport_headers
        .get_all(key)
        .iter()
        .map(|value| value.to_str().unwrap_or("[non-utf8]").to_string())
        .collect::<Vec<_>>();
    if !raw_values.is_empty() {
        return Some(
            serde_json::to_string(&raw_values).unwrap_or_else(|_| "[header-values]".to_string()),
        );
    }
    target.observation.headers.get(key).cloned()
}

fn diff_bodies(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<PairDiff>,
) {
    if primary.body_bytes == other.body_bytes {
        return;
    }

    let primary_json = serde_json::from_slice::<Value>(&primary.body_bytes);
    let other_json = serde_json::from_slice::<Value>(&other.body_bytes);

    match (primary_json, other_json) {
        (Ok(primary_json), Ok(other_json)) => {
            diff_json(
                "$",
                Some(&primary_json),
                Some(&other_json),
                role,
                config,
                false,
                diffs,
            );
        }
        _ => {
            let primary_text = normalize_text(&primary.body_bytes);
            let other_text = normalize_text(&other.body_bytes);
            if primary_text != other_text {
                let semantic_primary = Some(primary_text);
                let semantic_other = Some(other_text);
                let (candidate, secondary) = role.values(semantic_other.clone());
                push_pair_diff(
                    DiffEntry {
                        kind: DiffKind::Body,
                        path: "$body".to_string(),
                        primary: semantic_primary.clone(),
                        candidate,
                        secondary,
                        message: format!("primary body differs from {}", role.label()),
                    },
                    semantic_primary,
                    semantic_other,
                    diffs,
                );
            }
        }
    }
}

fn diff_stderr(
    primary: &CapturedTarget,
    other: &CapturedTarget,
    role: TargetRole,
    config: &CompareConfig,
    diffs: &mut Vec<PairDiff>,
) {
    if config.ignore_stderr || primary.stderr_bytes == other.stderr_bytes {
        return;
    }

    let primary_text = normalize_text(&primary.stderr_bytes);
    let other_text = normalize_text(&other.stderr_bytes);
    if primary_text != other_text {
        let semantic_primary = Some(primary_text);
        let semantic_other = Some(other_text);
        let (candidate, secondary) = role.values(semantic_other.clone());
        push_pair_diff(
            DiffEntry {
                kind: DiffKind::Stderr,
                path: "$stderr".to_string(),
                primary: semantic_primary.clone(),
                candidate,
                secondary,
                message: format!("primary stderr differs from {}", role.label()),
            },
            semantic_primary,
            semantic_other,
            diffs,
        );
    }
}

fn diff_json(
    path: &str,
    primary: Option<&Value>,
    other: Option<&Value>,
    role: TargetRole,
    config: &CompareConfig,
    inherited_redaction: bool,
    diffs: &mut Vec<PairDiff>,
) {
    if path_is_ignored(path, config) {
        return;
    }

    let redacted = inherited_redaction || path_is_redacted(path, config);
    match (primary, other) {
        (Some(Value::Object(primary_map)), Some(Value::Object(other_map))) => {
            let keys: BTreeSet<String> = primary_map
                .keys()
                .chain(other_map.keys())
                .cloned()
                .collect();
            if keys.is_empty() && primary_map != other_map {
                push_json_diff(path, primary, other, role, config, redacted, diffs);
                return;
            }
            for key in keys {
                let child_path = json_path::child_key_path(path, &key);
                diff_json(
                    &child_path,
                    primary_map.get(&key),
                    other_map.get(&key),
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (Some(Value::Array(primary_items)), Some(Value::Array(other_items))) => {
            let max_len = primary_items.len().max(other_items.len());
            for index in 0..max_len {
                let child_path = json_path::child_index_path(path, index);
                diff_json(
                    &child_path,
                    primary_items.get(index),
                    other_items.get(index),
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (None, Some(Value::Object(other_map))) => {
            if other_map.is_empty() {
                push_json_diff(path, None, other, role, config, redacted, diffs);
                return;
            }
            for (key, other_value) in other_map {
                let child_path = json_path::child_key_path(path, key);
                diff_json(
                    &child_path,
                    None,
                    Some(other_value),
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (Some(Value::Object(primary_map)), None) => {
            if primary_map.is_empty() {
                push_json_diff(path, primary, None, role, config, redacted, diffs);
                return;
            }
            for (key, primary_value) in primary_map {
                let child_path = json_path::child_key_path(path, key);
                diff_json(
                    &child_path,
                    Some(primary_value),
                    None,
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (None, Some(Value::Array(other_items))) => {
            if other_items.is_empty() {
                push_json_diff(path, None, other, role, config, redacted, diffs);
                return;
            }
            for (index, other_value) in other_items.iter().enumerate() {
                let child_path = json_path::child_index_path(path, index);
                diff_json(
                    &child_path,
                    None,
                    Some(other_value),
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (Some(Value::Array(primary_items)), None) => {
            if primary_items.is_empty() {
                push_json_diff(path, primary, None, role, config, redacted, diffs);
                return;
            }
            for (index, primary_value) in primary_items.iter().enumerate() {
                let child_path = json_path::child_index_path(path, index);
                diff_json(
                    &child_path,
                    Some(primary_value),
                    None,
                    role,
                    config,
                    redacted,
                    diffs,
                );
            }
        }
        (Some(primary_value), Some(other_value)) if primary_value == other_value => {}
        (None, None) => {}
        _ => push_json_diff(path, primary, other, role, config, redacted, diffs),
    }
}

fn push_json_diff(
    path: &str,
    primary: Option<&Value>,
    other: Option<&Value>,
    role: TargetRole,
    config: &CompareConfig,
    inherited_redaction: bool,
    diffs: &mut Vec<PairDiff>,
) {
    let semantic_primary = primary.map(json_preview);
    let semantic_other = other.map(json_preview);
    let primary_value = primary.map(|value| evidence_json_preview(path, value, config, inherited_redaction));
    let other_value = other.map(|value| evidence_json_preview(path, value, config, inherited_redaction));
    let (candidate, secondary) = role.values(other_value);

    push_pair_diff(
        DiffEntry {
            kind: DiffKind::Body,
            path: path.to_string(),
            primary: primary_value,
            candidate,
            secondary,
            message: format!("primary body value {path} differs from {}", role.label()),
        },
        semantic_primary,
        semantic_other,
        diffs,
    );
}

fn evidence_json_preview(
    path: &str,
    value: &Value,
    config: &CompareConfig,
    inherited_redaction: bool,
) -> String {
    let mut sanitized = value.clone();
    sanitize_json_value(path, &mut sanitized, config, inherited_redaction);
    json_preview(&sanitized)
}

fn sanitize_json_value(
    path: &str,
    value: &mut Value,
    config: &CompareConfig,
    inherited_redaction: bool,
) {
    if inherited_redaction || path_is_redacted(path, config) {
        *value = Value::String("[redacted]".to_string());
        return;
    }

    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = json_path::child_key_path(path, key);
                sanitize_json_value(&child_path, child, config, false);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter_mut().enumerate() {
                let child_path = json_path::child_index_path(path, index);
                sanitize_json_value(&child_path, child, config, false);
            }
        }
        _ => {}
    }
}

fn path_is_ignored(path: &str, config: &CompareConfig) -> bool {
    config.ignore_json_paths.contains(path)
        || json_path::matches_any_path(path, &config.ignore_json_path_patterns)
}

fn path_is_redacted(path: &str, config: &CompareConfig) -> bool {
    config.redact_json_paths.contains(path)
        || json_path::matches_any_path(path, &config.redact_json_path_patterns)
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
