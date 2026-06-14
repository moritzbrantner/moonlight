use crate::{
    command::run_command,
    types::{CapturedTargets, Case, PreparedCase, TargetCommand},
};
use chrono::Utc;
use moonlight_core::{
    compare::{capture_body, compare_targets},
    Adapter, ComparisonRun, RunInput,
};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(crate) async fn execute_case(prepared: PreparedCase, serial_targets: bool) -> ComparisonRun {
    let PreparedCase {
        case,
        compare_config,
    } = prepared;
    let targets = run_targets(&case, serial_targets).await;
    let comparison = compare_targets(
        &targets.primary,
        &targets.candidate,
        targets.secondary.as_ref(),
        &compare_config,
    );

    ComparisonRun {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        adapter: Adapter::Cli,
        input: RunInput::Cli {
            primary_command: case.primary.display(),
            candidate_command: case.candidate.display(),
            secondary_command: case.secondary.as_ref().map(TargetCommand::display),
        },
        request_headers: BTreeMap::new(),
        request_body: capture_body(&[], case.max_body_capture_bytes),
        primary: targets.primary.observation,
        candidate: targets.candidate.observation,
        secondary: targets.secondary.map(|target| target.observation),
        comparison,
    }
}

async fn run_targets(case: &Case, serial_targets: bool) -> CapturedTargets {
    if serial_targets {
        let primary = run_command("primary", &case.primary, case.max_body_capture_bytes).await;
        let candidate =
            run_command("candidate", &case.candidate, case.max_body_capture_bytes).await;
        let secondary = match &case.secondary {
            Some(command) => {
                Some(run_command("secondary", command, case.max_body_capture_bytes).await)
            }
            None => None,
        };
        return CapturedTargets {
            primary,
            candidate,
            secondary,
        };
    }

    match &case.secondary {
        Some(secondary_command) => {
            let (primary, candidate, secondary) = tokio::join!(
                run_command("primary", &case.primary, case.max_body_capture_bytes),
                run_command("candidate", &case.candidate, case.max_body_capture_bytes),
                run_command("secondary", secondary_command, case.max_body_capture_bytes),
            );
            CapturedTargets {
                primary,
                candidate,
                secondary: Some(secondary),
            }
        }
        None => {
            let (primary, candidate) = tokio::join!(
                run_command("primary", &case.primary, case.max_body_capture_bytes),
                run_command("candidate", &case.candidate, case.max_body_capture_bytes),
            );
            CapturedTargets {
                primary,
                candidate,
                secondary: None,
            }
        }
    }
}
