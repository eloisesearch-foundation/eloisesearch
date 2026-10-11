//! Index lifecycle: create, inspect, list and delete indexes in a data directory.
//!
//! Creation and deletion are made atomic with directory renames, so a crash at
//! any point leaves either the old state or the new one (see
//! `docs/design-persistence.md`, "Index lifecycle"). Index operations are not
//! written to the WAL.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::data_dir::{
    CREATING_PREFIX, DELETING_PREFIX, check_format, now_unix, random_id, read_json, write_json,
};
use crate::fsio::sync_dir;
use crate::{DataDir, Error, FORMAT_VERSION, IndexName, Result};

const INDEX_FILE: &str = "index.json";
const TANTIVY_DIR: &str = "tantivy";
const WAL_DIR: &str = "wal";

/// Persisted description of an index (`index.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// On-disk format version.
    pub format_version: u32,
    /// Random identifier of this index instance; a re-created index gets a new one.
    pub uuid: String,
    /// Creation time, seconds since the Unix epoch.
    pub created_at_unix: u64,
    /// Settings as accepted at creation.
    pub settings: Value,
    /// Mappings as accepted at creation.
    pub mappings: Value,
}

impl DataDir {
    /// Creates an index with the given settings and mappings.
    ///
    /// `settings` and `mappings` must be JSON objects (`null` means empty).
    /// Validation against the supported subset is added with the schema; for now
    /// they are stored as given.
    ///
    /// # Errors
    ///
    /// [`Error::IndexAlreadyExists`], [`Error::InvalidArgument`] for non-object
    /// settings or mappings, [`Error::Io`] for file system failures.
    pub fn create_index(
        &self,
        name: &IndexName,
        settings: Value,
        mappings: Value,
    ) -> Result<IndexMetadata> {
        let settings = object_or_empty("settings", settings)?;
        let mappings = object_or_empty("mappings", mappings)?;
        let _guard = self.lock_lifecycle();

        let target = self.index_path(name);
        if target.exists() {
            return Err(Error::IndexAlreadyExists {
                name: name.to_string(),
            });
        }

        let metadata = IndexMetadata {
            format_version: FORMAT_VERSION,
            uuid: random_id(),
            created_at_unix: now_unix(),
            settings,
            mappings,
        };

        // Build the whole index under a staging name, then publish it with one rename.
        let staging = self
            .indexes_dir()
            .join(format!("{CREATING_PREFIX}{}", metadata.uuid));
        let io = |what: &str, path: &PathBuf| Error::io(format!("{what} {}", path.display()));
        fs::create_dir(&staging).map_err(io("creating", &staging))?;
        for sub in [TANTIVY_DIR, WAL_DIR] {
            let dir = staging.join(sub);
            fs::create_dir(&dir).map_err(io("creating", &dir))?;
        }
        write_json(&staging.join(INDEX_FILE), &metadata)?;
        sync_dir(&staging).map_err(io("syncing", &staging))?;

        fs::rename(&staging, &target).map_err(io("publishing", &target))?;
        let indexes = self.indexes_dir();
        sync_dir(&indexes).map_err(io("syncing", &indexes))?;
        Ok(metadata)
    }

    /// Reads the metadata of an existing index.
    ///
    /// # Errors
    ///
    /// [`Error::IndexNotFound`], [`Error::UnsupportedFormat`],
    /// [`Error::CorruptMetadata`] or [`Error::Io`].
    pub fn index_metadata(&self, name: &IndexName) -> Result<IndexMetadata> {
        let path = self.index_path(name).join(INDEX_FILE);
        if !path.is_file() {
            return Err(Error::IndexNotFound {
                name: name.to_string(),
            });
        }
        let metadata: IndexMetadata = read_json(&path)?;
        check_format(&path, metadata.format_version)?;
        Ok(metadata)
    }

    /// Whether an index with this name exists.
    pub fn index_exists(&self, name: &IndexName) -> bool {
        self.index_path(name).join(INDEX_FILE).is_file()
    }

    /// Names of all existing indexes, sorted.
    ///
    /// Directories that are not valid index names (including in-progress
    /// `.creating-*` and `.deleting-*` directories) or that lack `index.json`
    /// are ignored.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the indexes directory cannot be read.
    pub fn list_indexes(&self) -> Result<Vec<IndexName>> {
        let indexes = self.indexes_dir();
        let entries =
            fs::read_dir(&indexes).map_err(Error::io(format!("listing {}", indexes.display())))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(Error::io(format!("listing {}", indexes.display())))?;
            let Ok(name) = IndexName::new(entry.file_name().to_string_lossy()) else {
                continue;
            };
            if self.index_exists(&name) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    /// Deletes an index and all its data.
    ///
    /// The index disappears atomically (one rename); its files are then removed.
    /// If removal is interrupted, the leftover directory is cleaned up the next
    /// time the data directory is opened.
    ///
    /// # Errors
    ///
    /// [`Error::IndexNotFound`] or [`Error::Io`].
    pub fn delete_index(&self, name: &IndexName) -> Result<()> {
        let _guard = self.lock_lifecycle();
        let target = self.index_path(name);
        if !target.exists() {
            return Err(Error::IndexNotFound {
                name: name.to_string(),
            });
        }
        let trash = self
            .indexes_dir()
            .join(format!("{DELETING_PREFIX}{}", random_id()));
        fs::rename(&target, &trash).map_err(Error::io(format!("deleting {}", target.display())))?;
        let indexes = self.indexes_dir();
        sync_dir(&indexes).map_err(Error::io(format!("syncing {}", indexes.display())))?;

        // The delete is already durable. A failure here only leaves files that
        // `DataDir::open` removes on the next start.
        let _ = fs::remove_dir_all(&trash);
        Ok(())
    }

    fn index_path(&self, name: &IndexName) -> PathBuf {
        self.indexes_dir().join(name.as_str())
    }
}

fn object_or_empty(what: &str, value: Value) -> Result<Value> {
    match value {
        Value::Null => Ok(Value::Object(Map::new())),
        Value::Object(_) => Ok(value),
        other => Err(Error::InvalidArgument(format!(
            "{what} must be a JSON object, got {}",
            json_type(&other)
        ))),
    }
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn name(s: &str) -> IndexName {
        IndexName::new(s).unwrap()
    }

    fn open() -> (tempfile::TempDir, DataDir) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = DataDir::open(tmp.path()).unwrap();
        (tmp, dir)
    }

    #[test]
    fn create_lays_out_the_index_directory() {
        let (tmp, dir) = open();
        let meta = dir
            .create_index(
                &name("movies"),
                json!({"index": {"refresh_interval": "1s"}}),
                json!({"properties": {"title": {"type": "text"}}}),
            )
            .unwrap();

        let root = tmp.path().join("indexes/movies");
        assert!(root.join("index.json").is_file());
        assert!(root.join("tantivy").is_dir());
        assert!(root.join("wal").is_dir());
        assert_eq!(meta.format_version, FORMAT_VERSION);
        assert_eq!(dir.index_metadata(&name("movies")).unwrap(), meta);
    }

    #[test]
    fn metadata_survives_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let created = DataDir::open(tmp.path())
            .unwrap()
            .create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
        let reopened = DataDir::open(tmp.path()).unwrap();
        assert_eq!(reopened.index_metadata(&name("movies")).unwrap(), created);
        assert_eq!(created.settings, json!({}));
        assert_eq!(created.mappings, json!({}));
    }

    #[test]
    fn creating_an_existing_index_fails() {
        let (_tmp, dir) = open();
        dir.create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
        assert!(matches!(
            dir.create_index(&name("movies"), Value::Null, Value::Null),
            Err(Error::IndexAlreadyExists { name }) if name == "movies"
        ));
    }

    #[test]
    fn settings_and_mappings_must_be_objects() {
        let (_tmp, dir) = open();
        for (settings, mappings) in [(json!([1]), Value::Null), (Value::Null, json!("x"))] {
            assert!(matches!(
                dir.create_index(&name("movies"), settings, mappings),
                Err(Error::InvalidArgument(_))
            ));
        }
        assert!(!dir.index_exists(&name("movies")), "nothing was created");
    }

    #[test]
    fn list_is_sorted_and_ignores_non_indexes() {
        let (tmp, dir) = open();
        for n in ["shows", "movies", "books"] {
            dir.create_index(&name(n), Value::Null, Value::Null)
                .unwrap();
        }
        let indexes = tmp.path().join("indexes");
        fs::create_dir(indexes.join("no-metadata")).unwrap();
        fs::create_dir(indexes.join("Not An Index")).unwrap();

        let listed: Vec<String> = dir
            .list_indexes()
            .unwrap()
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(listed, ["books", "movies", "shows"]);
    }

    #[test]
    fn delete_removes_the_index_and_allows_recreation() {
        let (tmp, dir) = open();
        let first = dir
            .create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
        dir.delete_index(&name("movies")).unwrap();

        assert!(!dir.index_exists(&name("movies")));
        assert!(matches!(
            dir.index_metadata(&name("movies")),
            Err(Error::IndexNotFound { .. })
        ));
        assert!(matches!(
            dir.delete_index(&name("movies")),
            Err(Error::IndexNotFound { .. })
        ));
        assert_eq!(fs::read_dir(tmp.path().join("indexes")).unwrap().count(), 0);

        let second = dir
            .create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
        assert_ne!(
            first.uuid, second.uuid,
            "a re-created index is a new instance"
        );
    }

    #[test]
    fn interrupted_creation_is_invisible_and_cleaned_up() {
        // Simulates a crash after the staging directory was fully written but
        // before the publishing rename.
        let tmp = tempfile::tempdir().unwrap();
        {
            let dir = DataDir::open(tmp.path()).unwrap();
            let meta = dir
                .create_index(&name("movies"), Value::Null, Value::Null)
                .unwrap();
            let indexes = tmp.path().join("indexes");
            fs::rename(
                indexes.join("movies"),
                indexes.join(format!("{CREATING_PREFIX}{}", meta.uuid)),
            )
            .unwrap();
            assert_eq!(dir.list_indexes().unwrap(), Vec::<IndexName>::new());
            assert!(!dir.index_exists(&name("movies")));
        }
        let dir = DataDir::open(tmp.path()).unwrap();
        assert_eq!(dir.list_indexes().unwrap(), Vec::<IndexName>::new());
        assert_eq!(fs::read_dir(tmp.path().join("indexes")).unwrap().count(), 0);
        dir.create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
    }

    #[test]
    fn interrupted_deletion_is_invisible_and_cleaned_up() {
        // Simulates a crash after the rename to `.deleting-*` but before removal.
        let tmp = tempfile::tempdir().unwrap();
        {
            let dir = DataDir::open(tmp.path()).unwrap();
            dir.create_index(&name("movies"), Value::Null, Value::Null)
                .unwrap();
            let indexes = tmp.path().join("indexes");
            fs::rename(
                indexes.join("movies"),
                indexes.join(format!("{DELETING_PREFIX}crashed")),
            )
            .unwrap();
        }
        let dir = DataDir::open(tmp.path()).unwrap();
        assert!(!dir.index_exists(&name("movies")));
        assert_eq!(fs::read_dir(tmp.path().join("indexes")).unwrap().count(), 0);
    }

    #[test]
    fn newer_index_format_is_refused() {
        let (tmp, dir) = open();
        dir.create_index(&name("movies"), Value::Null, Value::Null)
            .unwrap();
        let path = tmp.path().join("indexes/movies/index.json");
        let mut meta: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        meta["format_version"] = json!(FORMAT_VERSION + 1);
        fs::write(&path, serde_json::to_vec(&meta).unwrap()).unwrap();

        assert!(matches!(
            dir.index_metadata(&name("movies")),
            Err(Error::UnsupportedFormat { .. })
        ));
    }
}
