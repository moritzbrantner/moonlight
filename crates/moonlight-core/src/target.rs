use crate::TargetObservation;
use bytes::Bytes;

/// A transient target capture used while building and comparing a run.
///
/// `TargetObservation` is the persisted observation stored in a
/// `ComparisonRun`. `CapturedTarget` also carries raw body and stderr bytes so
/// comparison can inspect complete target output without widening the persisted
/// run shape.
#[derive(Debug, Clone)]
pub struct CapturedTarget {
    pub observation: TargetObservation,
    pub body_bytes: Bytes,
    pub stderr_bytes: Bytes,
}
