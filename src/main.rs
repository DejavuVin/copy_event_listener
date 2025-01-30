mod clipboard_content;
mod database;

use std::io;

use clipboard_content::{ClipboardEvent, ClipboardStorage};
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use objc2::rc::{autoreleasepool, Id, Retained};
use objc2::runtime::ProtocolObject;
use objc2_app_kit::NSPasteboard;
use objc2_app_kit::NSPasteboardWriting;
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
            None => {
                return Ok(());
            }
            Some(items) => items,
        };

        // !NOTE:
        // This method will gets more "apple-style" data from copy clipboard,
        // for example: `NSStringPboardType`, `Apple HTML pasteboard type`, `CorePasteboardFlavorType`, `NSFilenamesPboardType`, `com.apple.icns`... .
        // But, This method cannot get all the contents of the clipboard. (especially: multiple files or folders, this method can only get the public.file-url of the first file.)
        // And `pasteboard.pasteboardItems()` (Current methods used) is enough to stores user copied data.
        // ```
        // match unsafe { pasteboard.types() } {
        //     None => (),
        //     Some(types) => {
        //         for ele in types {
        //             let data = unsafe { pasteboard.dataForType(&ele) };
        //             match data {
        //                 None => (),
        //                 Some(data) => {
        //                     let bytes = data.bytes().to_vec();
        //                     println!("data.len for type {:?} : {:?}", ele, bytes.len());
        //                 }
        //             }
        //         }
        //     }
        // }
        // ```

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
                        content.add_type(pb_type.to_string(), bytes);
                    }
                }
            }
        }

        content.finalize_event().map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn set_clipboard_event(event: ClipboardEvent) -> Result<(), String> {
    autoreleasepool(|_| {
        let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
        unsafe { pasteboard.clearContents() };

        let mut writing_items = Vec::new();

        for item in event.items {
            let pb_item = unsafe { objc2_app_kit::NSPasteboardItem::new() };

            for type_data in item.types {
                let data = unsafe { objc2_foundation::NSData::with_bytes(&type_data.data) };
                let type_str = objc2_foundation::NSString::from_str(&type_data.uti);

                let success = unsafe { pb_item.setData_forType(&data, &type_str) };
                if !success {
                    return Err(format!("Failed to set data for type: {}", type_data.uti));
                }
            }

            // Cast NSPasteboardItem to NSPasteboardWriting protocol object and wrap in Retained
            let writing_item: Retained<ProtocolObject<dyn NSPasteboardWriting>> =
                unsafe { std::mem::transmute(pb_item) };
            writing_items.push(writing_item);
        }

        let items = unsafe { objc2_foundation::NSArray::from_vec(writing_items) };
        let success = unsafe { pasteboard.writeObjects(&items) };
        if !success {
            return Err("Failed to write objects to pasteboard".to_string());
        }

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

    // let handler = Handler {
    //     pasteboard: unsafe { NSPasteboard::generalPasteboard() },
    //     conn,
    // };

    // let mut master = Master::new(handler).unwrap();
    // match master.run() {
    //     Err(e) => {
    //         notify_error("paste_stack: clipboard monitor error", &e.to_string());
    //     }
    //     _ => (),
    // }

    let storage = ClipboardStorage::new(&conn);
    let event = storage.restore_event_by_id(2).unwrap();
    event.items.iter().for_each(|item| {
        item.types.iter().for_each(|t| {
            println!("{:?}", t.uti);
        });
    });

    set_clipboard_event(event).unwrap();
    println!("set clipboard event");
}
