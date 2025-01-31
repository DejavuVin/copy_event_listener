// 10.7 or later
use crate::event::Event;
use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::ProtocolObject,
};
use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSArray, NSData, NSString};
use std::time::Duration;

pub struct ClipboardListener {
    pasteboard: Retained<NSPasteboard>,
    interval_duration: Duration,
}

impl ClipboardListener {
    pub fn new() -> Self {
        let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
        Self {
            pasteboard,
            interval_duration: Duration::from_millis(500),
        }
    }

    pub fn with_interval(mut self, mut interval_millis: u64) -> Self {
        if interval_millis == 0 {
            interval_millis = 500;
        }
        self.interval_duration = Duration::from_millis(interval_millis);
        self
    }

    pub fn run(self, on_event: impl Fn(Event)) {
        let mut prev_count = unsafe { self.pasteboard.changeCount() };

        loop {
            autoreleasepool(|_| {
                let count = unsafe { self.pasteboard.changeCount() };
                if count == prev_count {
                    std::thread::sleep(self.interval_duration);
                    return;
                }
                prev_count = count;

                match unsafe { self.pasteboard.pasteboardItems() } {
                    None => (),
                    Some(items) => {
                        let mut event = Event::new();
                        for item in items {
                            event.new_item();
                            let types = unsafe { item.types() };
                            for pb_type in types {
                                match unsafe { item.dataForType(&pb_type) } {
                                    None => continue,
                                    Some(data) => {
                                        let bytes = data.bytes().to_vec();
                                        event.add_data(pb_type.to_string(), bytes);
                                    }
                                }
                            }
                        }
                        on_event(event);
                    }
                };
            });
        }
    }

    pub fn set_clipboard_event(self, event: Event) -> Result<(), String> {
        autoreleasepool(|_| {
            unsafe { self.pasteboard.clearContents() };

            let mut writing_items = Vec::new();

            for item in event.items {
                let pb_item = unsafe { objc2_app_kit::NSPasteboardItem::new() };

                for type_data in item.data_list {
                    let data = NSData::with_bytes(&type_data.data);
                    let type_str = NSString::from_str(&type_data.r#type);

                    let success = unsafe { pb_item.setData_forType(&data, &type_str) };
                    if !success {
                        return Err(format!("Failed to set data for type: {}", type_data.r#type));
                    }
                }

                writing_items.push(ProtocolObject::from_retained(pb_item));
            }

            let items = NSArray::from_vec(writing_items);
            let success = unsafe { self.pasteboard.writeObjects(&items) };
            if !success {
                return Err("Failed to write objects to pasteboard".to_string());
            }

            Ok(())
        })
    }
}
