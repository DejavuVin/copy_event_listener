use sha2::{Digest, Sha256};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct ClipboardType {
    pub id: Option<i64>, // Optional since we don't have an id until stored
    pub item_id: Option<i64>,
    pub uti: String,
    pub data: Vec<u8>,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ClipboardItem {
    pub id: Option<i64>,
    pub event_id: Option<i64>,
    pub types: Vec<ClipboardType>,
}

#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub items: Vec<ClipboardItem>,
}

impl ClipboardEvent {
    pub fn new(timestamp: i64) -> Self {
        Self {
            id: None,
            timestamp,
            items: Vec::new(),
        }
    }

    pub fn add_item(&mut self, item: ClipboardItem) {
        self.items.push(item);
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Sort items by their types to ensure consistent hashing
        for item in &self.items {
            let mut types = item.types.clone();
            types.sort_by(|a, b| a.uti.cmp(&b.uti));

            for type_data in types {
                hasher.update(type_data.uti.as_bytes());
                hasher.update(&type_data.data);
            }
        }

        format!("{:x}", hasher.finalize())
    }
}

impl ClipboardItem {
    pub fn new() -> Self {
        Self {
            id: None,
            event_id: None,
            types: Vec::new(),
        }
    }

    pub fn add_type(&mut self, clipboard_type: ClipboardType) {
        self.types.push(clipboard_type);
    }
}

impl ClipboardType {
    pub fn new(uti: String, data: Vec<u8>) -> Self {
        Self {
            id: None,
            item_id: None,
            uti,
            size: data.len(),
            data,
        }
    }
}
