use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    New,
    Accepted,
    Ignored,
    Fixed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn review_store_persists_and_filters_review_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("review-state.json");
        let store = ReviewStore::load(path.clone()).await.unwrap();
        let run_id = Uuid::new_v4();

        let saved = store
            .put(
                run_id,
                ReviewUpdate {
                    status: ReviewStatus::Ignored,
                    note: Some("known noise".to_string()),
                    tags: Some(vec!["noise".to_string()]),
                },
            )
            .await
            .unwrap();

        assert_eq!(saved.status, ReviewStatus::Ignored);
        let reloaded = ReviewStore::load(path).await.unwrap();
        assert_eq!(
            reloaded.get(run_id).await.note.as_deref(),
            Some("known noise")
        );
        assert_eq!(reloaded.list(Some(ReviewStatus::Ignored)).await.len(), 1);
        assert!(reloaded.list(Some(ReviewStatus::Fixed)).await.is_empty());
    }
}

impl std::str::FromStr for ReviewStatus {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "new" => Ok(Self::New),
            "accepted" => Ok(Self::Accepted),
            "ignored" => Ok(Self::Ignored),
            "fixed" => Ok(Self::Fixed),
            other => anyhow::bail!("invalid review status {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct RunReviewState {
    pub run_id: Uuid,
    pub status: ReviewStatus,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl RunReviewState {
    pub fn new(run_id: Uuid) -> Self {
        Self {
            run_id,
            status: ReviewStatus::New,
            note: None,
            tags: Vec::new(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct ReviewUpdate {
    pub status: ReviewStatus,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub note: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub tags: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct ReviewStore {
    path: PathBuf,
    states: Arc<RwLock<BTreeMap<Uuid, RunReviewState>>>,
}

impl ReviewStore {
    pub async fn load(path: PathBuf) -> anyhow::Result<Self> {
        let states = if tokio::fs::try_exists(&path).await? {
            let content = tokio::fs::read_to_string(&path).await?;
            if content.trim().is_empty() {
                BTreeMap::new()
            } else {
                serde_json::from_str(&content)?
            }
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            path,
            states: Arc::new(RwLock::new(states)),
        })
    }

    pub async fn get(&self, run_id: Uuid) -> RunReviewState {
        self.states
            .read()
            .await
            .get(&run_id)
            .cloned()
            .unwrap_or_else(|| RunReviewState::new(run_id))
    }

    pub async fn list(&self, status: Option<ReviewStatus>) -> Vec<RunReviewState> {
        self.states
            .read()
            .await
            .values()
            .filter(|state| status.as_ref().is_none_or(|status| &state.status == status))
            .cloned()
            .collect()
    }

    pub async fn put(&self, run_id: Uuid, update: ReviewUpdate) -> anyhow::Result<RunReviewState> {
        let state = RunReviewState {
            run_id,
            status: update.status,
            note: update.note.filter(|note| !note.trim().is_empty()),
            tags: update
                .tags
                .unwrap_or_default()
                .into_iter()
                .filter(|tag| !tag.trim().is_empty())
                .collect(),
            updated_at: Utc::now(),
        };

        {
            let mut states = self.states.write().await;
            states.insert(run_id, state.clone());
            self.persist_locked(&states).await?;
        }

        Ok(state)
    }

    async fn persist_locked(&self, states: &BTreeMap<Uuid, RunReviewState>) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(states)?;
        tokio::fs::write(&self.path, format!("{content}\n")).await?;
        Ok(())
    }
}
