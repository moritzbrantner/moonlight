use moonlight_core::{Classification, ComparisonRun, RunInput};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub(super) struct EvalSummary {
    pub(super) eval_id: Uuid,
    pub(super) project: String,
    pub(super) repo: String,
    pub(super) baseline_ref: String,
    pub(super) candidate_source: String,
    pub(super) total_checks: usize,
    pub(super) classifications: BTreeMap<String, usize>,
    pub(super) failed_checks: Vec<EvalFailedCheck>,
    pub(super) runs: Vec<EvalRunItem>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct EvalRunItem {
    pub(super) check_id: String,
    pub(super) check_name: Option<String>,
    pub(super) run_id: Uuid,
    pub(super) classification: Classification,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct EvalFailedCheck {
    pub(super) check_id: String,
    pub(super) check_name: Option<String>,
    pub(super) run_id: Uuid,
    pub(super) classification: Classification,
    pub(super) primary_status: Option<u16>,
    pub(super) candidate_status: Option<u16>,
    pub(super) diff_summary: String,
}

impl EvalSummary {
    pub(super) fn new(
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

    pub(super) fn from_runs(eval_id: Uuid, runs: &[ComparisonRun]) -> anyhow::Result<Self> {
        let Some(first) = runs.first() else {
            anyhow::bail!("eval has no runs");
        };
        let RunInput::Project {
            project,
            repo,
            baseline_ref,
            candidate_source,
            ..
        } = &first.input
        else {
            anyhow::bail!("stored eval run has non-project input");
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

    pub(super) fn record(&mut self, run: &ComparisonRun) {
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

pub(super) fn classification_key(classification: &Classification) -> String {
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
