use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

use anyhow::{ensure, Context, Result};
use chrono::{SecondsFormat, Utc};
use clap::Parser;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
    name = "moonlight-runtime-profile",
    version,
    about = "Evaluate immutable runtime-profiler reference/candidate bundles with Moonlight policy"
)]
struct Args {
    #[arg(long, value_name = "PATH", default_value = "runtime-profiler")]
    runtime_profiler: PathBuf,

    #[arg(long, value_name = "PATH")]
    reference: PathBuf,

    #[arg(long, value_name = "PATH")]
    candidate: PathBuf,

    #[arg(long, value_name = "URI")]
    reference_uri: String,

    #[arg(long, value_name = "URI")]
    candidate_uri: String,

    #[arg(long, value_name = "SCORE", default_value_t = 90, value_parser = clap::value_parser!(u8).range(0..=100))]
    minimum_score: u8,

    #[arg(long, value_name = "COUNT", default_value_t = 5)]
    minimum_samples: usize,
}

#[derive(Debug, Deserialize)]
struct ValidationReport {
    bundle_id: String,
    valid: bool,
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeScore {
    schema_version: String,
    scenario_id: String,
    reference_bundle_id: String,
    candidate_bundle_id: String,
    score: u8,
    rating: String,
}

#[derive(Debug, Deserialize)]
struct MetricsDocument {
    metrics: Vec<MetricSummary>,
}

#[derive(Debug, Deserialize)]
struct MetricSummary {
    id: String,
    statistics: Statistics,
}

#[derive(Debug, Deserialize)]
struct Statistics {
    sample_count: usize,
    mean: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceReference {
    schema_version: u32,
    kind: String,
    uri: String,
    digest: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<serde_json::Number>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluationResult {
    schema_version: u32,
    evaluation_id: String,
    evaluator: EvaluatorIdentity,
    baseline: EvaluationIdentity,
    candidate: EvaluationIdentity,
    outcome: String,
    summary: String,
    metrics: BTreeMap<String, serde_json::Value>,
    started_at: String,
    finished_at: String,
    evidence: Vec<EvidenceReference>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluatorIdentity {
    component: String,
    version: String,
    protocol: String,
    configuration_digest: String,
}

#[derive(Debug, Serialize)]
struct EvaluationIdentity {
    kind: String,
    identity: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyDigestInput {
    minimum_score: u8,
    minimum_samples: usize,
    require_full_candidate_success: bool,
}

struct PolicyDecision {
    outcome: &'static str,
    summary: String,
    minimum_reference_samples: Option<usize>,
    minimum_candidate_samples: Option<usize>,
    candidate_success_rate: Option<f64>,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    ensure!(args.minimum_samples > 0, "minimum samples must be positive");
    let started_at = timestamp();

    let reference_validation =
        validate_bundle(&args.runtime_profiler, &args.reference, "reference")?;
    let candidate_validation =
        validate_bundle(&args.runtime_profiler, &args.candidate, "candidate")?;

    let reference_evidence = evidence_reference(
        &args.runtime_profiler,
        &args.reference,
        &args.reference_uri,
        "reference",
    )?;
    let candidate_evidence = evidence_reference(
        &args.runtime_profiler,
        &args.candidate,
        &args.candidate_uri,
        "candidate",
    )?;

    let score_output = profiler_output(
        &args.runtime_profiler,
        &[
            "score".to_owned(),
            "--reference".to_owned(),
            path_arg(&args.reference),
            "--candidate".to_owned(),
            path_arg(&args.candidate),
            "--json".to_owned(),
        ],
    )?;

    let (score, decision) = if score_output.status.success() {
        let score: RuntimeScore = parse_stdout(&score_output, "runtime-profiler score")?;
        ensure!(
            score.schema_version == "runtime-profiler/score/v1",
            "unsupported runtime-profiler score schema: {}",
            score.schema_version
        );
        ensure!(
            score.reference_bundle_id == reference_validation.bundle_id,
            "runtime-profiler score reference bundle identity changed during evaluation"
        );
        ensure!(
            score.candidate_bundle_id == candidate_validation.bundle_id,
            "runtime-profiler score candidate bundle identity changed during evaluation"
        );

        let reference_metrics = summarize(&args.runtime_profiler, &args.reference, "reference")?;
        let candidate_metrics = summarize(&args.runtime_profiler, &args.candidate, "candidate")?;
        let decision = evaluate_policy(
            &score,
            &reference_metrics,
            &candidate_metrics,
            args.minimum_score,
            args.minimum_samples,
        );
        (Some(score), decision)
    } else {
        let reason = bounded_stderr(&score_output);
        (
            None,
            PolicyDecision {
                outcome: "inconclusive",
                summary: format!(
                    "Runtime evidence is valid but not strictly comparable under runtime-profiler: {reason}"
                ),
                minimum_reference_samples: None,
                minimum_candidate_samples: None,
                candidate_success_rate: None,
            },
        )
    };

    let mut metrics = BTreeMap::new();
    metrics.insert(
        "minimumScore".to_owned(),
        serde_json::Value::from(args.minimum_score),
    );
    metrics.insert(
        "minimumSamples".to_owned(),
        serde_json::Value::from(args.minimum_samples),
    );
    if let Some(score) = &score {
        metrics.insert(
            "runtimeScore".to_owned(),
            serde_json::Value::from(score.score),
        );
        metrics.insert(
            "runtimeRating".to_owned(),
            serde_json::Value::from(score.rating.clone()),
        );
        metrics.insert(
            "scenarioId".to_owned(),
            serde_json::Value::from(score.scenario_id.clone()),
        );
    }
    if let Some(value) = decision.minimum_reference_samples {
        metrics.insert(
            "referenceMinimumSampleCount".to_owned(),
            serde_json::Value::from(value),
        );
    }
    if let Some(value) = decision.minimum_candidate_samples {
        metrics.insert(
            "candidateMinimumSampleCount".to_owned(),
            serde_json::Value::from(value),
        );
    }
    if let Some(value) = decision.candidate_success_rate {
        metrics.insert(
            "candidateSuccessRate".to_owned(),
            serde_json::Value::from(value),
        );
    }

    let result = EvaluationResult {
        schema_version: 1,
        evaluation_id: Uuid::new_v4().to_string(),
        evaluator: EvaluatorIdentity {
            component: "moonlight".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: "runtime-profiler-bundle/v1".to_owned(),
            configuration_digest: policy_digest(args.minimum_score, args.minimum_samples)?,
        },
        baseline: EvaluationIdentity {
            kind: "evidence-bundle".to_owned(),
            identity: reference_validation.bundle_id,
        },
        candidate: EvaluationIdentity {
            kind: "evidence-bundle".to_owned(),
            identity: candidate_validation.bundle_id,
        },
        outcome: decision.outcome.to_owned(),
        summary: decision.summary,
        metrics,
        started_at,
        finished_at: timestamp(),
        evidence: vec![reference_evidence, candidate_evidence],
    };

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(match result.outcome.as_str() {
        "passed" => ExitCode::SUCCESS,
        "failed" => ExitCode::from(2),
        "inconclusive" => ExitCode::from(3),
        _ => ExitCode::from(1),
    })
}

fn validate_bundle(bin: &Path, bundle: &Path, label: &str) -> Result<ValidationReport> {
    let output = profiler_output(
        bin,
        &[
            "validate".to_owned(),
            "--bundle".to_owned(),
            path_arg(bundle),
        ],
    )?;
    let report: ValidationReport = parse_stdout(&output, &format!("{label} bundle validation"))?;
    ensure!(
        output.status.success() && report.valid,
        "{label} runtime-profiler bundle is invalid: {}",
        report.diagnostics.join("; ")
    );
    Ok(report)
}

fn evidence_reference(
    bin: &Path,
    bundle: &Path,
    uri: &str,
    label: &str,
) -> Result<EvidenceReference> {
    let output = profiler_output(
        bin,
        &[
            "evidence-reference".to_owned(),
            "--bundle".to_owned(),
            path_arg(bundle),
            "--uri".to_owned(),
            uri.to_owned(),
        ],
    )?;
    ensure!(
        output.status.success(),
        "failed to create {label} evidence reference: {}",
        bounded_stderr(&output)
    );
    let evidence: EvidenceReference =
        parse_stdout(&output, &format!("{label} evidence reference"))?;
    ensure!(
        evidence.schema_version == 1,
        "unsupported agent evidence schema version"
    );
    ensure!(
        evidence.kind == "runtime-profile-bundle",
        "unexpected runtime evidence kind"
    );
    Ok(evidence)
}

fn summarize(bin: &Path, bundle: &Path, label: &str) -> Result<MetricsDocument> {
    let output = profiler_output(
        bin,
        &[
            "summarize".to_owned(),
            "--bundle".to_owned(),
            path_arg(bundle),
            "--json".to_owned(),
        ],
    )?;
    ensure!(
        output.status.success(),
        "failed to summarize {label} runtime bundle: {}",
        bounded_stderr(&output)
    );
    parse_stdout(&output, &format!("{label} runtime summary"))
}

fn evaluate_policy(
    score: &RuntimeScore,
    reference: &MetricsDocument,
    candidate: &MetricsDocument,
    minimum_score: u8,
    minimum_samples: usize,
) -> PolicyDecision {
    let reference_min = minimum_sample_count(reference);
    let candidate_min = minimum_sample_count(candidate);
    let candidate_success_rate = metric_mean(candidate, "process.success_rate");

    if reference_min.is_none_or(|value| value < minimum_samples)
        || candidate_min.is_none_or(|value| value < minimum_samples)
    {
        return PolicyDecision {
            outcome: "inconclusive",
            summary: format!(
                "Runtime evidence is comparable but does not satisfy the Moonlight sample policy of at least {minimum_samples} samples per metric."
            ),
            minimum_reference_samples: reference_min,
            minimum_candidate_samples: candidate_min,
            candidate_success_rate,
        };
    }

    if candidate_success_rate.is_none_or(|value| value < 1.0) {
        return PolicyDecision {
            outcome: "failed",
            summary: "Candidate runtime workload did not complete successfully in every measured iteration."
                .to_owned(),
            minimum_reference_samples: reference_min,
            minimum_candidate_samples: candidate_min,
            candidate_success_rate,
        };
    }

    if score.score < minimum_score {
        return PolicyDecision {
            outcome: "failed",
            summary: format!(
                "Candidate runtime score {} is below the configured Moonlight minimum of {minimum_score}.",
                score.score
            ),
            minimum_reference_samples: reference_min,
            minimum_candidate_samples: candidate_min,
            candidate_success_rate,
        };
    }

    PolicyDecision {
        outcome: "passed",
        summary: format!(
            "Candidate runtime score {} satisfies the Moonlight minimum of {minimum_score} with complete measured execution.",
            score.score
        ),
        minimum_reference_samples: reference_min,
        minimum_candidate_samples: candidate_min,
        candidate_success_rate,
    }
}

fn minimum_sample_count(metrics: &MetricsDocument) -> Option<usize> {
    metrics
        .metrics
        .iter()
        .map(|metric| metric.statistics.sample_count)
        .min()
}

fn metric_mean(metrics: &MetricsDocument, id: &str) -> Option<f64> {
    metrics
        .metrics
        .iter()
        .find(|metric| metric.id == id)
        .map(|metric| metric.statistics.mean)
}

fn policy_digest(minimum_score: u8, minimum_samples: usize) -> Result<String> {
    let input = PolicyDigestInput {
        minimum_score,
        minimum_samples,
        require_full_candidate_success: true,
    };
    let encoded = serde_json::to_vec(&input)?;
    let digest = Sha256::digest(encoded);
    Ok(format!("sha256:{}", hex::encode(digest)))
}

fn profiler_output(bin: &Path, args: &[String]) -> Result<Output> {
    Command::new(bin)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute runtime-profiler at {}", bin.display()))
}

fn parse_stdout<T: DeserializeOwned>(output: &Output, label: &str) -> Result<T> {
    serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "{label} did not emit valid JSON (stderr: {})",
            bounded_stderr(output)
        )
    })
}

fn bounded_stderr(output: &Output) -> String {
    const LIMIT: usize = 1_024;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let trimmed = stderr.trim();
    if trimmed.len() <= LIMIT {
        return if trimmed.is_empty() {
            "no diagnostic was emitted".to_owned()
        } else {
            trimmed.to_owned()
        };
    }
    let mut boundary = LIMIT;
    while !trimmed.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}…", &trimmed[..boundary])
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(id: &str, sample_count: usize, mean: f64) -> MetricSummary {
        MetricSummary {
            id: id.to_owned(),
            statistics: Statistics { sample_count, mean },
        }
    }

    fn score(value: u8) -> RuntimeScore {
        RuntimeScore {
            schema_version: "runtime-profiler/score/v1".to_owned(),
            scenario_id: "scenario".to_owned(),
            reference_bundle_id: "reference".to_owned(),
            candidate_bundle_id: "candidate".to_owned(),
            score: value,
            rating: "good".to_owned(),
        }
    }

    #[test]
    fn passes_only_complete_adequately_sampled_candidate() {
        let reference = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 5, 10.0),
                metric("process.success_rate", 5, 1.0),
            ],
        };
        let candidate = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 5, 9.0),
                metric("process.success_rate", 5, 1.0),
            ],
        };

        let decision = evaluate_policy(&score(95), &reference, &candidate, 90, 5);
        assert_eq!(decision.outcome, "passed");
    }

    #[test]
    fn inadequate_samples_are_inconclusive() {
        let reference = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 4, 10.0),
                metric("process.success_rate", 4, 1.0),
            ],
        };
        let candidate = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 4, 9.0),
                metric("process.success_rate", 4, 1.0),
            ],
        };

        let decision = evaluate_policy(&score(100), &reference, &candidate, 90, 5);
        assert_eq!(decision.outcome, "inconclusive");
    }

    #[test]
    fn candidate_failure_is_blocking_even_with_high_score() {
        let reference = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 5, 10.0),
                metric("process.success_rate", 5, 1.0),
            ],
        };
        let candidate = MetricsDocument {
            metrics: vec![
                metric("process.wall_time", 5, 8.0),
                metric("process.success_rate", 5, 0.8),
            ],
        };

        let decision = evaluate_policy(&score(99), &reference, &candidate, 90, 5);
        assert_eq!(decision.outcome, "failed");
    }
}
