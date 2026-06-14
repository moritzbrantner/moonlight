use moonlight_core::compare::CompareConfig;

pub(crate) const DEFAULT_IGNORED_JSON_PATHS: &[&str] =
    &["$.timestamp", "$.requestId", "$.traceId", "$.id"];
pub(crate) const DEFAULT_IGNORED_HEADERS: &[&str] = &[
    "date",
    "server",
    "set-cookie",
    "x-request-id",
    "traceparent",
];
pub(crate) const DEFAULT_MAX_BODY_CAPTURE_BYTES: usize = 8192;

pub(crate) fn build_compare_config(
    ignored_json_paths: &[String],
    ignored_headers: &[String],
    ignore_stderr: bool,
) -> CompareConfig {
    let ignored_json_paths = values_or_defaults(ignored_json_paths, DEFAULT_IGNORED_JSON_PATHS);
    let ignored_headers = values_or_defaults(ignored_headers, DEFAULT_IGNORED_HEADERS);
    CompareConfig::new(&ignored_json_paths, &ignored_headers, ignore_stderr)
}

fn values_or_defaults(values: &[String], defaults: &[&str]) -> Vec<String> {
    if values.is_empty() {
        defaults.iter().map(|value| (*value).to_string()).collect()
    } else {
        values.to_vec()
    }
}
