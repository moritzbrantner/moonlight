use crate::BodyCapture;
use http::HeaderMap;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub fn capture_body(body: &[u8], max_bytes: usize) -> BodyCapture {
    capture_body_with_redactions(body, max_bytes, &[])
}

pub fn capture_body_with_redactions(
    body: &[u8],
    max_bytes: usize,
    redact_json_paths: &[String],
) -> BodyCapture {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let preview_source = redact_json_body(body, redact_json_paths).unwrap_or_else(|| body.to_vec());
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

fn redact_json_body(body: &[u8], redact_json_paths: &[String]) -> Option<Vec<u8>> {
    if redact_json_paths.is_empty() {
        return None;
    }

    let mut value = serde_json::from_slice::<Value>(body).ok()?;
    let mut changed = false;
    for path in redact_json_paths {
        if redact_json_value(&mut value, path) {
            changed = true;
        }
    }

    changed.then(|| serde_json::to_vec(&value).unwrap_or_else(|_| body.to_vec()))
}

fn redact_json_value(value: &mut Value, path: &str) -> bool {
    let Some(segments) = parse_json_path(path) else {
        return false;
    };

    let mut current = value;
    for segment in segments {
        match segment {
            PathSegment::Key(key) => {
                let Some(next) = current.get_mut(&key) else {
                    return false;
                };
                current = next;
            }
            PathSegment::Index(index) => {
                let Some(next) = current.get_mut(index) else {
                    return false;
                };
                current = next;
            }
        }
    }

    *current = Value::String("[redacted]".to_string());
    true
}

#[derive(Debug, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

fn parse_json_path(path: &str) -> Option<Vec<PathSegment>> {
    let mut chars = path.strip_prefix('$')?.chars().peekable();
    let mut segments = Vec::new();

    while let Some(next) = chars.peek().copied() {
        match next {
            '.' => {
                chars.next();
                let mut key = String::new();
                while let Some(ch) = chars.peek().copied() {
                    if ch == '.' || ch == '[' {
                        break;
                    }
                    key.push(ch);
                    chars.next();
                }
                if key.is_empty() {
                    return None;
                }
                segments.push(PathSegment::Key(key));
            }
            '[' => {
                chars.next();
                let mut index = String::new();
                while let Some(ch) = chars.peek().copied() {
                    if ch == ']' {
                        break;
                    }
                    index.push(ch);
                    chars.next();
                }
                if chars.next() != Some(']') {
                    return None;
                }
                segments.push(PathSegment::Index(index.parse().ok()?));
            }
            _ => return None,
        }
    }

    Some(segments)
}
