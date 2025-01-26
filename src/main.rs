mod clipboard_content;
mod database;

use std::io;

use clipboard_content::ClipboardStorage;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use objc2::rc::{autoreleasepool, Id};
use objc2_app_kit::NSPasteboard;
use rusqlite::Connection;

struct Handler {
    pasteboard: Id<NSPasteboard>,
    conn: Connection,
}

impl<'a> ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        let contents = get_clipboard_event(&self.pasteboard, &self.conn);
        match contents {
            Ok(_contents) => CallbackResult::Next,
            Err(e) => CallbackResult::StopWithError(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn sleep_interval(&self) -> core::time::Duration {
        core::time::Duration::from_millis(300)
    }
}

fn get_clipboard_event(pasteboard: &Id<NSPasteboard>, conn: &Connection) -> Result<(), String> {
    autoreleasepool(|_| {
        let items = match unsafe { pasteboard.pasteboardItems() } {
            None => return Err(String::from("Failed to get pasteboard items")),
            Some(items) => items,
        };

        let mut content = ClipboardStorage::new(conn);
        content.start_event();

        for item in items {
            content.start_item();
            let types = unsafe { item.types() };
            for pb_type in types {
                match unsafe { item.dataForType(&pb_type) } {
                    None => continue,
                    Some(data) => {
                        let bytes = data.bytes().to_vec();
                        content.add_type(pb_type.to_string(), bytes)?;
                    }
                }
            }
        }

        content.finalize_event().map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn notify_error(title: &str, e: &str) {
    println!("paste_stack: {} {}", title, e);
    // Notification::new().summary(title).body(&e).show().unwrap();
}

fn main() {
    let conn = match database::init_database() {
        Err(e) => {
            notify_error("paste_stack: database init error", &e.to_string());
            return;
        }
        Ok(conn) => conn,
    };

    let handler = Handler {
        pasteboard: unsafe { NSPasteboard::generalPasteboard() },
        conn,
    };

    let mut master = Master::new(handler).unwrap();
    match master.run() {
        Err(e) => {
            notify_error("paste_stack: clipboard monitor error", &e.to_string());
        }
        _ => (),
    }
}
