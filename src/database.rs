use rusqlite::{Connection, Result as SqlResult};
use std::{fs, path::PathBuf};

pub fn get_db_path() -> PathBuf {
    let mut dir = dirs::home_dir().unwrap();
    dir.push("data");
    dir.push("paste_stack");
    dir.push("clipboard.db");
    dir
}

pub fn init_database() -> SqlResult<Connection> {
    let db_path = get_db_path();
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some(format!("Failed to create database directory: {}", e)),
            )
        })?;
    }
    // print db dir
    println!("paste_stack: db dir: {}", db_path.to_string_lossy());
    let conn = Connection::open(&db_path)?;

    // Create tables for the hierarchical structure
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clipboard_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            event_hash TEXT
        );
        
        CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id INTEGER NOT NULL,
            FOREIGN KEY(event_id) REFERENCES clipboard_events(id) ON DELETE CASCADE
        );
        
        CREATE TABLE IF NOT EXISTS clipboard_types (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id INTEGER NOT NULL,
            uti TEXT NOT NULL,
            data BLOB NOT NULL,
            size INTEGER NOT NULL,
            FOREIGN KEY(item_id) REFERENCES clipboard_items(id) ON DELETE CASCADE
        );
        
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;",
    )?;

    Connection::open(get_db_path())
}
