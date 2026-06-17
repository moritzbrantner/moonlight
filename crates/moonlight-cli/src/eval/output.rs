use super::summary::{classification_key, EvalSummary};
use anyhow::bail;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(super) enum EvalOutputFormat {
    Text,
    Json,
    Markdown,
}

impl std::str::FromStr for EvalOutputFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            other => bail!("invalid eval output format {other:?}"),
        }
    }
}

pub(super) fn print_summary(summary: &EvalSummary, format: EvalOutputFormat) -> anyhow::Result<()> {
    match format {
        EvalOutputFormat::Json => println!("{}", serde_json::to_string_pretty(summary)?),
        EvalOutputFormat::Text => print_text_summary(summary),
        EvalOutputFormat::Markdown => print_markdown_summary(summary),
    }
    Ok(())
}

fn print_text_summary(summary: &EvalSummary) {
    println!("Moonlight eval {}", summary.eval_id);
    println!("project: {}", summary.project);
    println!("baseline: {}", summary.baseline_ref);
    println!("candidate: {}", summary.candidate_source);
    println!();
    println!(
        "{} checks, {}",
        summary.total_checks,
        format_classification_counts(&summary.classifications)
    );
    for failure in &summary.failed_checks {
        println!();
        println!("FAIL {}", failure.check_id);
        println!(
            "  status: primary {}, candidate {}",
            display_status(failure.primary_status),
            display_status(failure.candidate_status)
        );
        println!(
            "  classification: {}",
            classification_key(&failure.classification)
        );
        println!("  diff: {}", failure.diff_summary);
        println!("  run: {}", failure.run_id);
    }
}

fn print_markdown_summary(summary: &EvalSummary) {
    println!("# Moonlight Eval {}", summary.eval_id);
    println!();
    println!("- Project: `{}`", summary.project);
    println!("- Baseline: `{}`", summary.baseline_ref);
    println!("- Candidate: `{}`", summary.candidate_source);
    println!("- Checks: `{}`", summary.total_checks);
    println!(
        "- Classifications: `{}`",
        format_classification_counts(&summary.classifications)
    );
    if summary.failed_checks.is_empty() {
        println!();
        println!("No failed checks.");
        return;
    }
    println!();
    println!("## Failed Checks");
    for failure in &summary.failed_checks {
        println!();
        println!("### {}", failure.check_id);
        println!();
        println!(
            "- Classification: `{}`",
            classification_key(&failure.classification)
        );
        println!(
            "- Status: primary `{}`, candidate `{}`",
            display_status(failure.primary_status),
            display_status(failure.candidate_status)
        );
        println!("- Diff: `{}`", failure.diff_summary.replace('`', "'"));
        println!("- Run: `{}`", failure.run_id);
    }
}

fn format_classification_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "no runs".to_string();
    }
    counts
        .iter()
        .map(|(classification, count)| format!("{count} {classification}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_status(status: Option<u16>) -> String {
    status
        .map(|value| value.to_string())
        .unwrap_or_else(|| "ERR".to_string())
}
