use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub primary_url: String,
    pub candidate_url: String,
    pub secondary_url: String,
    pub enable_candidate: bool,
    pub enable_secondary: bool,
    pub return_backend: ReturnBackend,
    pub response_mode: ResponseMode,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    WaitAll,
    PrimaryThenShadow,
}

impl FromStr for ResponseMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wait_all" => Ok(Self::WaitAll),
            "primary_then_shadow" => Ok(Self::PrimaryThenShadow),
            other => anyhow::bail!(
                "invalid SHADOWDIFF_RESPONSE_MODE {other:?}; use wait_all or primary_then_shadow"
            ),
        }
    }
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            bind_addr: env_or(&get, "SHADOWDIFF_BIND_ADDR", "127.0.0.1:8080").parse()?,
            primary_url: normalize_base_url(env_or(
                &get,
                "SHADOWDIFF_PRIMARY_URL",
                "http://127.0.0.1:3001",
            )),
            candidate_url: normalize_base_url(env_or(
                &get,
                "SHADOWDIFF_CANDIDATE_URL",
                "http://127.0.0.1:3002",
            )),
            secondary_url: normalize_base_url(env_or(
                &get,
                "SHADOWDIFF_SECONDARY_URL",
                "http://127.0.0.1:3003",
            )),
            enable_candidate: env_bool(&get, "SHADOWDIFF_ENABLE_CANDIDATE", true),
            enable_secondary: env_bool(&get, "SHADOWDIFF_ENABLE_SECONDARY", true),
            return_backend: ReturnBackend::Primary,
            response_mode: env_or(&get, "SHADOWDIFF_RESPONSE_MODE", "wait_all").parse()?,
            max_body_capture_bytes: env_or(&get, "SHADOWDIFF_MAX_BODY_CAPTURE_BYTES", "8192")
                .parse()?,
            redact_headers: env_list(
                &get,
                "SHADOWDIFF_REDACT_HEADERS",
                &["authorization", "cookie", "set-cookie", "x-api-key"],
            ),
            ignored_json_paths: env_list(
                &get,
                "SHADOWDIFF_IGNORED_JSON_PATHS",
                &["$.timestamp", "$.requestId", "$.traceId", "$.id"],
            ),
            ignored_headers: env_list(
                &get,
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
                &get,
                "SHADOWDIFF_STORAGE_PATH",
                "data/shadowdiff/requests.jsonl",
            )),
        })
    }
}

fn env_or(get: &impl Fn(&str) -> Option<String>, key: &str, default: &str) -> String {
    get(key).unwrap_or_else(|| default.to_string())
}

fn env_bool(get: &impl Fn(&str) -> Option<String>, key: &str, default: bool) -> bool {
    get(key)
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn env_list(get: &impl Fn(&str) -> Option<String>, key: &str, defaults: &[&str]) -> Vec<String> {
    get(key)
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_else(|| defaults.iter().map(|item| item.to_string()).collect())
}

fn normalize_base_url(value: String) -> String {
    value.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_from(values: &[(&str, &str)]) -> anyhow::Result<AppConfig> {
        let values: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        AppConfig::from_lookup(|key| values.get(key).cloned())
    }

    #[test]
    fn response_mode_defaults_to_wait_all() {
        let config = config_from(&[]).unwrap();
        assert_eq!(config.response_mode, ResponseMode::WaitAll);
    }

    #[test]
    fn response_mode_parses_primary_then_shadow() {
        let config = config_from(&[("SHADOWDIFF_RESPONSE_MODE", "primary_then_shadow")]).unwrap();
        assert_eq!(config.response_mode, ResponseMode::PrimaryThenShadow);
    }

    #[test]
    fn invalid_response_mode_returns_error() {
        let error = config_from(&[("SHADOWDIFF_RESPONSE_MODE", "fast")]).unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid SHADOWDIFF_RESPONSE_MODE"));
    }
}
