use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub primary_url: String,
    pub candidate_url: String,
    pub secondary_url: String,
    pub enable_candidate: bool,
    pub enable_secondary: bool,
    pub return_backend: ReturnBackend,
    pub max_body_capture_bytes: usize,
    pub redact_headers: Vec<String>,
    pub ignored_json_paths: Vec<String>,
    pub ignored_headers: Vec<String>,
    pub storage_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnBackend {
    Primary,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind_addr: env_or("SHADOWDIFF_BIND_ADDR", "127.0.0.1:8080").parse()?,
            primary_url: normalize_base_url(env_or(
                "SHADOWDIFF_PRIMARY_URL",
                "http://127.0.0.1:3001",
            )),
            candidate_url: normalize_base_url(env_or(
                "SHADOWDIFF_CANDIDATE_URL",
                "http://127.0.0.1:3002",
            )),
            secondary_url: normalize_base_url(env_or(
                "SHADOWDIFF_SECONDARY_URL",
                "http://127.0.0.1:3003",
            )),
            enable_candidate: env_bool("SHADOWDIFF_ENABLE_CANDIDATE", true),
            enable_secondary: env_bool("SHADOWDIFF_ENABLE_SECONDARY", true),
            return_backend: ReturnBackend::Primary,
            max_body_capture_bytes: env_or("SHADOWDIFF_MAX_BODY_CAPTURE_BYTES", "8192").parse()?,
            redact_headers: env_list(
                "SHADOWDIFF_REDACT_HEADERS",
                &["authorization", "cookie", "set-cookie", "x-api-key"],
            ),
            ignored_json_paths: env_list(
                "SHADOWDIFF_IGNORED_JSON_PATHS",
                &["$.timestamp", "$.requestId", "$.traceId", "$.id"],
            ),
            ignored_headers: env_list(
                "SHADOWDIFF_IGNORED_HEADERS",
                &[
                    "date",
                    "server",
                    "set-cookie",
                    "x-request-id",
                    "traceparent",
                ],
            ),
            storage_path: PathBuf::from(env_or(
                "SHADOWDIFF_STORAGE_PATH",
                "data/shadowdiff/requests.jsonl",
            )),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn env_list(key: &str, defaults: &[&str]) -> Vec<String> {
    env::var(key)
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_else(|_| defaults.iter().map(|item| item.to_string()).collect())
}

fn normalize_base_url(value: String) -> String {
    value.trim_end_matches('/').to_string()
}
