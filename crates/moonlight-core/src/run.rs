use crate::{
    compare::{compare_targets, CompareConfig},
    target::CapturedTarget,
    Adapter, BodyCapture, ComparisonRun, RunInput,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Metadata supplied by an adapter when handing captured targets to core.
#[derive(Debug, Clone)]
pub struct RunMetadata {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub adapter: Adapter,
    pub input: RunInput,
    pub request_headers: BTreeMap<String, String>,
    pub request_body: BodyCapture,
}

/// Captured target results for the required and optional comparison roles.
#[derive(Debug, Clone)]
pub struct CapturedTargets {
    pub primary: CapturedTarget,
    pub candidate: CapturedTarget,
    pub secondary: Option<CapturedTarget>,
}

/// Build a persisted comparison run from adapter metadata and captured targets.
///
/// Adapters own target invocation and capture. Core owns the transition from
/// those captures into the stable `ComparisonRun` shape and its classification.
pub fn build_comparison_run(
    metadata: RunMetadata,
    targets: CapturedTargets,
    compare_config: &CompareConfig,
) -> ComparisonRun {
    let comparison = compare_targets(
        &targets.primary,
        &targets.candidate,
        targets.secondary.as_ref(),
        compare_config,
    );

    ComparisonRun {
        id: metadata.id,
        timestamp: metadata.timestamp,
        adapter: metadata.adapter,
        input: metadata.input,
        request_headers: metadata.request_headers,
        request_body: metadata.request_body,
        primary: targets.primary.observation,
        candidate: targets.candidate.observation,
        secondary: targets.secondary.map(|target| target.observation),
        comparison,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compare::capture_body, Classification, DiffKind, TargetObservation};
    use bytes::Bytes;
    use std::collections::BTreeMap;

    fn config() -> CompareConfig {
        CompareConfig::new(&[], &[], false)
    }

    fn metadata(adapter: Adapter, input: RunInput) -> RunMetadata {
        RunMetadata {
            id: Uuid::nil(),
            timestamp: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            adapter,
            input,
            request_headers: BTreeMap::new(),
            request_body: capture_body(&[], 1024),
        }
    }

    fn target(status: u16, body: &str) -> CapturedTarget {
        let bytes = Bytes::from(body.to_string());
        CapturedTarget {
            observation: TargetObservation {
                status: Some(status),
                headers: BTreeMap::new(),
                body: capture_body(&bytes, 1024),
                stderr: Some(capture_body(&[], 1024)),
                latency_ms: 7,
                error: None,
            },
            body_bytes: bytes,
            stderr_bytes: Bytes::new(),
        }
    }

    fn target_error(message: &str) -> CapturedTarget {
        CapturedTarget {
            observation: TargetObservation {
                status: None,
                headers: BTreeMap::new(),
                body: capture_body(&[], 1024),
                stderr: Some(capture_body(&[], 1024)),
                latency_ms: 0,
                error: Some(message.to_string()),
            },
            body_bytes: Bytes::new(),
            stderr_bytes: Bytes::new(),
        }
    }

    #[test]
    fn cli_metadata_builds_classified_run() {
        let run = build_comparison_run(
            metadata(
                Adapter::Cli,
                RunInput::Cli {
                    primary_command: "printf one".to_string(),
                    candidate_command: "printf two".to_string(),
                    secondary_command: None,
                },
            ),
            CapturedTargets {
                primary: target(0, "one"),
                candidate: target(0, "two"),
                secondary: None,
            },
            &config(),
        );

        assert_eq!(run.adapter, Adapter::Cli);
        assert!(matches!(run.input, RunInput::Cli { .. }));
        assert_eq!(
            run.comparison.classification,
            Classification::SuspiciousDifference
        );
    }

    #[test]
    fn http_metadata_preserves_request_fields() {
        let mut request_headers = BTreeMap::new();
        request_headers.insert("x-request".to_string(), "abc".to_string());
        let request_body = capture_body(b"{\"request\":true}", 1024);

        let run = build_comparison_run(
            RunMetadata {
                id: Uuid::nil(),
                timestamp: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
                adapter: Adapter::Http,
                input: RunInput::Http {
                    method: "POST".to_string(),
                    path: "/submit".to_string(),
                    query: Some("token=[redacted]".to_string()),
                },
                request_headers: request_headers.clone(),
                request_body: request_body.clone(),
            },
            CapturedTargets {
                primary: target(200, "{\"ok\":true}"),
                candidate: target(200, "{\"ok\":true}"),
                secondary: None,
            },
            &config(),
        );

        assert_eq!(run.request_headers, request_headers);
        assert_eq!(run.request_body.sha256, request_body.sha256);
        assert!(matches!(
            run.input,
            RunInput::Http {
                ref method,
                ref path,
                ref query
            } if method == "POST" && path == "/submit" && query.as_deref() == Some("token=[redacted]")
        ));
        assert_eq!(run.comparison.classification, Classification::Match);
    }

    #[test]
    fn project_metadata_builds_project_run() {
        let run = build_comparison_run(
            metadata(
                Adapter::Project,
                RunInput::Project {
                    eval_id: Uuid::nil(),
                    project: "moonlight".to_string(),
                    check_id: "test".to_string(),
                    check_name: Some("cargo test".to_string()),
                    repo: "/repo".to_string(),
                    baseline_ref: "main".to_string(),
                    candidate_source: "patch".to_string(),
                    primary_command: "cargo test".to_string(),
                    candidate_command: "cargo test".to_string(),
                    secondary_command: None,
                },
            ),
            CapturedTargets {
                primary: target(0, "ok"),
                candidate: target(0, "ok"),
                secondary: None,
            },
            &config(),
        );

        assert_eq!(run.adapter, Adapter::Project);
        assert!(matches!(run.input, RunInput::Project { .. }));
        assert_eq!(run.comparison.classification, Classification::Match);
    }

    #[test]
    fn secondary_reference_noise_filters_candidate_diff() {
        let run = build_comparison_run(
            metadata(
                Adapter::Cli,
                RunInput::Cli {
                    primary_command: "primary".to_string(),
                    candidate_command: "candidate".to_string(),
                    secondary_command: Some("secondary".to_string()),
                },
            ),
            CapturedTargets {
                primary: target(0, "stable"),
                candidate: target(0, "noisy"),
                secondary: Some(target(0, "noisy")),
            },
            &config(),
        );

        assert_eq!(
            run.comparison.classification,
            Classification::ReferenceNoise
        );
        assert!(run.comparison.noise_filtered_diffs.is_empty());
    }

    #[test]
    fn target_errors_are_classified_as_target_error() {
        let run = build_comparison_run(
            metadata(
                Adapter::Cli,
                RunInput::Cli {
                    primary_command: "primary".to_string(),
                    candidate_command: "candidate".to_string(),
                    secondary_command: None,
                },
            ),
            CapturedTargets {
                primary: target(0, "ok"),
                candidate: target_error("candidate failed"),
                secondary: None,
            },
            &config(),
        );

        assert_eq!(run.comparison.classification, Classification::TargetError);
    }

    #[test]
    fn compare_config_controls_diffing() {
        let compare_config = CompareConfig::new_with_patterns(
            &["$.ignored".to_string()],
            &[],
            &["$.secret".to_string()],
            &[],
            &[],
            false,
        );
        let run = build_comparison_run(
            metadata(
                Adapter::Cli,
                RunInput::Cli {
                    primary_command: "primary".to_string(),
                    candidate_command: "candidate".to_string(),
                    secondary_command: None,
                },
            ),
            CapturedTargets {
                primary: target(0, r#"{"ignored":1,"secret":"a","kept":1}"#),
                candidate: target(0, r#"{"ignored":2,"secret":"b","kept":2}"#),
                secondary: None,
            },
            &compare_config,
        );

        let paths = run
            .comparison
            .raw_candidate_diffs
            .iter()
            .map(|diff| (diff.kind.clone(), diff.path.as_str()))
            .collect::<Vec<_>>();
        assert!(paths.contains(&(DiffKind::Body, "$.secret")));
        assert!(paths.contains(&(DiffKind::Body, "$.kept")));
        assert!(!paths.iter().any(|(_, path)| *path == "$.ignored"));
    }
}
