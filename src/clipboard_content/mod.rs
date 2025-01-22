use rusqlite::{Connection, Result as SqlResult};
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_segmentation::UnicodeSegmentation;

const MAX_DISPLAY_LENGTH: usize = 150;
const DEFAULT_MAX_ITEMS: usize = 100;

#[derive(Debug)]
struct ClipboardType {
    id: i64,
    item_id: i64,
    uti: String,
    data: Vec<u8>,
    size: usize,
}

#[derive(Debug)]
struct ClipboardItem {
    id: i64,
    event_id: i64,
    types: Vec<ClipboardType>,
}

#[derive(Debug)]
struct ClipboardEvent {
    id: i64,
    timestamp: i64,
    items: Vec<ClipboardItem>,
}

#[derive(Debug)]
pub struct ClipboardContent {
    events: Vec<ClipboardEvent>,
    max_events: usize,
    conn: Connection,
    current_event_id: Option<i64>,
}

impl ClipboardContent {
    pub fn new() -> SqlResult<Self> {
        let mut dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        dir.push("cb_stack_rs");
        dir.push("clipboard.db");
        if let Some(parent) = dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&dir)?;

        // Create tables for the hierarchical structure
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_events (
                id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY,
                event_id INTEGER NOT NULL,
                FOREIGN KEY(event_id) REFERENCES clipboard_events(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_types (
                id INTEGER PRIMARY KEY,
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

        Ok(Self {
            events: Vec::new(),
            max_events: DEFAULT_MAX_ITEMS,
            conn,
            current_event_id: None,
        })
    }

    pub fn with_max_items(max_events: usize) -> SqlResult<Self> {
        let mut content = Self::new()?;
        content.max_events = max_events;
        Ok(content)
    }

    pub fn start_event(&mut self) -> SqlResult<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO clipboard_events (timestamp) VALUES (?1)",
            [timestamp],
        )?;

        self.current_event_id = Some(self.conn.last_insert_rowid());
        self.cleanup_old_events()?;
        Ok(())
    }

    pub fn start_item(&mut self) -> SqlResult<i64> {
        let event_id = self
            .current_event_id
            .ok_or_else(|| rusqlite::Error::InvalidParameterName("No active event".to_string()))?;

        self.conn.execute(
            "INSERT INTO clipboard_items (event_id) VALUES (?1)",
            [event_id],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_type(&mut self, item_id: i64, uti: String, data: Vec<u8>) -> SqlResult<()> {
        let size = data.len();

        self.conn.execute(
            "INSERT INTO clipboard_types (item_id, uti, data, size) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![item_id, uti, data, size as i64],
        )?;

        Ok(())
    }

    fn create_item(&mut self) -> SqlResult<i64> {
        let event_id = self
            .current_event_id
            .ok_or_else(|| rusqlite::Error::InvalidParameterName("No active event".to_string()))?;

        self.conn.execute(
            "INSERT INTO clipboard_items (event_id) VALUES (?1)",
            [event_id],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn on_data(&mut self, uti: String, data: Vec<u8>) -> SqlResult<()> {
        let item_id = self.create_item()?;
        let size = data.len();

        self.conn.execute(
            "INSERT INTO clipboard_types (item_id, uti, data, size) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![item_id, uti, data, size as i64],
        )?;

        Ok(())
    }

    fn cleanup_old_events(&mut self) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM clipboard_events WHERE id NOT IN (
                SELECT id FROM clipboard_events ORDER BY timestamp DESC LIMIT ?
            )",
            [self.max_events as i64],
        )?;
        Ok(())
    }

    fn load_items(&mut self) -> SqlResult<()> {
        self.events.clear();

        let mut stmt = self.conn.prepare(
            "SELECT e.id as event_id, e.timestamp, i.id as item_id, t.id as type_id, t.uti, t.data, t.size 
             FROM clipboard_events e
             JOIN clipboard_items i ON i.event_id = e.id
             JOIN clipboard_types t ON t.item_id = i.id
             ORDER BY e.timestamp DESC, i.id, t.id"
        )?;

        let mut current_event: Option<ClipboardEvent> = None;
        let mut current_item: Option<ClipboardItem> = None;

        let rows = stmt.query_map([], |row| {
            let event_id: i64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let item_id: i64 = row.get(2)?;
            let type_id: i64 = row.get(3)?;
            let uti: String = row.get(4)?;
            let data: Vec<u8> = row.get(5)?;
            let size: i64 = row.get(6)?;

            Ok((event_id, timestamp, item_id, type_id, uti, data, size))
        })?;

        for row in rows {
            let (event_id, timestamp, item_id, type_id, uti, data, size) = row?;

            // Handle event change
            if let Some(ref mut event) = current_event {
                if event.id != event_id {
                    // Push current item if exists
                    if let Some(item) = current_item.take() {
                        event.items.push(item);
                    }
                    // Push current event and create new one
                    self.events.push(current_event.take().unwrap());
                    current_event = Some(ClipboardEvent {
                        id: event_id,
                        timestamp,
                        items: Vec::new(),
                    });
                }
            } else {
                // First event
                current_event = Some(ClipboardEvent {
                    id: event_id,
                    timestamp,
                    items: Vec::new(),
                });
            }

            // Handle item change
            if let Some(ref mut item) = current_item {
                if item.id != item_id {
                    // Push current item to current event
                    if let Some(ref mut event) = current_event {
                        event.items.push(current_item.take().unwrap());
                    }
                    current_item = Some(ClipboardItem {
                        id: item_id,
                        event_id,
                        types: Vec::new(),
                    });
                }
            } else {
                // First item
                current_item = Some(ClipboardItem {
                    id: item_id,
                    event_id,
                    types: Vec::new(),
                });
            }

            // Add type to current item
            if let Some(ref mut item) = current_item {
                item.types.push(ClipboardType {
                    id: type_id,
                    item_id,
                    uti,
                    data,
                    size: size as usize,
                });
            }
        }

        // Don't forget to push the last item and event
        if let Some(ref mut event) = current_event {
            if let Some(item) = current_item.take() {
                event.items.push(item);
            }
            self.events.push(current_event.take().unwrap());
        }

        Ok(())
    }

    fn truncate_string(s: &str) -> String {
        let graphemes: Vec<&str> = s.graphemes(true).collect();
        if graphemes.len() > MAX_DISPLAY_LENGTH {
            format!("{}...", graphemes[..MAX_DISPLAY_LENGTH].join(""))
        } else {
            s.to_string()
        }
    }

    pub fn display_all(&mut self) -> SqlResult<()> {
        self.load_items()?;
        for event in &self.events {
            println!("Event {}:", event.id);
            for item in &event.items {
                println!("Item {}:", item.id);
                for type_data in &item.types {
                    if type_data.uti == "public.utf8-plain-text" {
                        println!(
                            "  {}: {}",
                            type_data.uti,
                            Self::truncate_string(&String::from_utf8_lossy(&type_data.data))
                        );
                    } else {
                        println!("  {}: {} bytes", type_data.uti, type_data.size);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn display(&mut self) -> SqlResult<String> {
        self.load_items()?;

        if self.events.is_empty() {
            return Ok("empty".to_string());
        }

        let event = &self.events[0];

        Ok(
            if let Some(img) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("public.png"))
            {
                format!("image, size: {} bytes", img.size)
            } else if let Some(_) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("public.html"))
            {
                if let Some(text) = event.items[0]
                    .types
                    .iter()
                    .find(|x| x.uti == "public.utf8-plain-text")
                {
                    format!(
                        "html, {}",
                        Self::truncate_string(&String::from_utf8_lossy(&text.data))
                    )
                } else {
                    "html, no plain text".to_string()
                }
            } else if let Some(pdf) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("com.adobe.pdf"))
            {
                format!("pdf, size: {} bytes", pdf.size)
            } else if let Some(url) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("public.url"))
            {
                format!(
                    "url: {}",
                    Self::truncate_string(&String::from_utf8_lossy(&url.data))
                )
            } else if let Some(file) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("public.file-url"))
            {
                format!(
                    "file: {}",
                    Self::truncate_string(&String::from_utf8_lossy(&file.data))
                )
            } else if let Some(rtf) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti.contains("public.rtf"))
            {
                if let Some(text) = event.items[0]
                    .types
                    .iter()
                    .find(|x| x.uti == "public.utf8-plain-text")
                {
                    format!(
                        "rtf, {}",
                        Self::truncate_string(&String::from_utf8_lossy(&text.data))
                    )
                } else {
                    format!("rtf, size: {} bytes", rtf.size)
                }
            } else if let Some(text) = event.items[0]
                .types
                .iter()
                .find(|x| x.uti == "public.utf8-plain-text")
            {
                format!(
                    "text, {}",
                    Self::truncate_string(&String::from_utf8_lossy(&text.data))
                )
            } else if let Some(first) = event.items[0].types.first() {
                format!("{}, size: {} bytes", first.uti, first.size)
            } else {
                "empty".to_string()
            },
        )
    }

    pub fn len(&mut self) -> SqlResult<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT event_id) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}
