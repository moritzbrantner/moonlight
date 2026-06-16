use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

pub const DEFAULT_CONFIG_PATH: &str = "moonlight.conf";
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_CLI_STORAGE_PATH: &str = "data/moonlight/cli-runs.jsonl";
pub const DEFAULT_HTTP_STORAGE_PATH: &str = "data/moonlight/http-runs.jsonl";
pub const DEFAULT_MAX_BODY_CAPTURE_BYTES: usize = 8192;
pub const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;

pub const DEFAULT_IGNORE_JSON_PATHS: &[&str] = &["$.timestamp", "$.requestId", "$.traceId", "$.id"];
pub const DEFAULT_IGNORE_HEADERS: &[&str] = &[
    "date",
    "server",
    "set-cookie",
    "x-request-id",
    "traceparent",
];
pub const DEFAULT_REDACT_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "proxy-authorization",
    "x-auth-token",
    "x-csrf-token",
];
pub const DEFAULT_REDACT_QUERY_PARAMS: &[&str] = &[
    "token",
    "access_token",
    "id_token",
    "api_key",
    "key",
    "secret",
    "password",
];
pub const DEFAULT_CORS_ORIGINS: &[&str] = &["http://127.0.0.1:5173", "http://localhost:5173"];

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
    pub ignore_json_paths: Vec<String>,
    pub ignore_headers: Vec<String>,
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
            other => anyhow::bail!("invalid return target {other:?}; use primary or candidate"),
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
            other => anyhow::bail!("invalid return fallback {other:?}; use none or primary"),
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
            "wait-all" => Ok(Self::WaitAll),
            "return-selected" => Ok(Self::ReturnSelected),
            other => {
                anyhow::bail!("invalid response timing {other:?}; use wait-all or return-selected")
            }
        }
    }
}

impl AppConfig {
    pub fn defaults() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR
                .parse()
                .expect("default bind addr is valid"),
            primary_url: String::new(),
            candidate_url: String::new(),
            secondary_url: String::new(),
            enable_secondary: false,
            return_target: ReturnTarget::Primary,
            return_fallback: ReturnFallback::None,
            response_timing: ResponseTiming::WaitAll,
            max_body_capture_bytes: DEFAULT_MAX_BODY_CAPTURE_BYTES,
            max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
            redact_headers: strings(DEFAULT_REDACT_HEADERS),
            redact_json_paths: Vec::new(),
            redact_query_params: strings(DEFAULT_REDACT_QUERY_PARAMS),
            ignore_json_paths: strings(DEFAULT_IGNORE_JSON_PATHS),
            ignore_headers: strings(DEFAULT_IGNORE_HEADERS),
            ignore_stderr: false,
            storage_path: PathBuf::from(DEFAULT_HTTP_STORAGE_PATH),
            cors_origins: strings(DEFAULT_CORS_ORIGINS),
            admin_token: None,
            retention_max_runs: None,
            retention_max_bytes: None,
        }
    }

    pub fn apply_shared_config(&mut self, config: &MoonlightConfig) {
        if let Some(storage) = &config.storage {
            if let Some(path) = &storage.path {
                self.storage_path = path.clone();
            }
        }
        if let Some(comparison) = &config.comparison {
            if let Some(value) = comparison.max_body_capture_bytes {
                self.max_body_capture_bytes = value;
            }
            extend(&mut self.ignore_json_paths, &comparison.ignore_json_paths);
            extend(&mut self.ignore_headers, &comparison.ignore_headers);
            if let Some(value) = comparison.ignore_stderr {
                self.ignore_stderr = value;
            }
            extend(&mut self.redact_headers, &comparison.redact_headers);
            extend(&mut self.redact_json_paths, &comparison.redact_json_paths);
            extend(
                &mut self.redact_query_params,
                &comparison.redact_query_params,
            );
        }
    }

    pub fn apply_http_config(&mut self, config: &HttpConfig) -> anyhow::Result<()> {
        if let Some(value) = &config.bind_addr {
            self.bind_addr = value.parse()?;
        }
        if let Some(value) = &config.primary_url {
            self.primary_url = normalize_base_url(value);
        }
        if let Some(value) = &config.candidate_url {
            self.candidate_url = normalize_base_url(value);
        }
        if let Some(value) = &config.secondary_url {
            self.secondary_url = normalize_base_url(value);
            self.enable_secondary = true;
        }
        if let Some(value) = config.enable_secondary {
            self.enable_secondary = value;
        }
        if let Some(value) = &config.return_target {
            self.return_target = value.parse()?;
        }
        if let Some(value) = &config.return_fallback {
            self.return_fallback = value.parse()?;
        }
        if let Some(value) = &config.response_timing {
            self.response_timing = value.parse()?;
        }
        if let Some(value) = config.max_request_body_bytes {
            self.max_request_body_bytes = value;
        }
        extend(&mut self.cors_origins, &config.cors_origins);
        if let Some(value) = &config.admin_token {
            self.admin_token = nonempty(value);
        }
        if let Some(value) = config.retention_max_runs {
            self.retention_max_runs = Some(value);
        }
        if let Some(value) = config.retention_max_bytes {
            self.retention_max_bytes = Some(value);
        }
        Ok(())
    }

    pub fn validate_http(&self) -> anyhow::Result<()> {
        if self.primary_url.trim().is_empty() {
            anyhow::bail!("primary URL is required; set [http].primary_url or --primary-url");
        }
        if self.candidate_url.trim().is_empty() {
            anyhow::bail!("candidate URL is required; set [http].candidate_url or --candidate-url");
        }
        if self.enable_secondary && self.secondary_url.trim().is_empty() {
            anyhow::bail!(
                "secondary URL is required when secondary is enabled; set [http].secondary_url or --secondary-url"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoonlightConfig {
    #[serde(default)]
    pub storage: Option<StorageConfig>,
    #[serde(default)]
    pub comparison: Option<ComparisonConfig>,
    #[serde(default)]
    pub cli: Option<CliConfig>,
    #[serde(default)]
    pub http: Option<HttpConfig>,
}

impl MoonlightConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", path.display()))?;
        toml::from_str(&content)
            .map_err(|error| anyhow::anyhow!("invalid {}: {error}", path.display()))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonConfig {
    pub max_body_capture_bytes: Option<usize>,
    #[serde(default)]
    pub ignore_json_paths: Vec<String>,
    #[serde(default)]
    pub ignore_headers: Vec<String>,
    pub ignore_stderr: Option<bool>,
    #[serde(default)]
    pub redact_headers: Vec<String>,
    #[serde(default)]
    pub redact_json_paths: Vec<String>,
    #[serde(default)]
    pub redact_query_params: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliConfig {
    #[serde(default)]
    pub run: Option<CliRunConfig>,
    #[serde(default)]
    pub batch: Option<CliBatchConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliRunConfig {
    #[serde(flatten)]
    pub targets: CliTargetConfig,
    pub serial_targets: Option<bool>,
    pub quiet: Option<bool>,
    pub compact: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliTargetConfig {
    pub primary: Option<String>,
    pub candidate: Option<String>,
    pub secondary: Option<String>,
    pub primary_argv: Option<Vec<String>>,
    pub candidate_argv: Option<Vec<String>>,
    pub secondary_argv: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliBatchConfig {
    pub input: Option<PathBuf>,
    pub jobs: Option<usize>,
    pub quiet: Option<bool>,
    pub emit_runs: Option<bool>,
    pub serial_targets: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    pub bind_addr: Option<String>,
    pub primary_url: Option<String>,
    pub candidate_url: Option<String>,
    pub secondary_url: Option<String>,
    pub enable_secondary: Option<bool>,
    pub return_target: Option<String>,
    pub return_fallback: Option<String>,
    pub response_timing: Option<String>,
    pub max_request_body_bytes: Option<usize>,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    pub admin_token: Option<String>,
    pub retention_max_runs: Option<usize>,
    pub retention_max_bytes: Option<u64>,
}

pub fn load_optional_config(
    path: Option<&Path>,
    no_config: bool,
) -> anyhow::Result<MoonlightConfig> {
    if no_config {
        return Ok(MoonlightConfig::default());
    }

    match path {
        Some(path) => MoonlightConfig::load(path),
        None => {
            let default_path = Path::new(DEFAULT_CONFIG_PATH);
            if default_path.exists() {
                MoonlightConfig::load(default_path)
            } else {
                Ok(MoonlightConfig::default())
            }
        }
    }
}

pub fn extend(target: &mut Vec<String>, values: &[String]) {
    target.extend(values.iter().filter_map(|value| nonempty(value)));
}

pub fn normalize_base_url(value: &str) -> String {
    value.trim_end_matches('/').to_string()
}

pub fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn response_timing_parses_kebab_case() {
        assert_eq!(
            "return-selected".parse::<ResponseTiming>().unwrap(),
            ResponseTiming::ReturnSelected
        );
        assert!("return_selected".parse::<ResponseTiming>().is_err());
    }

    #[test]
    fn app_defaults_require_http_targets() {
        let config = AppConfig::defaults();
        assert!(config.validate_http().is_err());
    }

    #[test]
    fn shared_config_extends_default_lists() {
        let parsed: MoonlightConfig = toml::from_str(
            r#"
            [comparison]
            ignore_headers = ["x-generated"]
            redact_json_paths = ["$.secret"]
            "#,
        )
        .unwrap();
        let mut config = AppConfig::defaults();
        config.apply_shared_config(&parsed);

        assert!(config.ignore_headers.contains(&"date".to_string()));
        assert!(config.ignore_headers.contains(&"x-generated".to_string()));
        assert_eq!(config.redact_json_paths, vec!["$.secret".to_string()]);
    }

    #[test]
    fn http_config_secondary_url_enables_secondary() {
        let parsed: MoonlightConfig = toml::from_str(
            r#"
            [http]
            primary_url = "http://primary/"
            candidate_url = "http://candidate/"
            secondary_url = "http://secondary/"
            "#,
        )
        .unwrap();
        let mut config = AppConfig::defaults();
        config
            .apply_http_config(parsed.http.as_ref().unwrap())
            .unwrap();

        assert_eq!(config.primary_url, "http://primary");
        assert_eq!(config.candidate_url, "http://candidate");
        assert_eq!(config.secondary_url, "http://secondary");
        assert!(config.enable_secondary);
        config.validate_http().unwrap();
    }

    #[test]
    fn serialized_config_omits_admin_token_and_uses_ignore_names() {
        let mut config = AppConfig::defaults();
        config.admin_token = Some("secret".to_string());
        let json = serde_json::to_value(config).unwrap();

        assert!(json.get("admin_token").is_none());
        assert!(json.get("ignore_json_paths").is_some());
        assert!(json.get("ignore_headers").is_some());
        assert!(json.get("ignored_json_paths").is_none());
    }

    #[test]
    fn explicit_missing_config_path_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.conf");

        assert!(load_optional_config(Some(&path), false).is_err());
    }
}
