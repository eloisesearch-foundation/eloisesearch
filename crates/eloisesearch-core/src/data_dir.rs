//! The data directory: node metadata, exclusive ownership and index lifecycle.
//!
//! Layout and crash-safety rules are specified in `docs/design-persistence.md`
//! ("On-disk layout" and "Index lifecycle").

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs4::{FileExt, TryLockError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::fsio::{sync_dir, write_atomic};
use crate::{Error, Result};

/// Highest on-disk format version this build reads and writes.
pub const FORMAT_VERSION: u32 = 1;

const NODE_FILE: &str = "node.json";
const LOCK_FILE: &str = "node.lock";
const INDEXES_DIR: &str = "indexes";

/// Identity of this node, persisted in `node.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// On-disk format version.
    pub format_version: u32,
    /// Random identifier generated when the data directory was created.
    pub node_id: String,
    /// Creation time, seconds since the Unix epoch.
    pub created_at_unix: u64,
}

/// An open data directory, owned exclusively by this process.
///
/// Holding a `DataDir` keeps an exclusive lock on `node.lock`; a second process
/// (or a second `DataDir` in the same process) opening the same directory fails
/// with [`Error::DataDirLocked`]. The lock is released on drop.
#[derive(Debug)]
pub struct DataDir {
    root: PathBuf,
    node: NodeMetadata,
    _lock: File,
}

impl DataDir {
    /// Opens (or initializes) the data directory at `root`.
    ///
    /// Creates `root`, `node.json` and `indexes/` when missing, takes the
    /// exclusive lock, checks the format version and removes leftovers of index
    /// creations or deletions interrupted by a crash.
    ///
    /// # Errors
    ///
    /// [`Error::DataDirLocked`] when another owner holds the directory,
    /// [`Error::UnsupportedFormat`] or [`Error::CorruptMetadata`] for an unusable
    /// `node.json`, [`Error::Io`] for file system failures.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(Error::io(format!("creating {}", root.display())))?;

        let lock = acquire_lock(&root)?;
        let node = load_or_create_node(&root)?;

        let indexes = root.join(INDEXES_DIR);
        if !indexes.is_dir() {
            fs::create_dir(&indexes)
                .map_err(Error::io(format!("creating {}", indexes.display())))?;
            sync_dir(&root).map_err(Error::io(format!("syncing {}", root.display())))?;
        }

        let data_dir = Self {
            root,
            node,
            _lock: lock,
        };
        data_dir.remove_interrupted_operations()?;
        Ok(data_dir)
    }

    /// The data directory path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// This node's persisted identity.
    pub fn node(&self) -> &NodeMetadata {
        &self.node
    }

    fn indexes_dir(&self) -> PathBuf {
        self.root.join(INDEXES_DIR)
    }

    /// Removes `.creating-*` and `.deleting-*` directories left by a crash.
    fn remove_interrupted_operations(&self) -> Result<()> {
        let indexes = self.indexes_dir();
        let entries =
            fs::read_dir(&indexes).map_err(Error::io(format!("listing {}", indexes.display())))?;
        let mut removed = false;
        for entry in entries {
            let entry = entry.map_err(Error::io(format!("listing {}", indexes.display())))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(CREATING_PREFIX) || name.starts_with(DELETING_PREFIX) {
                let path = entry.path();
                fs::remove_dir_all(&path)
                    .map_err(Error::io(format!("removing {}", path.display())))?;
                removed = true;
            }
        }
        if removed {
            sync_dir(&indexes).map_err(Error::io(format!("syncing {}", indexes.display())))?;
        }
        Ok(())
    }
}

pub(crate) const CREATING_PREFIX: &str = ".creating-";
pub(crate) const DELETING_PREFIX: &str = ".deleting-";

fn acquire_lock(root: &Path) -> Result<File> {
    let path = root.join(LOCK_FILE);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(Error::io(format!("opening {}", path.display())))?;
    // Fully qualified: `File::try_lock` also exists in newer standard libraries.
    match FileExt::try_lock(&file) {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(Error::DataDirLocked {
            path: root.to_path_buf(),
        }),
        Err(TryLockError::Error(source)) => Err(Error::Io {
            context: format!("locking {}", path.display()),
            source,
        }),
    }
}

fn load_or_create_node(root: &Path) -> Result<NodeMetadata> {
    let path = root.join(NODE_FILE);
    if path.exists() {
        let node: NodeMetadata = read_json(&path)?;
        check_format(&path, node.format_version)?;
        return Ok(node);
    }
    let node = NodeMetadata {
        format_version: FORMAT_VERSION,
        node_id: random_id(),
        created_at_unix: now_unix(),
    };
    write_json(&path, &node)?;
    Ok(node)
}

pub(crate) fn check_format(path: &Path, found: u32) -> Result<()> {
    if found > FORMAT_VERSION {
        return Err(Error::UnsupportedFormat {
            path: path.to_path_buf(),
            found,
            supported: FORMAT_VERSION,
        });
    }
    Ok(())
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).map_err(Error::io(format!("reading {}", path.display())))?;
    serde_json::from_slice(&bytes).map_err(|err| Error::CorruptMetadata {
        path: path.to_path_buf(),
        reason: err.to_string(),
    })
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|err| Error::InvalidArgument(err.to_string()))?;
    bytes.push(b'\n');
    write_atomic(path, &bytes).map_err(Error::io(format!("writing {}", path.display())))
}

/// 128 random bits as 32 lowercase hex characters.
pub(crate) fn random_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_initializes_an_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("data");

        let dir = DataDir::open(&root).unwrap();
        assert!(root.join("node.json").is_file());
        assert!(root.join("indexes").is_dir());
        assert_eq!(dir.node().format_version, FORMAT_VERSION);
        assert_eq!(dir.node().node_id.len(), 32);
    }

    #[test]
    fn node_identity_survives_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let first = DataDir::open(tmp.path()).unwrap().node().clone();
        let second = DataDir::open(tmp.path()).unwrap().node().clone();
        assert_eq!(first, second);
    }

    #[test]
    fn a_second_owner_is_rejected_until_the_first_is_dropped() {
        let tmp = tempfile::tempdir().unwrap();
        let first = DataDir::open(tmp.path()).unwrap();
        assert!(matches!(
            DataDir::open(tmp.path()),
            Err(Error::DataDirLocked { .. })
        ));
        drop(first);
        DataDir::open(tmp.path()).unwrap();
    }

    #[test]
    fn newer_format_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("node.json"),
            r#"{"format_version": 99, "node_id": "x", "created_at_unix": 0}"#,
        )
        .unwrap();
        assert!(matches!(
            DataDir::open(tmp.path()),
            Err(Error::UnsupportedFormat {
                found: 99,
                supported: FORMAT_VERSION,
                ..
            })
        ));
    }

    #[test]
    fn corrupt_node_metadata_is_reported_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("node.json"), b"{ not json").unwrap();
        assert!(matches!(
            DataDir::open(tmp.path()),
            Err(Error::CorruptMetadata { .. })
        ));
        assert_eq!(
            fs::read(tmp.path().join("node.json")).unwrap(),
            b"{ not json"
        );
    }

    #[test]
    fn interrupted_operations_are_cleaned_up_on_open() {
        let tmp = tempfile::tempdir().unwrap();
        drop(DataDir::open(tmp.path()).unwrap());
        let indexes = tmp.path().join("indexes");
        fs::create_dir_all(indexes.join(".creating-abc/tantivy")).unwrap();
        fs::create_dir_all(indexes.join(".deleting-def/wal")).unwrap();
        fs::create_dir(indexes.join("movies")).unwrap();

        DataDir::open(tmp.path()).unwrap();
        let mut left: Vec<_> = fs::read_dir(&indexes)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, ["movies"]);
    }
}
