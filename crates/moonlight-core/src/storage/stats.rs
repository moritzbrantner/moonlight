use crate::{Classification, ComparisonRun, ComparisonRunListItem, LatencyStats, StatsSummary};
use std::collections::VecDeque;

#[derive(Default)]
pub(super) struct StatsAccumulator {
    total_runs: usize,
    matches: usize,
    suspicious_differences: usize,
    reference_noise: usize,
    suspicious_with_noise: usize,
    target_errors: usize,
    primary_total: u128,
    candidate_total: u128,
    secondary_total: u128,
    secondary_count: usize,
    latest_runs: VecDeque<ComparisonRunListItem>,
}

impl StatsAccumulator {
    pub(super) fn record(&mut self, run: &ComparisonRun) {
        self.total_runs += 1;
        match run.comparison.classification {
            Classification::Match => self.matches += 1,
            Classification::SuspiciousDifference => self.suspicious_differences += 1,
            Classification::ReferenceNoise => self.reference_noise += 1,
            Classification::SuspiciousWithNoise => self.suspicious_with_noise += 1,
            Classification::TargetError => self.target_errors += 1,
        }
        self.primary_total += run.primary.latency_ms;
        self.candidate_total += run.candidate.latency_ms;
        if let Some(secondary) = &run.secondary {
            self.secondary_total += secondary.latency_ms;
            self.secondary_count += 1;
        }

        self.latest_runs.push_back(ComparisonRunListItem::from(run));
        if self.latest_runs.len() > 20 {
            self.latest_runs.pop_front();
        }
    }

    pub(super) fn finish(self) -> StatsSummary {
        StatsSummary {
            total_runs: self.total_runs,
            matches: self.matches,
            suspicious_differences: self.suspicious_differences,
            reference_noise: self.reference_noise,
            suspicious_with_noise: self.suspicious_with_noise,
            target_errors: self.target_errors,
            latency: LatencyStats {
                primary_avg_ms: avg(self.total_runs, self.primary_total),
                candidate_avg_ms: avg(self.total_runs, self.candidate_total),
                secondary_avg_ms: avg_opt(self.secondary_count, self.secondary_total),
            },
            latest_runs: self.latest_runs.into_iter().rev().collect(),
        }
    }
}

fn avg(count: usize, total: u128) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn avg_opt(count: usize, total: u128) -> Option<f64> {
    if count == 0 {
        None
    } else {
        Some(total as f64 / count as f64)
    }
}
