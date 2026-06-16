use crate::{
    args::{EvalReportArgs, EvalRunArgs},
    command::run_command,
    config::{build_compare_config, CliDefaults},
    eval_config::{CheckCommand, CheckConfig, KeepWorktrees, ProjectEvalConfig},
    types::{CommandForm, TargetCommand},
    worktree::{
        cleanup_worktrees, prepare_worktrees, resolve_repo_root, CandidateSource, EvalWorktrees,
    },
};
use anyhow::{bail, Context};
use bytes::Bytes;
use chrono::Utc;
use futures::{stream, StreamExt};
use moonlight_core::{
    compare::{capture_body, compare_targets, CapturedTarget},
    storage::RunWriter,
    Adapter, Classification, ComparisonRun, RunInput,
};
use regex::Regex;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
};
use uuid::Uuid;

pub(crate) async fn run(args: EvalRunArgs, defaults: &CliDefaults) -> anyhow::Result<ExitCode> {
    let candidate_source = candidate_source(&args)?;
    let mut config = ProjectEvalConfig::load(&args.project)?;
    apply_overrides(&mut config, &args)?;
    validate_patterns(&config.checks)?;

    let eval_id = Uuid::new_v4();
    let repo_root = resolve_repo_root(&config.project.repo)?;
    let work_dir = absolutize(&repo_root, &config.eval.work_dir);
    let storage_path = args
        .storage
        .storage_path
        .unwrap_or_else(|| defaults.storage_path.clone());
    let worktrees = match prepare_worktrees(
        &repo_root,
        &work_dir,
        eval_id,
        &config.project.baseline_ref,
        &candidate_source,
    ) {
        Ok(paths) => paths,
        Err(error) => {
            let paths = EvalWorktrees::paths(&work_dir, eval_id);
            let _ = cleanup_worktrees(&repo_root, &paths);
            return Err(error);
        }
    };

    let summary = execute_eval(
        eval_id,
        &config,
        &repo_root,
        &candidate_source,
        &worktrees,
        storage_path,
    )
    .await?;
    let success = summary.failed_checks.is_empty();
    if should_cleanup(config.eval.keep_worktrees, success) {
        cleanup_worktrees(&repo_root, &worktrees)?;
    }

    if !args.quiet {
        print_summary(&summary, args.format.parse()?)?;
    }

    if summary.failed_checks.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

pub(crate) async fn report(
    args: EvalReportArgs,
    defaults: &CliDefaults,
) -> anyhow::Result<ExitCode> {
    let storage_path = args
        .storage
        .storage_path
        .unwrap_or_else(|| defaults.storage_path.clone());
    let runs = read_eval_runs(&storage_path, args.eval_id)?;
    if runs.is_empty() {
        bail!("no project eval runs found for {}", args.eval_id);
    }
    let summary = EvalSummary::from_runs(args.eval_id, &runs)?;
    print_summary(&summary, args.format.parse()?)?;
    Ok(ExitCode::SUCCESS)
}

async fn execute_eval(
    eval_id: Uuid,
    config: &ProjectEvalConfig,
    repo_root: &Path,
    candidate_source: &CandidateSource,
    worktrees: &EvalWorktrees,
    storage_path: PathBuf,
) -> anyhow::Result<EvalSummary> {
    let writer = RunWriter::open(storage_path).await?;
    let compare_config = Arc::new(build_compare_config(&[], &[], &[], &[], &[], false));
    let jobs = config.eval.jobs.max(1);
    let checks = config.checks.clone();
    let mut runs = stream::iter(checks.into_iter().map(|check| {
        let compare_config = Arc::clone(&compare_config);
        async move {
            execute_check(
                eval_id,
                config,
                repo_root,
                candidate_source,
                worktrees,
                check,
                compare_config,
            )
            .await
        }
    }))
    .buffer_unordered(jobs);

    let mut summary = EvalSummary::new(
        eval_id,
        config.project.name.clone(),
        repo_root.display().to_string(),
        config.project.baseline_ref.clone(),
        candidate_source.label(),
    );

    while let Some(run) = runs.next().await {
        let run = run?;
        writer.append(&run).await?;
        summary.record(&run);
    }
    writer.flush().await?;
    Ok(summary)
}

async fn execute_check(
    eval_id: Uuid,
    config: &ProjectEvalConfig,
    repo_root: &Path,
    candidate_source: &CandidateSource,
    worktrees: &EvalWorktrees,
    check: CheckConfig,
    compare_config: Arc<moonlight_core::compare::CompareConfig>,
) -> anyhow::Result<ComparisonRun> {
    let primary_command = target_command(&check, &worktrees.primary);
    let candidate_command = target_command(&check, &worktrees.candidate);
    let max_body_capture_bytes = config.eval.max_body_capture_bytes;
    let timeout_ms = check.timeout_ms.unwrap_or(config.eval.target_timeout_ms);
    let (mut primary, mut candidate) = tokio::join!(
        run_command(
            "primary",
            &primary_command,
            max_body_capture_bytes,
            timeout_ms
        ),
        run_command(
            "candidate",
            &candidate_command,
            max_body_capture_bytes,
            timeout_ms
        )
    );
    normalize_target(&mut primary, &check, max_body_capture_bytes)?;
    normalize_target(&mut candidate, &check, max_body_capture_bytes)?;

    let compare_config = if check.ignore_stderr {
        Arc::new(build_compare_config(&[], &[], &[], &[], &[], true))
    } else {
        compare_config
    };
    let comparison = compare_targets(&primary, &candidate, None, &compare_config);

    Ok(ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        adapter: Adapter::Project,
        input: RunInput::Project {
            eval_id,
            project: config.project.name.clone(),
            check_id: check.id,
            check_name: check.name,
            repo: repo_root.display().to_string(),
            baseline_ref: config.project.baseline_ref.clone(),
            candidate_source: candidate_source.label(),
            primary_command: primary_command.display(),
            candidate_command: candidate_command.display(),
            secondary_command: None,
        },
        request_headers: BTreeMap::new(),
        request_body: capture_body(&[], max_body_capture_bytes),
        primary: primary.observation,
        candidate: candidate.observation,
        secondary: None,
        comparison,
    })
}

fn target_command(check: &CheckConfig, worktree: &Path) -> TargetCommand {
    let cwd = Some(worktree.join(&check.cwd));
    let env = check.env.clone();
    let form = match &check.command {
        CheckCommand::Shell(command) => CommandForm::Shell(command.clone()),
        CheckCommand::Argv(argv) => CommandForm::Argv(argv.clone()),
    };
    TargetCommand { form, cwd, env }
}

fn normalize_target(
    target: &mut CapturedTarget,
    check: &CheckConfig,
    max_body_capture_bytes: usize,
) -> anyhow::Result<()> {
    if check.ignore_stdout {
        replace_stdout(target, Bytes::new(), max_body_capture_bytes);
    } else if !check.normalize_stdout_patterns.is_empty() {
        let bytes = normalize_bytes(&target.body_bytes, &check.normalize_stdout_patterns)?;
        replace_stdout(target, bytes, max_body_capture_bytes);
    }

    if let Some(stderr) = &target.observation.stderr {
        if !check.normalize_stderr_patterns.is_empty() {
            let bytes = normalize_bytes(&target.stderr_bytes, &check.normalize_stderr_patterns)?;
            target.stderr_bytes = bytes.clone();
            target.observation.stderr = Some(capture_body(&bytes, max_body_capture_bytes));
        } else {
            target.observation.stderr = Some(stderr.clone());
        }
    }
    Ok(())
}

fn replace_stdout(target: &mut CapturedTarget, bytes: Bytes, max_body_capture_bytes: usize) {
    target.body_bytes = bytes.clone();
    target.observation.body = capture_body(&bytes, max_body_capture_bytes);
}

fn normalize_bytes(bytes: &Bytes, patterns: &[String]) -> anyhow::Result<Bytes> {
    let mut value = String::from_utf8_lossy(bytes).to_string();
    for pattern in patterns {
        let regex = Regex::new(pattern)
            .with_context(|| format!("invalid normalize pattern {pattern:?}"))?;
        value = regex.replace_all(&value, "<normalized>").to_string();
    }
    Ok(Bytes::from(value))
}

fn validate_patterns(checks: &[CheckConfig]) -> anyhow::Result<()> {
    for check in checks {
        for pattern in check
            .normalize_stdout_patterns
            .iter()
            .chain(check.normalize_stderr_patterns.iter())
        {
            Regex::new(pattern)
                .with_context(|| format!("invalid normalize pattern in check {}", check.id))?;
        }
    }
    Ok(())
}

fn apply_overrides(config: &mut ProjectEvalConfig, args: &EvalRunArgs) -> anyhow::Result<()> {
    if let Some(repo) = &args.repo {
        config.project.repo = repo.clone();
    }
    if let Some(baseline_ref) = &args.baseline_ref {
        config.project.baseline_ref = baseline_ref.clone();
    }
    if let Some(keep_worktrees) = &args.keep_worktrees {
        config.eval.keep_worktrees = keep_worktrees.parse()?;
    }
    if let Some(jobs) = args.jobs {
        config.eval.jobs = jobs.max(1);
    }
    Ok(())
}

fn candidate_source(args: &EvalRunArgs) -> anyhow::Result<CandidateSource> {
    match (&args.candidate_ref, &args.candidate_patch) {
        (Some(_), Some(_)) => bail!("provide exactly one of --candidate-ref or --candidate-patch"),
        (Some(candidate_ref), None) => {
            if candidate_ref.trim().is_empty() {
                bail!("--candidate-ref must not be empty");
            }
            Ok(CandidateSource::Ref(candidate_ref.clone()))
        }
        (None, Some(candidate_patch)) => Ok(CandidateSource::Patch(candidate_patch.clone())),
        (None, None) => bail!("provide exactly one of --candidate-ref or --candidate-patch"),
    }
}

fn should_cleanup(keep_worktrees: KeepWorktrees, success: bool) -> bool {
    match keep_worktrees {
        KeepWorktrees::Never => true,
        KeepWorktrees::Failed => success,
        KeepWorktrees::Always => false,
    }
}

fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[derive(Debug, Clone, Copy)]
enum EvalOutputFormat {
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

#[derive(Debug, Clone, Serialize)]
struct EvalSummary {
    eval_id: Uuid,
    project: String,
    repo: String,
    baseline_ref: String,
    candidate_source: String,
    total_checks: usize,
    classifications: BTreeMap<String, usize>,
    failed_checks: Vec<EvalFailedCheck>,
    runs: Vec<EvalRunItem>,
}

#[derive(Debug, Clone, Serialize)]
struct EvalRunItem {
    check_id: String,
    check_name: Option<String>,
    run_id: Uuid,
    classification: Classification,
}

#[derive(Debug, Clone, Serialize)]
struct EvalFailedCheck {
    check_id: String,
    check_name: Option<String>,
    run_id: Uuid,
    classification: Classification,
    primary_status: Option<u16>,
    candidate_status: Option<u16>,
    diff_summary: String,
}

impl EvalSummary {
    fn new(
        eval_id: Uuid,
        project: String,
        repo: String,
        baseline_ref: String,
        candidate_source: String,
    ) -> Self {
        Self {
            eval_id,
            project,
            repo,
            baseline_ref,
            candidate_source,
            total_checks: 0,
            classifications: BTreeMap::new(),
            failed_checks: Vec::new(),
            runs: Vec::new(),
        }
    }

    fn from_runs(eval_id: Uuid, runs: &[ComparisonRun]) -> anyhow::Result<Self> {
        let first = runs.first().context("eval has no runs")?;
        let RunInput::Project {
            project,
            repo,
            baseline_ref,
            candidate_source,
            ..
        } = &first.input
        else {
            bail!("stored eval run has non-project input");
        };
        let mut summary = Self::new(
            eval_id,
            project.clone(),
            repo.clone(),
            baseline_ref.clone(),
            candidate_source.clone(),
        );
        for run in runs {
            summary.record(run);
        }
        Ok(summary)
    }

    fn record(&mut self, run: &ComparisonRun) {
        self.total_checks += 1;
        let key = classification_key(&run.comparison.classification);
        *self.classifications.entry(key).or_insert(0) += 1;

        let (check_id, check_name) = match &run.input {
            RunInput::Project {
                check_id,
                check_name,
                ..
            } => (check_id.clone(), check_name.clone()),
            _ => ("unknown".to_string(), None),
        };
        self.runs.push(EvalRunItem {
            check_id: check_id.clone(),
            check_name: check_name.clone(),
            run_id: run.id,
            classification: run.comparison.classification.clone(),
        });

        if !is_success_classification(&run.comparison.classification) {
            self.failed_checks.push(EvalFailedCheck {
                check_id,
                check_name,
                run_id: run.id,
                classification: run.comparison.classification.clone(),
                primary_status: run.primary.status,
                candidate_status: run.candidate.status,
                diff_summary: run.comparison.raw_diff_summary.clone(),
            });
        }
    }
}

fn classification_key(classification: &Classification) -> String {
    serde_json::to_value(classification)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{classification:?}").to_ascii_lowercase())
}

fn is_success_classification(classification: &Classification) -> bool {
    matches!(
        classification,
        Classification::Match | Classification::ReferenceNoise
    )
}

fn print_summary(summary: &EvalSummary, format: EvalOutputFormat) -> anyhow::Result<()> {
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

fn read_eval_runs(storage_path: &Path, eval_id: Uuid) -> anyhow::Result<Vec<ComparisonRun>> {
    if !storage_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(storage_path)
        .with_context(|| format!("failed to read {}", storage_path.display()))?;
    let mut runs = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let run: ComparisonRun = serde_json::from_str(line)
            .with_context(|| format!("invalid JSONL in {}", storage_path.display()))?;
        if matches!(&run.input, RunInput::Project { eval_id: id, .. } if *id == eval_id) {
            runs.push(run);
        }
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_source_requires_exactly_one_input() {
        let mut args = EvalRunArgs {
            storage: crate::args::StorageArgs { storage_path: None },
            project: PathBuf::from("moonlight.eval.toml"),
            repo: None,
            baseline_ref: None,
            candidate_ref: None,
            candidate_patch: None,
            format: "text".to_string(),
            keep_worktrees: None,
            jobs: None,
            quiet: false,
        };
        assert!(candidate_source(&args).is_err());
        args.candidate_ref = Some("candidate".to_string());
        assert!(candidate_source(&args).is_ok());
        args.candidate_patch = Some(PathBuf::from("patch.diff"));
        assert!(candidate_source(&args).is_err());
    }

    #[test]
    fn normalization_replaces_matching_output() {
        let output = normalize_bytes(
            &Bytes::from("built in 123ms at /tmp/demo"),
            &["[0-9]+ms".to_string(), "/tmp/[^ ]+".to_string()],
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(output.to_vec()).unwrap(),
            "built in <normalized> at <normalized>"
        );
    }
}
