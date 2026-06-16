use clap::{ArgAction, Parser};
use moonlight_core::config::{
    extend, normalize_base_url, AppConfig, ResponseTiming, ReturnFallback, ReturnTarget,
};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "moonlight-http",
    version,
    about = "Run the Moonlight HTTP shadow proxy",
    after_help = "Examples:\n  moonlight-http --primary-url http://127.0.0.1:3001 --candidate-url http://127.0.0.1:3002\n  moonlight-http --config moonlight.conf --response-timing return-selected"
)]
pub(crate) struct ProxyArgs {
    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Config",
        conflicts_with = "no_config",
        help = "TOML config file to read instead of ./moonlight.conf"
    )]
    pub(crate) config: Option<PathBuf>,

    #[arg(long, help_heading = "Config", help = "Do not read ./moonlight.conf")]
    pub(crate) no_config: bool,

    #[arg(
        long,
        value_name = "ADDR",
        help_heading = "HTTP",
        help = "HTTP proxy bind address"
    )]
    bind_addr: Option<SocketAddr>,

    #[arg(
        long,
        value_name = "URL",
        help_heading = "Targets",
        help = "Primary reference target base URL"
    )]
    primary_url: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help_heading = "Targets",
        help = "Candidate target base URL"
    )]
    candidate_url: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help_heading = "Targets",
        help = "Secondary reference target base URL"
    )]
    secondary_url: Option<String>,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with = "disable_secondary",
        help_heading = "Targets",
        help = "Enable the secondary reference target"
    )]
    enable_secondary: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with = "enable_secondary",
        help_heading = "Targets",
        help = "Disable the secondary reference target"
    )]
    disable_secondary: bool,

    #[arg(
        long,
        value_name = "TARGET",
        value_parser = ["primary", "candidate"],
        help_heading = "Response",
        help = "Target response returned to callers"
    )]
    return_target: Option<String>,

    #[arg(
        long,
        value_name = "MODE",
        value_parser = ["none", "primary"],
        help_heading = "Response",
        help = "Fallback behavior when returning the candidate response fails"
    )]
    return_fallback: Option<String>,

    #[arg(
        long,
        value_name = "MODE",
        value_parser = ["wait-all", "return-selected"],
        help_heading = "Response",
        help = "Whether to wait for all targets before returning a response"
    )]
    response_timing: Option<String>,

    #[arg(
        long,
        value_name = "BYTES",
        help_heading = "Comparison",
        help = "Maximum response body bytes to store"
    )]
    max_body_capture_bytes: Option<usize>,

    #[arg(
        long,
        value_name = "BYTES",
        help_heading = "HTTP",
        help = "Maximum request body bytes to accept"
    )]
    max_request_body_bytes: Option<usize>,

    #[arg(
        long = "redact-header",
        value_name = "HEADER",
        action = ArgAction::Append,
        help_heading = "Redaction",
        help = "Header name to redact from stored observations"
    )]
    redact_headers: Vec<String>,

    #[arg(
        long = "redact-json-path",
        value_name = "PATH",
        action = ArgAction::Append,
        help_heading = "Redaction",
        help = "Exact JSON body path to redact from stored previews and diffs"
    )]
    redact_json_paths: Vec<String>,

    #[arg(
        long = "redact-query-param",
        value_name = "PARAM",
        action = ArgAction::Append,
        help_heading = "Redaction",
        help = "Query parameter name to redact from stored request input"
    )]
    redact_query_params: Vec<String>,

    #[arg(
        long = "ignore-json-path",
        value_name = "PATH",
        action = ArgAction::Append,
        help_heading = "Comparison",
        help = "Exact JSON diff path to ignore"
    )]
    ignore_json_paths: Vec<String>,

    #[arg(
        long = "ignore-header",
        value_name = "HEADER",
        action = ArgAction::Append,
        help_heading = "Comparison",
        help = "Header name to ignore during comparison"
    )]
    ignore_headers: Vec<String>,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help_heading = "Comparison",
        help = "Ignore stderr differences"
    )]
    ignore_stderr: bool,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Storage",
        help = "JSONL storage file"
    )]
    storage_path: Option<PathBuf>,

    #[arg(
        long = "cors-origin",
        value_name = "ORIGIN",
        action = ArgAction::Append,
        help_heading = "HTTP",
        help = "Allowed admin API CORS origin"
    )]
    cors_origins: Vec<String>,

    #[arg(
        long,
        value_name = "TOKEN",
        help_heading = "HTTP",
        help = "Admin API bearer token"
    )]
    admin_token: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help_heading = "Storage",
        help = "Maximum active JSONL runs to retain"
    )]
    retention_max_runs: Option<usize>,

    #[arg(
        long,
        value_name = "BYTES",
        help_heading = "Storage",
        help = "Maximum active JSONL bytes to retain"
    )]
    retention_max_bytes: Option<u64>,
}

impl ProxyArgs {
    pub(crate) fn apply_to(self, mut config: AppConfig) -> anyhow::Result<AppConfig> {
        if let Some(bind_addr) = self.bind_addr {
            config.bind_addr = bind_addr;
        }
        if let Some(primary_url) = self.primary_url {
            config.primary_url = normalize_base_url(&primary_url);
        }
        if let Some(candidate_url) = self.candidate_url {
            config.candidate_url = normalize_base_url(&candidate_url);
        }
        if let Some(secondary_url) = self.secondary_url {
            config.secondary_url = normalize_base_url(&secondary_url);
            config.enable_secondary = true;
        }
        if self.enable_secondary {
            config.enable_secondary = true;
        }
        if self.disable_secondary {
            config.enable_secondary = false;
        }
        if let Some(return_target) = self.return_target {
            config.return_target = return_target.parse::<ReturnTarget>()?;
        }
        if let Some(return_fallback) = self.return_fallback {
            config.return_fallback = return_fallback.parse::<ReturnFallback>()?;
        }
        if let Some(response_timing) = self.response_timing {
            config.response_timing = response_timing.parse::<ResponseTiming>()?;
        }
        if let Some(max_body_capture_bytes) = self.max_body_capture_bytes {
            config.max_body_capture_bytes = max_body_capture_bytes;
        }
        if let Some(max_request_body_bytes) = self.max_request_body_bytes {
            config.max_request_body_bytes = max_request_body_bytes;
        }
        extend(&mut config.redact_headers, &self.redact_headers);
        extend(&mut config.redact_json_paths, &self.redact_json_paths);
        extend(&mut config.redact_query_params, &self.redact_query_params);
        extend(&mut config.ignore_json_paths, &self.ignore_json_paths);
        extend(&mut config.ignore_headers, &self.ignore_headers);
        if self.ignore_stderr {
            config.ignore_stderr = true;
        }
        if let Some(storage_path) = self.storage_path {
            config.storage_path = storage_path;
        }
        extend(&mut config.cors_origins, &self.cors_origins);
        if let Some(admin_token) = self.admin_token {
            config.admin_token = moonlight_core::config::nonempty(&admin_token);
        }
        if let Some(retention_max_runs) = self.retention_max_runs {
            config.retention_max_runs = Some(retention_max_runs);
        }
        if let Some(retention_max_bytes) = self.retention_max_bytes {
            config.retention_max_bytes = Some(retention_max_bytes);
        }

        config.validate_http()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> ProxyArgs {
        let argv = std::iter::once("moonlight-http").chain(args.iter().copied());
        ProxyArgs::try_parse_from(argv).unwrap()
    }

    #[test]
    fn primary_and_candidate_urls_are_required() {
        let error = parse(&[]).apply_to(AppConfig::defaults()).unwrap_err();

        assert!(error.to_string().contains("primary URL is required"));
    }

    #[test]
    fn target_urls_and_secondary_flag_merge_into_defaults() {
        let config = parse(&[
            "--primary-url",
            "http://primary/",
            "--candidate-url",
            "http://candidate/",
            "--secondary-url",
            "http://secondary/",
        ])
        .apply_to(AppConfig::defaults())
        .unwrap();

        assert_eq!(config.primary_url, "http://primary");
        assert_eq!(config.candidate_url, "http://candidate");
        assert_eq!(config.secondary_url, "http://secondary");
        assert!(config.enable_secondary);
    }

    #[test]
    fn response_timing_uses_kebab_case_values() {
        let config = parse(&[
            "--primary-url",
            "http://primary",
            "--candidate-url",
            "http://candidate",
            "--response-timing",
            "return-selected",
        ])
        .apply_to(AppConfig::defaults())
        .unwrap();

        assert_eq!(config.response_timing, ResponseTiming::ReturnSelected);

        let argv = ["moonlight-http", "--response-timing", "return_selected"];
        assert!(ProxyArgs::try_parse_from(argv).is_err());
    }

    #[test]
    fn ignore_flags_extend_defaults() {
        let config = parse(&[
            "--primary-url",
            "http://primary",
            "--candidate-url",
            "http://candidate",
            "--ignore-header",
            "x-generated",
            "--ignore-json-path",
            "$.volatile",
        ])
        .apply_to(AppConfig::defaults())
        .unwrap();

        assert!(config.ignore_headers.contains(&"date".to_string()));
        assert!(config.ignore_headers.contains(&"x-generated".to_string()));
        assert!(config.ignore_json_paths.contains(&"$.id".to_string()));
        assert!(config.ignore_json_paths.contains(&"$.volatile".to_string()));
    }

    #[test]
    fn old_ignored_flags_are_rejected() {
        let argv = ["moonlight-http", "--ignored-header", "date"];

        assert!(ProxyArgs::try_parse_from(argv).is_err());
    }
}
