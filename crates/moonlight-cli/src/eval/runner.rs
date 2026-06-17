use super::{
    normalize::{normalize_target, prepare_checks, PreparedEvalCheck},
    output::{print_summary, EvalOutputFormat},
    report::read_eval_runs,
    summary::EvalSummary,
};
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
use anyhow::bail;
use chrono::Utc;
use futures::{stream, StreamExt};
use moonlight_core::{
    compare::{capture_body, compare_targets},
    storage::RunWriter,
    Adapter, ComparisonRun, RunInput,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
};
use uuid::Uuid;

pub(crate) async fn run(args: EvalRunArgs, defaults: &CliDefaults) -> anyhow::Result<ExitCode> {
    let candidate_source = candidate_source(&args)?;
    let mut config = ProjectEvalConfig::load(&args.project)?;
    apply_overrides(&mut config, &args)?;
    let checks = prepare_checks(config.checks.clone())?;

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
        checks,
    )
    .await?;
    let success = summary.failed_checks.is_empty();
    if should_cleanup(config.eval.keep_worktrees, success) {
        cleanup_worktrees(&repo_root, &worktrees)?;
    }

    if !args.quiet {
        print_summary(&summary, args.format.parse::<EvalOutputFormat>()?)?;
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
    print_summary(&summary, args.format.parse::<EvalOutputFormat>()?)?;
    Ok(ExitCode::SUCCESS)
}

async fn execute_eval(
    eval_id: Uuid,
    config: &ProjectEvalConfig,
    repo_root: &Path,
    candidate_source: &CandidateSource,
    worktrees: &EvalWorktrees,
    storage_path: PathBuf,
    checks: Vec<PreparedEvalCheck>,
) -> anyhow::Result<EvalSummary> {
    let writer = RunWriter::open(storage_path).await?;
    let compare_config = Arc::new(build_compare_config(&[], &[], &[], &[], &[], false));
    let jobs = config.eval.jobs.max(1);
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
    check: PreparedEvalCheck,
    compare_config: Arc<moonlight_core::compare::CompareConfig>,
) -> anyhow::Result<ComparisonRun> {
    let primary_command = target_command(&check.config, &worktrees.primary);
    let candidate_command = target_command(&check.config, &worktrees.candidate);
    let max_body_capture_bytes = config.eval.max_body_capture_bytes;
    let timeout_ms = check
        .config
        .timeout_ms
        .unwrap_or(config.eval.target_timeout_ms);
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

    let compare_config = if check.config.ignore_stderr {
        Arc::new(build_compare_config(&[], &[], &[], &[], &[], true))
    } else {
        compare_config
    };
    let comparison = compare_targets(&primary, &candidate, None, &compare_config);
    let check_id = check.config.id;
    let check_name = check.config.name;

    Ok(ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        adapter: Adapter::Project,
        input: RunInput::Project {
            eval_id,
            project: config.project.name.clone(),
            check_id,
            check_name,
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
}
