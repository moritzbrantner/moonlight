use moonlight_core::{
    config::{AppConfig, ResponseTiming, ReturnFallback, ReturnTarget},
    review::{ReviewStatus, ReviewUpdate, RunReviewState},
    Adapter, BodyCapture, Classification, ComparisonRun, ComparisonRunListItem, ComparisonSummary,
    DiffEntry, DiffKind, LatencyStats, MetricsClassificationCounts, MetricsSnapshot, RunInput,
    RunPage, StatsSummary, TargetObservation,
};
use ts_rs::{Config, TS};

fn main() {
    println!(
        "// Generated from Moonlight Rust API models. Keep UI-only view types outside this file."
    );
    println!();
    export::<Classification>();
    export::<DiffKind>();
    export::<Adapter>();
    export::<ReviewStatus>();
    export::<ReturnTarget>();
    export::<ReturnFallback>();
    export::<ResponseTiming>();
    export::<BodyCapture>();
    export::<TargetObservation>();
    export::<RunInput>();
    export::<DiffEntry>();
    export::<ComparisonSummary>();
    export::<ComparisonRunListItem>();
    export::<ComparisonRun>();
    export::<RunPage>();
    export::<RunReviewState>();
    export::<ReviewUpdate>();
    export::<LatencyStats>();
    export::<StatsSummary>();
    export::<AppConfig>();
    export::<MetricsClassificationCounts>();
    export::<MetricsSnapshot>();
}

fn export<T: TS>() {
    let config = Config::new().with_large_int("number");
    println!("export {}", T::decl(&config));
    println!();
}
