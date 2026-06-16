use moonlight_core::{
    compare::CompareConfig,
    config::{
        extend, normalize_timeout, CliConfig as FileCliConfig, CliTargetConfig, MoonlightConfig,
        DEFAULT_CLI_STORAGE_PATH, DEFAULT_IGNORE_HEADERS, DEFAULT_IGNORE_JSON_PATHS,
        DEFAULT_MAX_BODY_CAPTURE_BYTES, DEFAULT_REVIEW_STATE_PATH, DEFAULT_TARGET_TIMEOUT_MS,
    },
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct CliDefaults {
    pub(crate) storage_path: PathBuf,
    pub(crate) max_body_capture_bytes: usize,
    pub(crate) ignore_json_paths: Vec<String>,
    pub(crate) ignore_json_path_patterns: Vec<String>,
    pub(crate) redact_json_paths: Vec<String>,
    pub(crate) redact_json_path_patterns: Vec<String>,
    pub(crate) ignore_headers: Vec<String>,
    pub(crate) ignore_stderr: bool,
    pub(crate) target_timeout_ms: u64,
    pub(crate) review_state_path: PathBuf,
    pub(crate) run: RunDefaults,
    pub(crate) batch: BatchDefaults,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RunDefaults {
    pub(crate) targets: CliTargetConfig,
    pub(crate) serial_targets: bool,
    pub(crate) quiet: bool,
    pub(crate) compact: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BatchDefaults {
    pub(crate) input: PathBuf,
    pub(crate) jobs: Option<usize>,
    pub(crate) quiet: bool,
    pub(crate) emit_runs: bool,
    pub(crate) serial_targets: bool,
}

impl Default for BatchDefaults {
    fn default() -> Self {
        Self {
            input: PathBuf::from("-"),
            jobs: None,
            quiet: false,
            emit_runs: false,
            serial_targets: false,
        }
    }
}

impl CliDefaults {
    pub(crate) fn from_config(config: &MoonlightConfig) -> Self {
        let mut defaults = Self {
            storage_path: PathBuf::from(DEFAULT_CLI_STORAGE_PATH),
            review_state_path: PathBuf::from(DEFAULT_REVIEW_STATE_PATH),
            max_body_capture_bytes: DEFAULT_MAX_BODY_CAPTURE_BYTES,
            ignore_json_paths: DEFAULT_IGNORE_JSON_PATHS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            ignore_json_path_patterns: Vec::new(),
            redact_json_paths: Vec::new(),
            redact_json_path_patterns: Vec::new(),
            ignore_headers: DEFAULT_IGNORE_HEADERS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            ignore_stderr: false,
            target_timeout_ms: DEFAULT_TARGET_TIMEOUT_MS,
            run: RunDefaults::default(),
            batch: BatchDefaults::default(),
        };

        if let Some(storage) = &config.storage {
            if let Some(path) = &storage.path {
                defaults.storage_path = path.clone();
            }
            if let Some(path) = &storage.review_state_path {
                defaults.review_state_path = path.clone();
            }
        }
        if let Some(comparison) = &config.comparison {
            if let Some(value) = comparison.max_body_capture_bytes {
                defaults.max_body_capture_bytes = value;
            }
            if let Some(value) = comparison.target_timeout_ms {
                defaults.target_timeout_ms = normalize_timeout(value);
            }
            extend(
                &mut defaults.ignore_json_paths,
                &comparison.ignore_json_paths,
            );
            extend(
                &mut defaults.ignore_json_path_patterns,
                &comparison.ignore_json_path_patterns,
            );
            extend(
                &mut defaults.redact_json_paths,
                &comparison.redact_json_paths,
            );
            extend(
                &mut defaults.redact_json_path_patterns,
                &comparison.redact_json_path_patterns,
            );
            extend(&mut defaults.ignore_headers, &comparison.ignore_headers);
            if let Some(value) = comparison.ignore_stderr {
                defaults.ignore_stderr = value;
            }
        }
        if let Some(cli) = &config.cli {
            defaults.apply_cli_config(cli);
        }

        defaults
    }

    fn apply_cli_config(&mut self, config: &FileCliConfig) {
        if let Some(run) = &config.run {
            self.run.targets = run.targets.clone();
            if let Some(value) = run.serial_targets {
                self.run.serial_targets = value;
            }
            if let Some(value) = run.quiet {
                self.run.quiet = value;
            }
            if let Some(value) = run.compact {
                self.run.compact = value;
            }
        }
        if let Some(batch) = &config.batch {
            if let Some(value) = &batch.input {
                self.batch.input = value.clone();
            }
            if let Some(value) = batch.jobs {
                self.batch.jobs = Some(value);
            }
            if let Some(value) = batch.quiet {
                self.batch.quiet = value;
            }
            if let Some(value) = batch.emit_runs {
                self.batch.emit_runs = value;
            }
            if let Some(value) = batch.serial_targets {
                self.batch.serial_targets = value;
            }
        }
    }
}

pub(crate) fn build_compare_config(
    ignore_json_paths: &[String],
    ignore_json_path_patterns: &[String],
    redact_json_paths: &[String],
    redact_json_path_patterns: &[String],
    ignore_headers: &[String],
    ignore_stderr: bool,
) -> CompareConfig {
    CompareConfig::new_with_patterns(
        ignore_json_paths,
        ignore_json_path_patterns,
        redact_json_paths,
        redact_json_path_patterns,
        ignore_headers,
        ignore_stderr,
    )
}
