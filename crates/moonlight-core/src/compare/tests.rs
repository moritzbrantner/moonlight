use super::*;
use crate::{Classification, DiffKind, TargetObservation};
use bytes::Bytes;
use std::collections::BTreeMap;

fn config() -> CompareConfig {
    CompareConfig::new(
        &[
            "$.timestamp".into(),
            "$.requestId".into(),
            "$.traceId".into(),
            "$.id".into(),
        ],
        &[
            "date".into(),
            "server".into(),
            "set-cookie".into(),
            "x-request-id".into(),
            "traceparent".into(),
        ],
        false,
    )
}

fn target(status: u16, headers: &[(&str, &str)], body: &str) -> CapturedTarget {
    target_with_stderr(status, headers, body, "")
}

fn target_with_stderr(
    status: u16,
    headers: &[(&str, &str)],
    body: &str,
    stderr: &str,
) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status: Some(status),
            headers: headers
                .iter()
                .map(|(key, value)| (key.to_ascii_lowercase(), value.to_string()))
                .collect(),
            body: capture_body(body.as_bytes(), 1024),
            stderr: Some(capture_body(stderr.as_bytes(), 1024)),
            latency_ms: 1,
            error: None,
        },
        body_bytes: Bytes::copy_from_slice(body.as_bytes()),
        stderr_bytes: Bytes::copy_from_slice(stderr.as_bytes()),
    }
}

fn target_error(message: &str) -> CapturedTarget {
    CapturedTarget {
        observation: TargetObservation {
            status: None,
            headers: BTreeMap::new(),
            body: capture_body(&[], 1024),
            stderr: None,
            latency_ms: 1,
            error: Some(message.to_string()),
        },
        body_bytes: Bytes::new(),
        stderr_bytes: Bytes::new(),
    }
}

#[test]
fn identical_targets_match() {
    let primary = target(
        200,
        &[("content-type", "application/json")],
        r#"{"ok":true}"#,
    );
    let candidate = target(
        200,
        &[("content-type", "application/json")],
        r#"{"ok":true}"#,
    );
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::Match);
    assert!(result.raw_candidate_diffs.is_empty());
}

#[test]
fn identical_non_json_bodies_match() {
    let primary = target(200, &[], "plain text");
    let candidate = target(200, &[], "plain text");
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::Match);
    assert!(result.raw_candidate_diffs.is_empty());
}

#[test]
fn candidate_difference_without_secondary_is_suspicious() {
    let primary = target(200, &[], r#"{"ok":true,"value":1}"#);
    let candidate = target(200, &[], r#"{"ok":true,"value":2}"#);
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::SuspiciousDifference);
    assert_eq!(result.noise_filtered_diffs[0].path, "$.value");
}

#[test]
fn candidate_matching_primary_on_noisy_path_is_reference_noise() {
    let primary = target(200, &[], r#"{"region":"a","value":1}"#);
    let candidate = target(200, &[], r#"{"region":"a","value":1}"#);
    let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
    let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
    assert_eq!(result.classification, Classification::ReferenceNoise);
    assert_eq!(result.reference_noise[0].path, "$.region");
}

#[test]
fn candidate_matching_secondary_on_noisy_path_is_reference_noise() {
    let primary = target(200, &[], r#"{"region":"a","value":1}"#);
    let candidate = target(200, &[], r#"{"region":"b","value":1}"#);
    let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
    let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
    assert_eq!(result.classification, Classification::ReferenceNoise);
    assert!(result.noise_filtered_diffs.is_empty());
}

#[test]
fn candidate_different_from_both_references_is_suspicious_with_noise() {
    let primary = target(200, &[], r#"{"region":"a","value":1}"#);
    let candidate = target(200, &[], r#"{"region":"c","value":1}"#);
    let secondary = target(200, &[], r#"{"region":"b","value":1}"#);
    let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
    assert_eq!(result.classification, Classification::SuspiciousWithNoise);
    assert_eq!(result.noise_filtered_diffs[0].path, "$.region");
}

#[test]
fn status_noise_uses_candidate_must_match_reference_rule() {
    let primary = target(200, &[], r#"{"ok":true}"#);
    let candidate = target(500, &[], r#"{"ok":true}"#);
    let secondary = target(404, &[], r#"{"ok":true}"#);
    let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
    assert_eq!(result.classification, Classification::SuspiciousWithNoise);
    assert_eq!(result.noise_filtered_diffs[0].kind, DiffKind::Status);
}

#[test]
fn header_noise_filters_when_candidate_matches_secondary() {
    let primary = target(200, &[("x-region", "a")], "ok");
    let candidate = target(200, &[("x-region", "b")], "ok");
    let secondary = target(200, &[("x-region", "b")], "ok");
    let result = compare_targets(&primary, &candidate, Some(&secondary), &config());
    assert_eq!(result.classification, Classification::ReferenceNoise);
}

#[test]
fn stderr_differences_are_compared_by_default() {
    let primary = target_with_stderr(0, &[], "ok", "primary");
    let candidate = target_with_stderr(0, &[], "ok", "candidate");
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::SuspiciousDifference);
    assert_eq!(result.noise_filtered_diffs[0].kind, DiffKind::Stderr);
}

#[test]
fn stderr_can_be_ignored() {
    let primary = target_with_stderr(0, &[], "ok", "primary");
    let candidate = target_with_stderr(0, &[], "ok", "candidate");
    let config = CompareConfig::new(&[], &[], true);
    let result = compare_targets(&primary, &candidate, None, &config);
    assert_eq!(result.classification, Classification::Match);
}

#[test]
fn target_errors_are_top_level_errors() {
    let primary = target(200, &[], "ok");
    let candidate = target_error("candidate failed");
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::TargetError);
}

#[test]
fn ignored_json_fields_do_not_diff() {
    let primary = target(200, &[], r#"{"id":"a","value":1}"#);
    let candidate = target(200, &[], r#"{"id":"b","value":1}"#);
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::Match);
}

#[test]
fn ignored_headers_do_not_diff() {
    let primary = target(200, &[("date", "one"), ("x-mode", "a")], "ok");
    let candidate = target(200, &[("date", "two"), ("x-mode", "a")], "ok");
    let result = compare_targets(&primary, &candidate, None, &config());
    assert_eq!(result.classification, Classification::Match);
}
