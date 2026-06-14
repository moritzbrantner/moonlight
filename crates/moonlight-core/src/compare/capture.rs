use crate::BodyCapture;
use http::HeaderMap;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

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
