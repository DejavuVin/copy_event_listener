use crate::clipboard_content::clipboard_event::{ClipboardEvent, ClipboardItem, ClipboardType};
use rusqlite::{Connection, Result as SqlResult};
use std::time::{SystemTime, UNIX_EPOCH};
const DEFAULT_MAX_ITEMS: usize = 100;

#[derive(Debug)]
pub struct ClipboardStorage<'a> {
    max_events: usize,
    conn: &'a Connection,
    current_event: Option<ClipboardEvent>,
    current_item: Option<ClipboardItem>,
}

impl<'a> ClipboardStorage<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            max_events: DEFAULT_MAX_ITEMS,
            conn,
            current_event: None,
            current_item: None,
        }
    }

    pub fn start_event(&mut self) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.current_event = Some(ClipboardEvent::new(timestamp));
        self.current_item = None;
    }

    fn store_item(&mut self) {
        if let Some(event) = self.current_event.as_mut() {
            if let Some(item) = self.current_item.take() {
                event.add_item(item);
            }
        }
    }

    pub fn start_item(&mut self) {
        if self.current_event.is_none() {
            self.start_event();
        } else {
            self.store_item();
        }
        self.current_item = Some(ClipboardItem::new());
    }

    pub fn add_type(&mut self, uti: String, data: Vec<u8>) {
        if self.current_item.is_none() {
            self.start_item();
        }

        self.current_item
            .as_mut()
            .unwrap()
            .add_type(ClipboardType::new(uti, data));
    }

    fn is_duplicate_event(&self, event_hash: &str) -> SqlResult<bool> {
        match self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_events WHERE event_hash = ?1",
            [event_hash],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => Ok(count > 0),
            Err(e) => Err(e),
        }
    }

    fn store_event(&mut self, event: &mut ClipboardEvent) -> SqlResult<i64> {
        let event_hash = event.calculate_hash();
        self.conn.execute(
            "INSERT INTO clipboard_events (timestamp, event_hash) VALUES (?1, ?2)",
            rusqlite::params![event.timestamp, event_hash],
        )?;

        let event_id = self.conn.last_insert_rowid();
        event.id = Some(event_id);

        for item in &mut event.items {
            self.conn.execute(
                "INSERT INTO clipboard_items (event_id) VALUES (?1)",
                [event_id],
            )?;

            let item_id = self.conn.last_insert_rowid();
            item.id = Some(item_id);
            item.event_id = Some(event_id);

            for type_data in &mut item.types {
                self.conn.execute(
                    "INSERT INTO clipboard_types (item_id, uti, data, size) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![item_id, &type_data.uti, &type_data.data, type_data.size as i64],
                )?;

                let type_id = self.conn.last_insert_rowid();
                type_data.id = Some(type_id);
                type_data.item_id = Some(item_id);
            }
        }

        Ok(event_id)
    }

    pub fn finalize_event(&mut self) -> SqlResult<()> {
        self.store_item();

        let mut event = match self.current_event.take() {
            None => return Ok(()),
            Some(event) => event,
        };

        let event_hash = event.calculate_hash();
        if self.is_duplicate_event(&event_hash)? {
            return Ok(());
        }

        self.store_event(&mut event)?;
        self.cleanup_old_events()?;

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
}
