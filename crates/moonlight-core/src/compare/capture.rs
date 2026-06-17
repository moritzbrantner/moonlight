use crate::BodyCapture;
use http::HeaderMap;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

use super::json_path;

pub fn capture_body(body: &[u8], max_bytes: usize) -> BodyCapture {
    capture_body_with_redactions(body, max_bytes, &[])
}

pub fn capture_body_with_redactions(
    body: &[u8],
    max_bytes: usize,
    redact_json_paths: &[String],
) -> BodyCapture {
    capture_body_with_redaction_patterns(body, max_bytes, redact_json_paths, &[])
}

pub fn capture_body_with_redaction_patterns(
    body: &[u8],
    max_bytes: usize,
    redact_json_paths: &[String],
    redact_json_path_patterns: &[String],
) -> BodyCapture {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let preview_source = redact_json_body(body, redact_json_paths, redact_json_path_patterns)
        .unwrap_or_else(|| body.to_vec());
    let preview_bytes = &preview_source[..preview_source.len().min(max_bytes)];
    BodyCapture {
        size_bytes: body.len(),
        sha256: hex::encode(hasher.finalize()),
        preview: String::from_utf8_lossy(preview_bytes).to_string(),
        truncated: preview_source.len() > max_bytes,
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

fn redact_json_body(
    body: &[u8],
    redact_json_paths: &[String],
    redact_json_path_patterns: &[String],
) -> Option<Vec<u8>> {
    if redact_json_paths.is_empty() && redact_json_path_patterns.is_empty() {
        return None;
    }

    let mut value = serde_json::from_slice::<Value>(body).ok()?;
    let mut changed = false;
    for path in redact_json_paths {
        if json_path::redact_value_at_path(&mut value, path) {
            changed = true;
        }
    }
    for pattern in redact_json_path_patterns {
        if json_path::redact_value_at_matching_paths(&mut value, pattern) {
            changed = true;
        }
    }

    changed.then(|| serde_json::to_vec(&value).unwrap_or_else(|_| body.to_vec()))
}
