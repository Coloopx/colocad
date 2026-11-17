//! Cold revision members share append-only segments. The committed end and the
//! two directory locators advance atomically with the new accepted head.
use super::{one_entry, Result, ACCEPTED};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "
CREATE TABLE segments(id INTEGER PRIMARY KEY, bytes INTEGER NOT NULL, revisions INTEGER NOT NULL);
CREATE TABLE archived(dir TEXT PRIMARY KEY, segment INTEGER NOT NULL, offset INTEGER NOT NULL, length INTEGER NOT NULL, digest TEXT NOT NULL, pending INTEGER NOT NULL) WITHOUT ROWID;
CREATE INDEX cleanup ON archived(dir) WHERE pending=1;
PRAGMA user_version=3;";

fn validate_dir(dir: &str) -> Result<()> {
    let parts: Vec<_> = dir.split('/').collect();
    let valid = match parts.as_slice() {
        ["steps", id] => id
            .strip_prefix('s')
            .is_some_and(|n| n.len() >= 4 && n.bytes().all(|c| c.is_ascii_digit())),
        ["artifacts", job, "revision"] => job
            .strip_prefix("job-")
            .is_some_and(|s| s.len() == 6 && s.bytes().all(|c| c.is_ascii_alphanumeric())),
        _ => false,
    };
    if !valid {
        return Err("SESSION_PATH: invalid cold directory".into());
    }
    Ok(())
}

fn owned(root: &Path, dir: &str) -> Result<PathBuf> {
    validate_dir(dir)?;
    let path = root.join(dir);
    if path.canonicalize()? != path {
        return Err("SESSION_PATH: cold directory symlink".into());
    }
    Ok(path)
}

fn pack_path(root: &Path, id: i64) -> Result<PathBuf> {
    let parent = root.join("segments");
    if parent.canonicalize()? != parent {
        return Err("SESSION_PATH: segment directory symlink".into());
    }
    Ok(parent.join(format!("history-{id}.pack")))
}

pub fn lookup(db: &Connection, dir: &str) -> Result<Value> {
    Ok(db.query_row("SELECT segment,offset,length,digest FROM archived WHERE dir=?1",[dir],|r|
        Ok(json!({"segment":r.get::<_,i64>(0)?,"offset":r.get::<_,i64>(1)?,"length":r.get::<_,i64>(2)?,"sha256":r.get::<_,String>(3)?}))
    ).optional()?.unwrap_or(Value::Null))
}

fn archive_dir(db: &Connection, root: &Path, dir: &str, id: i64, pack: &mut File) -> Result<()> {
    let source = owned(root, dir)?;
    let mut members = BTreeMap::new();
    let mut buffer = [0u8; 64 * 1024];
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid member name")?;
        if !entry.file_type()?.is_file() || name.contains('\\') {
            return Err("SESSION_PATH: invalid archive member".into());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(entry.path())?;
        if !file.metadata()?.is_file() {
            return Err("SESSION_PATH: invalid member type".into());
        }
        let offset = pack.stream_position()?;
        let mut hash = Sha256::new();
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            pack.write_all(&buffer[..n])?;
            hash.update(&buffer[..n]);
        }
        members.insert(name,json!({"offset":offset,"length":pack.stream_position()?-offset,"sha256":format!("{:x}",hash.finalize())}));
    }
    let offset = pack.stream_position()?;
    let index = serde_json::to_vec(&members)?;
    if index.len() > 16 * 1024 * 1024 {
        return Err("SESSION_LIMIT: archive index exceeds 16 MiB".into());
    }
    pack.write_all(&index)?;
    let digest = format!("{:x}", Sha256::digest(&index));
    db.execute(
        "INSERT INTO archived VALUES (?1,?2,?3,?4,?5,1)",
        params![dir, id, offset, index.len(), digest],
    )?;
    Ok(())
}

pub fn displace(db: &Connection, root: &Path, step: &Value) -> Result<()> {
    if step["status"] != "succeeded"
        || !matches!(step["operation"].as_str(), Some("edit" | "build"))
    {
        return Ok(());
    }
    let old = one_entry(
        db,
        &format!("SELECT entry FROM steps WHERE {ACCEPTED} ORDER BY seq DESC LIMIT 1"),
    )?;
    if old.is_null() || old["artifactDirectory"] == step["artifactDirectory"] {
        return Ok(());
    }
    let (last, bytes, count) = db
        .query_row(
            "SELECT id,bytes,revisions FROM segments ORDER BY id DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, u64>(1)?,
                    r.get::<_, usize>(2)?,
                ))
            },
        )
        .optional()?
        .unwrap_or((0, 0, 0));
    let new = last == 0 || count >= 16 || bytes >= 64 * 1024 * 1024;
    let (id, end) = if new { (last + 1, 0) } else { (last, bytes) };
    let path = pack_path(root, id)?;
    let mut pack = OpenOptions::new()
        .write(true)
        .read(true)
        .create(new)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !pack.metadata()?.is_file() || pack.metadata()?.nlink() != 1 {
        return Err("SESSION_PATH: invalid segment file".into());
    }
    if pack.metadata()?.len() < end {
        return Err("SESSION_INTEGRITY: truncated segment".into());
    }
    // A killed writer may have left an unindexed tail. Only committed ranges are visible.
    pack.set_len(end)?;
    pack.seek(SeekFrom::Start(end))?;
    archive_dir(
        db,
        root,
        old["artifactDirectory"]
            .as_str()
            .ok_or("missing geometry")?,
        id,
        &mut pack,
    )?;
    archive_dir(
        db,
        root,
        &format!("steps/{}", old["id"].as_str().ok_or("missing step")?),
        id,
        &mut pack,
    )?;
    pack.sync_all()?;
    if new {
        File::open(root.join("segments"))?.sync_all()?;
    }
    db.execute("INSERT INTO segments VALUES (?1,?2,1) ON CONFLICT(id) DO UPDATE SET bytes=excluded.bytes,revisions=revisions+1",params![id,pack.stream_position()?])?;
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        unsafe {
            libc::posix_fadvise(pack.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED);
        }
    }
    Ok(())
}

// Only remove a directory after its complete immutable copy was committed.
// A killed or failed cleanup is retried under the next writer lease.
pub fn cleanup(db: &Connection, root: &Path) -> Result<()> {
    let mut statement = db.prepare("SELECT dir FROM archived WHERE pending=1 LIMIT 8")?;
    let dirs = statement
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    if dirs.is_empty() {
        return Ok(());
    }
    let tx = db.unchecked_transaction()?;
    for dir in dirs {
        validate_dir(&dir)?;
        let path = root.join(&dir);
        if path.try_exists()? {
            let checked = owned(root, &dir)?;
            // Never recursively delete user-created content or follow links.
            let locator = lookup(db, &dir)?;
            let mut pack = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(pack_path(
                    root,
                    locator["segment"].as_i64().ok_or("segment")?,
                )?)?;
            pack.seek(SeekFrom::Start(locator["offset"].as_u64().ok_or("offset")?))?;
            let length = locator["length"]
                .as_u64()
                .filter(|n| *n <= 16 * 1024 * 1024)
                .ok_or("invalid index length")?;
            let mut index = Vec::new();
            pack.take(length).read_to_end(&mut index)?;
            if format!("{:x}", Sha256::digest(&index)) != locator["sha256"] {
                return Err("SESSION_INTEGRITY: segment index".into());
            }
            let members: BTreeMap<String, Value> = serde_json::from_slice(&index)?;
            for name in members.keys() {
                if Path::new(name).file_name().and_then(|s| s.to_str()) != Some(name) {
                    return Err("invalid member".into());
                }
                match fs::remove_file(checked.join(name)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    r => r?,
                }
            }
            match fs::remove_dir(&checked) {
                Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => (),
                r => r?,
            }
        }
        if dir.starts_with("artifacts/") {
            let parent = path.parent().ok_or("invalid job")?;
            if parent.try_exists()? && parent.canonicalize()? != parent {
                return Err("SESSION_PATH: job symlink".into());
            }
            match fs::remove_dir(parent) {
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) =>
                {
                    ()
                }
                r => r?,
            }
        }
        tx.execute("UPDATE archived SET pending=0 WHERE dir=?1", [dir])?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn published_index_precedes_cleanup_and_corrupt_index_preserves_loose_files() -> Result<()> {
        let temp = std::env::temp_dir().join(format!(
            "colocad-pack-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&temp)?;
        let root = temp.canonicalize()?;
        let result = (|| -> Result<()> {
            fs::create_dir(root.join("segments"))?;
            let dir = "artifacts/job-Abc123/revision";
            let steps = "steps/s0001";
            for name in [dir, steps] {
                fs::create_dir_all(root.join(name))?;
            }
            fs::write(root.join(dir).join("model.step"), b"preserved payload")?;
            fs::write(root.join(steps).join("input.json"), b"{}")?;
            let db = Connection::open_in_memory()?;
            db.execute_batch(super::super::SCHEMA)?;
            db.execute_batch(SCHEMA)?;
            let head = json!({"id":"s0001","operation":"build","status":"succeeded","artifactDirectory":dir});
            db.execute(
                "INSERT INTO steps(seq,entry) VALUES(1,?1)",
                [head.to_string()],
            )?;
            let next = json!({"operation":"edit","status":"succeeded","artifactDirectory":"artifacts/job-Xyz123/revision"});
            let tx = db.unchecked_transaction()?;
            displace(&tx, &root, &next)?;
            tx.rollback()?;
            assert!(lookup(&db, dir)?.is_null());
            assert!(root.join(dir).join("model.step").is_file());
            let tx = db.unchecked_transaction()?;
            displace(&tx, &root, &next)?;
            tx.commit()?;
            let locator = lookup(&db, dir)?;
            let pos = locator["offset"].as_u64().unwrap();
            let mut pack = OpenOptions::new()
                .read(true)
                .write(true)
                .open(pack_path(&root, 1)?)?;
            pack.seek(SeekFrom::Start(pos))?;
            let mut original = [0u8; 1];
            pack.read_exact(&mut original)?;
            pack.seek(SeekFrom::Start(pos))?;
            pack.write_all(b"!")?;
            assert!(cleanup(&db, &root).is_err());
            assert_eq!(
                fs::read(root.join(dir).join("model.step"))?,
                b"preserved payload"
            );
            pack.seek(SeekFrom::Start(pos))?;
            pack.write_all(&original)?;
            pack.sync_all()?;
            cleanup(&db, &root)?;
            cleanup(&db, &root)?;
            assert!(!root.join(dir).exists());
            assert_eq!(
                db.query_row("SELECT count(*) FROM archived WHERE pending=1", [], |r| r
                    .get::<_, i64>(
                    0
                ))?,
                0
            );
            Ok(())
        })();
        fs::remove_dir_all(temp)?;
        result
    }
}
