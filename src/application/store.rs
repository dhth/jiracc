use crate::domain::{Issue, Snapshot};
use std::error::Error;

pub trait SnapshotStore {
    type Error: Error + Send + Sync + 'static;

    fn save_snapshot(&self, snapshot: &Snapshot) -> Result<(), Self::Error>;

    fn get_snapshot(&self) -> Result<Snapshot, Self::Error>;

    fn get_issue(&self, key: &str) -> Result<Option<Issue>, Self::Error>;
}
