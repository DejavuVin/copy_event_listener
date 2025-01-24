use rusqlite::{Connection, Result as SqlResult};
use std::path::PathBuf;

pub fn get_db_path() -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.push("paste_stack");
    dir.push("clipboard.db");
    dir
}

pub fn init_database() -> SqlResult<()> {
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let conn = Connection::open(&db_path)?;

    // Create tables for the hierarchical structure
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            event_hash TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id INTEGER NOT NULL,
            FOREIGN KEY(event_id) REFERENCES clipboard_events(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_types (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id INTEGER NOT NULL,
            uti TEXT NOT NULL,
            data BLOB NOT NULL,
            size INTEGER NOT NULL,
            FOREIGN KEY(item_id) REFERENCES clipboard_items(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Enable foreign key support and WAL mode for better performance
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    conn.execute("PRAGMA journal_mode = WAL", [])?;

    Ok(())
}

pub fn get_connection() -> SqlResult<Connection> {
    Connection::open(get_db_path())
}
