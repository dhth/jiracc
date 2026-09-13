use super::Issue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub metadata: SnapshotMetadata,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub fetched_at: DateTime<Utc>,
}
