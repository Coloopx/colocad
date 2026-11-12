//! Release clean pages belonging to the single geometry revision displaced by a commit.
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::ffi::{CString, OsStr};
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) struct Revision {
    id: String,
    artifact: Option<String>,
}

pub(super) fn previous(db: &Connection, request: &Value) -> Option<Revision> {
    let step = &request["step"];
    if request["op"] != "finish"
        || step["status"] != "succeeded"
        || !matches!(step["operation"].as_str(), Some("build" | "edit"))
    {
        return None;
    }
    // Use the same partial index as head(). Cache advice is never a commit prerequisite.
    db.query_row(
        &format!("SELECT id,json_extract(entry,'$.artifactDirectory') FROM steps WHERE {} ORDER BY seq DESC LIMIT 1", super::ACCEPTED),
        [],
        |row| Ok(Revision { id: row.get(0)?, artifact: row.get(1)? }),
    )
    .optional()
    .ok()
    .flatten()
    .filter(|revision| revision.artifact.is_some() && revision.artifact.as_deref() != step["artifactDirectory"].as_str())
}

fn child(parent: &File, name: &OsStr, directory: bool) -> io::Result<File> {
    if name.is_empty() || name == "." || name == ".." || name.as_bytes().contains(&b'/') {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let name = CString::new(name.as_bytes())?;
    let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK;
    if directory {
        flags |= libc::O_DIRECTORY;
    }
    // SAFETY: parent remains open; name is a single NUL-terminated component.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a new uniquely owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn advise_file(directory: &File, name: &OsStr) {
    let Ok(file) = child(directory, name, false) else {
        return;
    };
    if file.metadata().is_ok_and(|m| m.is_file() && m.nlink() == 1) {
        // The SDK synced this immutable file before publishing the old revision.
        unsafe {
            libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED);
        }
    }
}

fn revision_job(path: &str) -> Option<&str> {
    let job = path.strip_prefix("artifacts/")?.strip_suffix("/revision")?;
    let suffix = job.strip_prefix("job-")?;
    (suffix.len() == 6 && suffix.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(job)
}

fn artifact_directory(root: &File, path: &str) -> Option<File> {
    let job = revision_job(path)?;
    child(root, "artifacts".as_ref(), true)
        .and_then(|artifacts| child(&artifacts, job.as_ref(), true))
        .and_then(|job| child(&job, "revision".as_ref(), true))
        .ok()
}

// The edit path consumes the current checkpoints and query snapshots. Bulk exchange
// files are already durable and are not read to build the next revision.
pub(super) fn release_exports(root: &Path, artifact: &str) {
    let Ok(root) = File::open(root) else { return };
    let Some(directory) = artifact_directory(&root, artifact) else {
        return;
    };
    for name in ["model.step", "model.glb", "model.stl"] {
        advise_file(&directory, name.as_ref());
    }
}

impl Revision {
    pub(super) fn release(&self, root: &Path) {
        let Some(number) = self.id.strip_prefix('s') else {
            return;
        };
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            return;
        }
        let Ok(root) = File::open(root) else { return };
        if let Ok(steps) = child(&root, "steps".as_ref(), true)
            .and_then(|steps| child(&steps, self.id.as_ref(), true))
        {
            for name in ["input.json", "receipt.json"] {
                advise_file(&steps, name.as_ref());
            }
        }
        let Some(dir) = self
            .artifact
            .as_deref()
            .and_then(|path| artifact_directory(&root, path))
        else {
            return;
        };
        // The publisher writes a flat directory. Iterate the already opened directory descriptor.
        if let Ok(entries) = fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd())) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_file()) {
                    advise_file(&dir, &entry.file_name());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_a_new_successful_geometry_displaces_an_accepted_head() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(super::super::SCHEMA).unwrap();
        for (i, operation, status) in [
            (1, "build", "succeeded"),
            (2, "query", "succeeded"),
            (3, "edit", "failed"),
            (4, "edit", "running"),
        ] {
            let entry = json!({"id":format!("s{i:04}"),"operation":operation,"status":status,"artifactDirectory":format!("artifacts/job-{i:06}/revision")});
            db.execute(
                "INSERT INTO steps(seq,entry) VALUES (?1,?2)",
                rusqlite::params![i, entry.to_string()],
            )
            .unwrap();
        }
        let finish = |operation, status, dir| json!({"op":"finish","step":{"id":"s0004","operation":operation,"status":status,"artifactDirectory":dir}});
        let new_dir = "artifacts/job-000004/revision";
        let head = previous(&db, &finish("edit", "succeeded", new_dir)).unwrap();
        assert_eq!(head.id, "s0001");
        assert!(previous(&db, &finish("edit", "failed", new_dir)).is_none());
        assert!(previous(&db, &finish("observe", "succeeded", new_dir)).is_none());
        assert!(previous(&db, &finish("query", "succeeded", new_dir)).is_none());
        assert!(previous(
            &db,
            &finish("build", "succeeded", "artifacts/job-000001/revision")
        )
        .is_none());
    }

    #[test]
    fn cache_advice_only_resolves_one_owned_child_at_a_time() {
        assert_eq!(
            revision_job("artifacts/job-Ab93x9/revision"),
            Some("job-Ab93x9")
        );
        assert!(revision_job("artifacts/job-12/345/revision").is_none());
        let proc = File::open("/proc").unwrap();
        for name in ["..", "self", "/tmp", "self/fd"] {
            assert!(child(&proc, name.as_ref(), true).is_err());
        }
    }
}
