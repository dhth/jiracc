use crate::application::SnapshotStore;
use crate::config::CanonicalConfigPath;
use crate::domain::Snapshot;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

const STORAGE_VERSION: &str = "v1";
const SNAPSHOT_FILE: &str = "snapshot.json";

pub struct FileSnapshotStore {
    namespace_directory: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum FileSnapshotStoreError {
    #[error("couldn't serialize snapshot")]
    Serialize(#[source] serde_json::Error),

    #[error("couldn't create snapshot directory at {path:?}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't create a temporary snapshot in {path:?}")]
    CreateTemporaryFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't write temporary snapshot in {path:?}")]
    WriteTemporaryFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("couldn't replace snapshot at {path:?}")]
    ReplaceSnapshot {
        path: PathBuf,
        #[source]
        source: tempfile::PersistError,
    },
}

impl FileSnapshotStore {
    pub fn new(data_directory: &Path, config_path: &CanonicalConfigPath) -> Self {
        let namespace = namespace_name(config_path.as_str());
        let namespace_directory = data_directory
            .join("snapshots")
            .join(STORAGE_VERSION)
            .join(namespace);

        Self {
            namespace_directory,
        }
    }
}

impl SnapshotStore for FileSnapshotStore {
    type Error = FileSnapshotStoreError;

    fn save_snapshot(&self, snapshot: &Snapshot) -> Result<(), Self::Error> {
        let serialized_snapshot =
            serde_json::to_vec_pretty(snapshot).map_err(FileSnapshotStoreError::Serialize)?;

        save_snapshot(&self.namespace_directory, &serialized_snapshot)
    }
}

fn save_snapshot(
    namespace_directory: &Path,
    serialized_snapshot: &[u8],
) -> Result<(), FileSnapshotStoreError> {
    std::fs::create_dir_all(namespace_directory).map_err(|source| {
        FileSnapshotStoreError::CreateDirectory {
            path: namespace_directory.to_path_buf(),
            source,
        }
    })?;

    let mut temporary_file =
        tempfile::NamedTempFile::new_in(namespace_directory).map_err(|source| {
            FileSnapshotStoreError::CreateTemporaryFile {
                path: namespace_directory.to_path_buf(),
                source,
            }
        })?;
    temporary_file
        .write_all(serialized_snapshot)
        .map_err(|source| FileSnapshotStoreError::WriteTemporaryFile {
            path: namespace_directory.to_path_buf(),
            source,
        })?;

    let snapshot_path = namespace_directory.join(SNAPSHOT_FILE);
    temporary_file.persist(&snapshot_path).map_err(|source| {
        FileSnapshotStoreError::ReplaceSnapshot {
            path: snapshot_path,
            source,
        }
    })?;

    Ok(())
}

fn namespace_name(config_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config_path.as_bytes());
    format!("{:x}", hasher.finalize())
}
