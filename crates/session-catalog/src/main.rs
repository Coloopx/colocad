mod migration;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ACCEPTED: &str = "status='succeeded' AND operation NOT IN ('observe','query')";
const APP_ID: i64 = 0x434f4c4f;
const SCHEMA: &str = "
CREATE TABLE metadata(header TEXT NOT NULL);
CREATE TABLE steps(
    seq INTEGER PRIMARY KEY,
    entry TEXT NOT NULL,
    id TEXT GENERATED ALWAYS AS (json_extract(entry,'$.id')) VIRTUAL NOT NULL UNIQUE,
    status TEXT GENERATED ALWAYS AS (json_extract(entry,'$.status')) VIRTUAL NOT NULL
        CHECK(status IN ('running','succeeded','failed','interrupted')),
    operation TEXT GENERATED ALWAYS AS (coalesce(json_extract(entry,'$.operation'),'')) VIRTUAL
);
CREATE INDEX accepted ON steps(seq) WHERE status='succeeded' AND operation NOT IN ('observe','query');
CREATE INDEX unfinished ON steps(seq) WHERE status='running';
PRAGMA application_id=1129270351;
PRAGMA user_version=1;";

fn catalog_path(root: &Path, name: &str) -> Result<PathBuf> {
    let token = name
        .strip_prefix("session-")
        .and_then(|s| s.strip_suffix(".sqlite"))
        .unwrap_or("");
    if token.len() != 36 || !token.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-') {
        return Err("SESSION_PATH: invalid catalog name".into());
    }
    let path = root.join(name);
    if path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("SESSION_PATH: catalog symlink".into());
    }
    Ok(path)
}

fn open_db(path: &Path, writable: bool) -> Result<Connection> {
    let flags = if writable {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let db = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_NOFOLLOW)?;
    db.execute_batch("PRAGMA cache_size=-2048; PRAGMA mmap_size=0; PRAGMA temp_store=FILE; PRAGMA trusted_schema=OFF;")?;
    if writable {
        db.execute_batch("PRAGMA synchronous=FULL;")?;
    }
    Ok(db)
}

fn initialize(root: &Path, name: &str, mode: &str, input: Value) -> Result<Value> {
    let manifest = root.join("session.json");
    if mode == "init" && manifest.try_exists()? {
        return Err("EEXIST: session already exists".into());
    }
    let path = catalog_path(root, name)?;
    let stage = root.join(".colocad-import.tmp");
    // A previous importer may have been killed. Only this private staging
    // database and its SQLite sidecars are disposable under the OS lease.
    for suffix in ["", "-journal", "-wal", "-shm"] {
        let stale = root.join(format!(".colocad-import.tmp{suffix}"));
        if stale.try_exists()? {
            fs::remove_file(stale)?;
        }
    }
    File::create_new(&stage)?;
    let marker = root.join(format!("{name}.manifest.tmp"));
    let mut linked_database = false;
    let prepared = (|| -> Result<Value> {
        let mut db = open_db(&stage, true)?;
        let tx = db.transaction()?;
        tx.execute_batch(SCHEMA)?;
        let (tx, header) = if mode == "import" {
            tx.commit()?;
            let header = migration::import(File::open(&manifest)?, File::open(&stage)?, &db)?;
            (db.transaction()?, header)
        } else {
            (tx, input)
        };
        if header["format"] != "colocad/session/1" {
            return Err("SESSION_FORMAT".into());
        }
        tx.execute("INSERT INTO metadata VALUES (?1)", [header.to_string()])?;
        tx.commit()?;
        // A closed project can be mounted read-only with just its database.
        // Live writers hold the OS lease and commit short per-step transactions.
        // SQLite's rollback journal avoids requiring WAL/SHM files on readers.
        let count: i64 =
            db.query_row("SELECT coalesce(max(seq),0) FROM steps", [], |r| r.get(0))?;
        db.close().map_err(|(_, error)| error)?;
        File::open(&stage)?.sync_all()?;
        fs::hard_link(&stage, &path)?;
        linked_database = true;
        fs::remove_file(&stage)?;
        let backup = if mode == "import" {
            let backup = format!("{name}.v1.json");
            fs::hard_link(&manifest, root.join(&backup))?;
            Some(backup)
        } else {
            None
        };
        let mut file = File::create_new(&marker)?;
        writeln!(
            file,
            "{}",
            json!({"format":"colocad/session/2","catalog":name})
        )?;
        file.sync_all()?;
        File::open(root)?.sync_all()?;
        // This rename is the cutover. Until now the v1 ledger remains authoritative.
        if mode == "import" {
            fs::rename(&marker, &manifest)?;
        } else {
            fs::hard_link(&marker, &manifest)?;
            fs::remove_file(&marker)?;
        }
        Ok(json!({"steps":count,"backup":backup}))
    })();
    if prepared.is_err() {
        // Do not delete a database reachable through a published manifest.
        let published = fs::metadata(&manifest)
            .ok()
            .filter(|m| m.len() <= 256)
            .and_then(|_| fs::read_to_string(&manifest).ok())
            .filter(|s| s.len() < 256)
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .is_some_and(|v| v["catalog"] == name);
        if linked_database && !published {
            let _ = fs::remove_file(&path);
        }
        let _ = fs::remove_file(&marker);
        let _ = fs::remove_file(&stage);
    }
    let data = prepared?;
    File::open(root)?.sync_all()?;
    Ok(data)
}

fn one_entry(db: &Connection, sql: &str) -> Result<Value> {
    let raw: Option<String> = db.query_row(sql, [], |r| r.get(0)).optional()?;
    Ok(match raw {
        Some(s) => serde_json::from_str(&s)?,
        None => Value::Null,
    })
}

fn page(db: &Connection, request: &Value) -> Result<Value> {
    let after = request["after"].as_u64().ok_or("invalid history cursor")?;
    let limit = request["limit"]
        .as_u64()
        .filter(|n| (1..=256).contains(n))
        .ok_or("history limit must be 1..256")?;
    let mut stmt = db.prepare("SELECT seq,entry FROM steps WHERE seq>?1 ORDER BY seq LIMIT ?2")?;
    let mut rows = stmt.query(params![after, limit + 1])?;
    let mut steps = Vec::new();
    let mut bytes = 0;
    let mut cursor = after;
    let mut next = None;
    while let Some(row) = rows.next()? {
        let entry: String = row.get(1)?;
        if !steps.is_empty() && (steps.len() == limit as usize || bytes + entry.len() > 1024 * 1024)
        {
            next = Some(cursor);
            break;
        }
        bytes += entry.len();
        cursor = row.get(0)?;
        steps.push(serde_json::from_str::<Value>(&entry)?);
    }
    Ok(json!({"steps":steps,"nextCursor":next}))
}

fn dispatch(db: &Connection, request: Value, writable: bool) -> Result<Value> {
    match request["op"].as_str().ok_or("missing operation")? {
        "header" => one_entry(db, "SELECT header FROM metadata"),
        "head" => one_entry(
            db,
            &format!("SELECT entry FROM steps WHERE {ACCEPTED} ORDER BY seq DESC LIMIT 1"),
        ),
        "length" => Ok(json!(db.query_row(
            "SELECT coalesce(max(seq),0) FROM steps",
            [],
            |r| r.get::<_, i64>(0)
        )?)),
        "page" => page(db, &request),
        "begin" if writable => {
            let step = &request["step"];
            let next: i64 =
                db.query_row("SELECT coalesce(max(seq),0)+1 FROM steps", [], |r| r.get(0))?;
            if step["id"] != format!("s{next:04}") || step["status"] != "running" {
                return Err("SESSION_CONFLICT: unexpected next step".into());
            }
            db.execute(
                "INSERT INTO steps(seq,entry) VALUES (?1,?2)",
                params![next, step.to_string()],
            )?;
            Ok(Value::Null)
        }
        "finish" if writable => {
            let step = &request["step"];
            if !matches!(step["status"].as_str(), Some("succeeded" | "failed")) {
                return Err("invalid terminal state".into());
            }
            let changed = db.execute(
                "UPDATE steps SET entry=?1 WHERE id=?2 AND status='running'",
                params![step.to_string(), step["id"].as_str().ok_or("missing id")?],
            )?;
            if changed != 1 {
                return Err("SESSION_CONFLICT: missing running step".into());
            }
            Ok(Value::Null)
        }
        "recover" if writable => {
            let changed=db.execute("UPDATE steps SET entry=json_set(entry,'$.status','interrupted','$.finishedAt',?1,'$.error',json_object('code','INTERRUPTED','message','Previous process ended before committing its result')) WHERE status='running'",[request["now"].as_str().ok_or("missing recovery time")?])?;
            Ok(json!(changed))
        }
        _ => Err("unsupported catalog operation or read-only connection".into()),
    }
}

fn reply(value: Result<Value>) -> io::Result<()> {
    let result = match value {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => json!({"ok":false,"error":e.to_string()}),
    };
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, &result)?;
    writeln!(out)?;
    out.flush()
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("expected ROOT MODE CATALOG".into());
    }
    let canonical_root = fs::canonicalize(&args[1])?;
    let root = canonical_root.as_path();
    let mode = args[2].as_str();
    let name = &args[3];
    if !matches!(mode, "init" | "import" | "read" | "write" | "guard") {
        return Err("invalid mode".into());
    }
    let _lease = if mode != "read" {
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join(".catalog-writer"))?;
        lease
            .try_lock()
            .map_err(|_| "SESSION_BUSY: another local writer owns this session")?;
        Some(lease)
    } else {
        None
    };
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    if matches!(mode, "init" | "import" | "guard") {
        reply(Ok(Value::Null))?;
        if mode == "guard" {
            for line in lines {
                line?;
            }
            return Ok(());
        }
        let input = serde_json::from_str(&lines.next().ok_or("missing initialization data")??)?;
        reply(initialize(root, name, mode, input))?;
        // Retain the OS lease until the SDK has removed the legacy PID lock.
        for line in lines {
            line?;
        }
        return Ok(());
    }
    let db = open_db(&catalog_path(root, name)?, mode == "write")?;
    let app: i64 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if app != APP_ID || version != 1 {
        return Err("SESSION_FORMAT: unsupported SQLite catalog".into());
    }
    if mode == "read" {
        db.execute_batch("BEGIN DEFERRED")?;
    }
    reply(Ok(Value::Null))?;
    for line in lines {
        let request = serde_json::from_str(&line?)?;
        reply(dispatch(&db, request, mode == "write"))?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        let _ = reply(Err(e));
        std::process::exit(1);
    }
}
