use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Event {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub data_list: Vec<Data>,
}

impl Item {
    pub fn new() -> Self {
        Self {
            data_list: Vec::new(),
        }
    }

    pub fn add_data(&mut self, data: Data) {
        self.data_list.push(data);
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    pub r#type: String,
    pub data: Vec<u8>,
}

impl Data {
    pub fn new(r#type: String, data: Vec<u8>) -> Self {
        Self { r#type, data }
    }
}

impl Event {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_item(&mut self) {
        self.items.push(Item::new());
    }

    pub fn add_data(&mut self, r#type: String, data: Vec<u8>) {
        if let Some(item) = self.items.last_mut() {
            item.add_data(Data::new(r#type, data));
        }
    }
}
