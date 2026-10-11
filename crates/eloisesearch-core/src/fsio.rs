//! Crash-safe file system primitives.
//!
//! Everything that must survive a crash or power loss is written through these
//! helpers (see `docs/design-persistence.md`, "On-disk layout"). They rely on two
//! properties of POSIX file systems: `rename` within a directory is atomic, and
//! a rename is only durable once the parent directory has been synced.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Replaces `path` with `bytes` atomically and durably.
///
/// Writes `<path>.tmp`, syncs it, renames it over `path` and syncs the parent
/// directory. After a crash, `path` holds either the old or the new content,
/// never a mix; a leftover `.tmp` file is ignored by readers.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = tmp_path(path);
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_parent(path)
}

/// Makes the creation, removal or rename of entries in `dir` durable.
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        // Directories cannot be opened for syncing on this platform; renames are
        // made durable by the file system itself.
        let _ = dir;
        Ok(())
    }
}

/// Syncs the directory that contains `path`.
pub(crate) fn sync_parent(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => sync_dir(parent),
        _ => sync_dir(Path::new(".")),
    }
}

fn tmp_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_creates_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meta.json");

        write_atomic(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");

        write_atomic(&path, b"second, longer").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second, longer");
        assert!(
            !dir.path().join("meta.json.tmp").exists(),
            "the temporary file is renamed away"
        );
    }

    #[test]
    fn write_atomic_ignores_a_stale_tmp_file() {
        // A crash between writing the temporary file and the rename leaves it behind.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meta.json");
        fs::write(dir.path().join("meta.json.tmp"), b"partial garbage").unwrap();

        write_atomic(&path, b"good").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"good");
    }

    #[test]
    fn sync_dir_works_on_existing_directories() {
        let dir = tempfile::tempdir().unwrap();
        sync_dir(dir.path()).unwrap();
        sync_parent(&dir.path().join("anything")).unwrap();
    }
}
