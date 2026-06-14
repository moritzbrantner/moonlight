use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

pub const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub primary_url: String,
    pub candidate_url: String,
    pub secondary_url: String,
    pub enable_secondary: bool,
    pub return_target: ReturnTarget,
    pub return_fallback: ReturnFallback,
    pub response_timing: ResponseTiming,
    pub max_body_capture_bytes: usize,
    pub max_request_body_bytes: usize,
    pub redact_headers: Vec<String>,
    pub redact_json_paths: Vec<String>,
    pub redact_query_params: Vec<String>,
    pub ignored_json_paths: Vec<String>,
    pub ignored_headers: Vec<String>,
    pub ignore_stderr: bool,
    pub storage_path: PathBuf,
    pub cors_origins: Vec<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub admin_token: Option<String>,
    pub retention_max_runs: Option<usize>,
    pub retention_max_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReturnTarget {
    Primary,
    Candidate,
}

impl FromStr for ReturnTarget {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "primary" => Ok(Self::Primary),
            "candidate" => Ok(Self::Candidate),
            other => {
                anyhow::bail!("invalid MOONLIGHT_RETURN_TARGET {other:?}; use primary or candidate")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReturnFallback {
    None,
    Primary,
}

impl FromStr for ReturnFallback {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "primary" => Ok(Self::Primary),
            other => {
                anyhow::bail!("invalid MOONLIGHT_RETURN_FALLBACK {other:?}; use none or primary")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseTiming {
    WaitAll,
    ReturnSelected,
}

impl FromStr for ResponseTiming {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wait_all" => Ok(Self::WaitAll),
            "return_selected" => Ok(Self::ReturnSelected),
            other => anyhow::bail!(
                "invalid MOONLIGHT_RESPONSE_TIMING {other:?}; use wait_all or return_selected"
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
            bind_addr: env_or(&get, "MOONLIGHT_BIND_ADDR", "127.0.0.1:8080").parse()?,
            primary_url: normalize_base_url(env_or(
                &get,
                "MOONLIGHT_PRIMARY_URL",
                "http://127.0.0.1:3001",
            )),
            candidate_url: normalize_base_url(env_or(
                &get,
                "MOONLIGHT_CANDIDATE_URL",
                "http://127.0.0.1:3002",
            )),
            secondary_url: normalize_base_url(env_or(
                &get,
                "MOONLIGHT_SECONDARY_URL",
                "http://127.0.0.1:3003",
            )),
            enable_secondary: env_bool(&get, "MOONLIGHT_ENABLE_SECONDARY", true),
            return_target: env_or(&get, "MOONLIGHT_RETURN_TARGET", "primary").parse()?,
            return_fallback: env_or(&get, "MOONLIGHT_RETURN_FALLBACK", "none").parse()?,
            response_timing: env_or(&get, "MOONLIGHT_RESPONSE_TIMING", "wait_all").parse()?,
            max_body_capture_bytes: env_or(&get, "MOONLIGHT_MAX_BODY_CAPTURE_BYTES", "8192")
                .parse()?,
            max_request_body_bytes: env_or(
                &get,
                "MOONLIGHT_MAX_REQUEST_BODY_BYTES",
                &DEFAULT_MAX_REQUEST_BODY_BYTES.to_string(),
            )
            .parse()?,
            redact_headers: env_list(
                &get,
                "MOONLIGHT_REDACT_HEADERS",
                &[
                    "authorization",
                    "cookie",
                    "set-cookie",
                    "x-api-key",
                    "proxy-authorization",
                    "x-auth-token",
                    "x-csrf-token",
                ],
            ),
            redact_json_paths: env_list(&get, "MOONLIGHT_REDACT_JSON_PATHS", &[]),
            redact_query_params: env_list(
                &get,
                "MOONLIGHT_REDACT_QUERY_PARAMS",
                &[
                    "token",
                    "access_token",
                    "id_token",
                    "api_key",
                    "key",
                    "secret",
                    "password",
                ],
            ),
            ignored_json_paths: env_list(
                &get,
                "MOONLIGHT_IGNORED_JSON_PATHS",
                &["$.timestamp", "$.requestId", "$.traceId", "$.id"],
            ),
            ignored_headers: env_list(
                &get,
                "MOONLIGHT_IGNORED_HEADERS",
                &[
                    "date",
                    "server",
                    "set-cookie",
                    "x-request-id",
                    "traceparent",
                ],
            ),
            ignore_stderr: env_bool(&get, "MOONLIGHT_IGNORE_STDERR", false),
            storage_path: PathBuf::from(env_or(
                &get,
                "MOONLIGHT_STORAGE_PATH",
                "data/moonlight/http-runs.jsonl",
            )),
            cors_origins: env_list(
                &get,
                "MOONLIGHT_CORS_ORIGINS",
                &["http://127.0.0.1:5173", "http://localhost:5173"],
            ),
            admin_token: get("MOONLIGHT_ADMIN_TOKEN").filter(|value| !value.trim().is_empty()),
            retention_max_runs: env_optional(&get, "MOONLIGHT_RETENTION_MAX_RUNS")?,
            retention_max_bytes: env_optional(&get, "MOONLIGHT_RETENTION_MAX_BYTES")?,
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

fn env_optional<T>(get: &impl Fn(&str) -> Option<String>, key: &str) -> anyhow::Result<Option<T>>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    get(key)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.parse())
        .transpose()
        .map_err(Into::into)
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
    fn response_timing_defaults_to_wait_all() {
        let config = config_from(&[]).unwrap();
        assert_eq!(config.response_timing, ResponseTiming::WaitAll);
    }

    #[test]
    fn response_timing_parses_return_selected() {
        let config = config_from(&[("MOONLIGHT_RESPONSE_TIMING", "return_selected")]).unwrap();
        assert_eq!(config.response_timing, ResponseTiming::ReturnSelected);
    }

    #[test]
    fn invalid_response_timing_returns_error() {
        let error = config_from(&[("MOONLIGHT_RESPONSE_TIMING", "fast")]).unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid MOONLIGHT_RESPONSE_TIMING"));
    }

    #[test]
    fn return_target_defaults_to_primary() {
        let config = config_from(&[]).unwrap();
        assert_eq!(config.return_target, ReturnTarget::Primary);
    }

    #[test]
    fn return_fallback_defaults_to_none() {
        let config = config_from(&[]).unwrap();
        assert_eq!(config.return_fallback, ReturnFallback::None);
    }

    #[test]
    fn parses_local_first_hardening_env_vars() {
        let config = config_from(&[
            ("MOONLIGHT_CORS_ORIGINS", "http://example.test,*"),
            ("MOONLIGHT_ADMIN_TOKEN", "secret"),
            ("MOONLIGHT_MAX_REQUEST_BODY_BYTES", "128"),
            ("MOONLIGHT_REDACT_JSON_PATHS", "$.token,$.items[0].secret"),
            ("MOONLIGHT_REDACT_QUERY_PARAMS", "token,key"),
            ("MOONLIGHT_RETENTION_MAX_RUNS", "50"),
            ("MOONLIGHT_RETENTION_MAX_BYTES", "2048"),
        ])
        .unwrap();

        assert_eq!(
            config.cors_origins,
            vec!["http://example.test".to_string(), "*".to_string()]
        );
        assert_eq!(config.admin_token, Some("secret".to_string()));
        assert_eq!(config.max_request_body_bytes, 128);
        assert_eq!(
            config.redact_json_paths,
            vec!["$.token".to_string(), "$.items[0].secret".to_string()]
        );
        assert_eq!(
            config.redact_query_params,
            vec!["token".to_string(), "key".to_string()]
        );
        assert_eq!(config.retention_max_runs, Some(50));
        assert_eq!(config.retention_max_bytes, Some(2048));
    }

    #[test]
    fn serialized_config_omits_admin_token() {
        let config = config_from(&[("MOONLIGHT_ADMIN_TOKEN", "secret")]).unwrap();
        let json = serde_json::to_value(config).unwrap();

        assert!(json.get("admin_token").is_none());
    }
}
