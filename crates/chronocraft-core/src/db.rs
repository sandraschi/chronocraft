//! Bundled SQLite persistence for ChronoCraft (`rusqlite` 0.41).

use crate::error::ChronoError;
use crate::ChronoEngine;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub fn default_db_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("chronocraft").join("chronocraft.db")
}

pub fn save_to_db(engine: &ChronoEngine, path: &Path) -> Result<(), ChronoError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ChronoError::Storage(format!("create_dir_all failed: {e}")))?;
        }
    }
    let mut conn =
        Connection::open(path).map_err(|e| ChronoError::Storage(format!("open db failed: {e}")))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS scenarios (
            name TEXT PRIMARY KEY,
            model_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chrono_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_name TEXT NOT NULL
        );",
    )
    .map_err(|e| ChronoError::Storage(format!("schema init failed: {e}")))?;

    let snapshot =
        serde_json::to_string(engine).map_err(|e| ChronoError::Serialization(e.to_string()))?;
    let tx = conn
        .transaction()
        .map_err(|e| ChronoError::Storage(format!("tx start failed: {e}")))?;

    tx.execute(
        "INSERT INTO scenarios (name, model_json) VALUES (?1, ?2)
         ON CONFLICT(name) DO UPDATE SET model_json = excluded.model_json",
        params![engine.active_scenario, snapshot],
    )
    .map_err(|e| ChronoError::Storage(format!("upsert active scenario: {e}")))?;
    tx.execute("DELETE FROM chrono_meta", [])
        .map_err(|e| ChronoError::Storage(format!("clear chrono_meta: {e}")))?;
    tx.execute(
        "INSERT INTO chrono_meta (id, active_name) VALUES (1, ?1)",
        params![engine.active_scenario],
    )
    .map_err(|e| ChronoError::Storage(format!("insert chrono_meta: {e}")))?;

    tx.commit()
        .map_err(|e| ChronoError::Storage(format!("tx commit: {e}")))?;
    Ok(())
}

pub fn list_scenario_names(path: &Path) -> Result<Vec<String>, ChronoError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn =
        Connection::open(path).map_err(|e| ChronoError::Storage(format!("open db failed: {e}")))?;
    let has: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='scenarios')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ChronoError::Storage(format!("table check failed: {e}")))?;
    if !has {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare("SELECT name FROM scenarios ORDER BY name ASC")
        .map_err(|e| ChronoError::Storage(format!("prepare scenarios: {e}")))?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| ChronoError::Storage(format!("query scenarios: {e}")))?;
    let mut names = Vec::new();
    for r in rows {
        names.push(r.map_err(|e| ChronoError::Storage(format!("scenario row: {e}")))?);
    }
    Ok(names)
}

pub fn load_scenario(path: &Path, name: &str) -> Result<Option<ChronoEngine>, ChronoError> {
    if !path.exists() {
        return Ok(None);
    }
    let conn =
        Connection::open(path).map_err(|e| ChronoError::Storage(format!("open db failed: {e}")))?;
    let has: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='scenarios')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ChronoError::Storage(format!("table check failed: {e}")))?;
    if !has {
        return Ok(None);
    }
    let json: Option<String> = conn
        .query_row(
            "SELECT model_json FROM scenarios WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )
        .ok();
    match json {
        Some(text) => {
            let mut engine: ChronoEngine = serde_json::from_str(&text)
                .map_err(|e| ChronoError::Serialization(e.to_string()))?;
            engine.active_scenario = name.to_string();
            Ok(Some(engine))
        }
        None => Ok(None),
    }
}

pub fn load_active(path: &Path) -> Result<Option<ChronoEngine>, ChronoError> {
    if !path.exists() {
        return Ok(None);
    }
    let conn =
        Connection::open(path).map_err(|e| ChronoError::Storage(format!("open db failed: {e}")))?;
    let has: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='chrono_meta')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ChronoError::Storage(format!("table check failed: {e}")))?;
    if !has {
        return Ok(None);
    }
    let name: Option<String> = conn
        .query_row(
            "SELECT active_name FROM chrono_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .ok();
    match name {
        Some(n) => load_scenario(path, &n),
        None => Ok(None),
    }
}
