// Deserialize the top-level steps array one entry at a time. All inserts and the
// header check share one transaction, including when "format" follows "steps".
use rusqlite::Connection;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde_json::{Map, Value};
use std::{fmt, io::Read};

struct Steps<'a>(&'a Connection);
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
        let mut insert = self
            .0
            .prepare("INSERT INTO steps(entry) VALUES (?1)")
            .map_err(de::Error::custom)?;
        while let Some(entry) = seq.next_element::<Value>()? {
            insert
                .execute([entry.to_string()])
                .map_err(de::Error::custom)?;
        }
        Ok(())
    }
}

struct Ledger<'a>(&'a Connection);
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
                map.next_value_seed(Steps(self.0))?;
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

pub fn import(reader: impl Read, conn: &Connection) -> Result<Value, serde_json::Error> {
    let mut de = serde_json::Deserializer::from_reader(reader);
    let header = de.deserialize_map(Ledger(conn))?;
    de.end()?;
    Ok(header)
}
