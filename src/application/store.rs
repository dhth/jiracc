use crate::domain::Snapshot;
use std::error::Error;

pub trait SnapshotStore {
    type Error: Error + Send + Sync + 'static;

    fn save_snapshot(&self, snapshot: &Snapshot) -> Result<(), Self::Error>;
}
