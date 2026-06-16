use crate::{config::AppConfig, ComparisonRun, DiffEntry, TargetObservation};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Markdown,
    Json,
}

impl std::str::FromStr for ReportFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => anyhow::bail!("invalid report format {other:?}; use markdown or json"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport<'a> {
    pub run: &'a ComparisonRun,
    pub config: Option<&'a AppConfig>,
}

pub fn render_report(
    run: &ComparisonRun,
    config: Option<&AppConfig>,
    format: ReportFormat,
) -> anyhow::Result<String> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(&RunReport { run, config })?),
        ReportFormat::Markdown => Ok(render_markdown(run, config)),
    }
}

fn render_markdown(run: &ComparisonRun, config: Option<&AppConfig>) -> String {
    let mut output = String::new();
    output.push_str(&format!("# Moonlight Report {}\n\n", run.id));
    output.push_str(&format!(
        "- Classification: `{:?}`\n",
        run.comparison.classification
    ));
    output.push_str(&format!("- Adapter: `{:?}`\n", run.adapter));
    output.push_str(&format!("- Timestamp: `{}`\n", run.timestamp.to_rfc3339()));
    output.push_str(&format!("- Input: `{}`\n\n", input_label(run)));

    output.push_str("## Targets\n\n");
    push_target(&mut output, "Primary", &run.primary);
    push_target(&mut output, "Candidate", &run.candidate);
    if let Some(secondary) = &run.secondary {
        push_target(&mut output, "Secondary", secondary);
    }

    push_diffs(
        &mut output,
        "Noise-filtered Diffs",
        &run.comparison.noise_filtered_diffs,
    );
    push_diffs(
        &mut output,
        "Raw Candidate Diffs",
        &run.comparison.raw_candidate_diffs,
    );
    push_diffs(
        &mut output,
        "Reference Noise",
        &run.comparison.reference_noise,
    );

    if let Some(config) = config {
        output.push_str("## Relevant Config\n\n");
        output.push_str(&format!(
            "- Return target: `{:?}`\n- Response timing: `{:?}`\n- Max body capture bytes: `{}`\n- Target timeout ms: `{}`\n- Ignore JSON paths: `{}`\n- Ignore JSON path patterns: `{}`\n- Ignore headers: `{}`\n\n",
            config.return_target,
            config.response_timing,
            config.max_body_capture_bytes,
            config.target_timeout_ms,
            config.ignore_json_paths.join(", "),
            config.ignore_json_path_patterns.join(", "),
            config.ignore_headers.join(", "),
        ));
    }

    output
}

fn input_label(run: &ComparisonRun) -> String {
    match &run.input {
        crate::RunInput::Http {
            method,
            path,
            query,
        } => match query {
            Some(query) => format!("{method} {path}?{query}"),
            None => format!("{method} {path}"),
        },
        crate::RunInput::Cli {
            primary_command,
            candidate_command,
            ..
        } => format!("{primary_command} vs {candidate_command}"),
        crate::RunInput::Project {
            project, check_id, ..
        } => format!("{project} / {check_id}"),
    }
}

fn push_target(output: &mut String, label: &str, target: &TargetObservation) {
    output.push_str(&format!("### {label}\n\n"));
    output.push_str(&format!(
        "- Status: `{}`\n- Latency: `{} ms`\n- Body bytes: `{}`\n- Body SHA-256: `{}`\n- Truncated: `{}`\n",
        target
            .status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "ERR".to_string()),
        target.latency_ms,
        target.body.size_bytes,
        target.body.sha256,
        target.body.truncated,
    ));
    if let Some(error) = &target.error {
        output.push_str(&format!("- Error: `{}`\n", escape_inline(error)));
    }
    if !target.body.preview.is_empty() {
        output.push_str("\n```text\n");
        output.push_str(&target.body.preview);
        output.push_str("\n```\n");
    }
    output.push('\n');
}

fn push_diffs(output: &mut String, title: &str, diffs: &[DiffEntry]) {
    output.push_str(&format!("## {title}\n\n"));
    if diffs.is_empty() {
        output.push_str("No diffs.\n\n");
        return;
    }
    for diff in diffs {
        output.push_str(&format!(
            "- `{}` `{}`: {}\n",
            format!("{:?}", diff.kind).to_ascii_lowercase(),
            diff.path,
            diff.message
        ));
        output.push_str(&format!(
            "  - primary: `{}`\n  - candidate: `{}`\n  - secondary: `{}`\n",
            diff.primary.as_deref().unwrap_or("null"),
            diff.candidate.as_deref().unwrap_or("null"),
            diff.secondary.as_deref().unwrap_or("null"),
        ));
    }
    output.push('\n');
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "'")
}
