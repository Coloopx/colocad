// The descriptor rename publishes the import. All batches remain private until EOF and header validation.
use rusqlite::Connection;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde_json::{Map, Value};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::{
    fmt,
    fs::File,
    io::{self, BufReader, Read},
};

const WINDOW: u64 = 4 << 20;
const MAX_ROWS: usize = 4096;

// Only the legacy input and the private staging DB are sequential, disposable
// cache users. Never apply this policy to reads/writes of a live catalog.
fn discard(file: &File, start: u64, length: u64) {
    #[cfg(target_os = "linux")]
    // SAFETY: borrowed live fd; no pointers. A filesystem may ignore this hint.
    unsafe {
        libc::posix_fadvise(
            file.as_raw_fd(),
            start as _,
            length as _,
            libc::POSIX_FADV_DONTNEED,
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (file, start, length);
}

struct Source {
    file: File,
    read: u64,
    advised: u64,
}
impl Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buf)?;
        self.read += n as u64;
        if self.read - self.advised >= WINDOW || n == 0 {
            let end = self.read / 65536 * 65536;
            if end > self.advised {
                discard(&self.file, self.advised, end - self.advised);
                self.advised = end;
            }
        }
        Ok(n)
    }
}

struct Steps<'a>(&'a Connection, &'a File);
impl<'de> DeserializeSeed<'de> for Steps<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for Steps<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("session steps")
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
        let mut advised = 0;
        loop {
            let tx = self.0.unchecked_transaction().map_err(de::Error::custom)?;
            let mut insert = tx
                .prepare_cached("INSERT INTO steps(entry) VALUES (?1)")
                .map_err(de::Error::custom)?;
            let mut bytes = 0;
            let mut ended = false;
            for _ in 0..MAX_ROWS {
                let Some(entry) = seq.next_element::<Value>()? else {
                    ended = true;
                    break;
                };
                let row = entry.to_string();
                bytes += row.len() as u64;
                insert.execute([row]).map_err(de::Error::custom)?;
                if bytes >= WINDOW {
                    break;
                }
            }
            drop(insert);
            tx.commit().map_err(de::Error::custom)?;
            // FULL sync made newly appended pages clean. Evict just this range;
            // an all-file sweep after each batch would revisit a growing prefix.
            let end = self.1.metadata().map_err(de::Error::custom)?.len() / 65536 * 65536;
            if end > advised {
                discard(self.1, advised, end - advised);
                advised = end;
            }
            if ended {
                break;
            }
        }
        Ok(())
    }
}

struct Ledger<'a>(&'a Connection, &'a File);
impl<'de> Visitor<'de> for Ledger<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an alpha.8 session ledger")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Value, M::Error> {
        let mut header = Map::new();
        let mut found_steps = false;
        while let Some(key) = map.next_key::<String>()? {
            if key == "steps" {
                if found_steps {
                    return Err(de::Error::duplicate_field("steps"));
                }
                map.next_value_seed(Steps(self.0, self.1))?;
                found_steps = true;
            } else {
                header.insert(key, map.next_value()?);
            }
        }
        if !found_steps || header.get("format").and_then(Value::as_str) != Some("colocad/session/1")
        {
            return Err(de::Error::custom(
                "SESSION_FORMAT: expected a version 1 ledger",
            ));
        }
        Ok(Value::Object(header))
    }
}

pub fn import(file: File, stage: File, conn: &Connection) -> Result<Value, serde_json::Error> {
    let reader = BufReader::with_capacity(
        64 << 10,
        Source {
            file,
            read: 0,
            advised: 0,
        },
    );
    let mut de = serde_json::Deserializer::from_reader(reader);
    let header = de.deserialize_map(Ledger(conn, &stage))?;
    de.end()?;
    Ok(header)
}
