mod clipboard_content;

use std::io::{self};

use clipboard_content::ClipboardContent;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use notify_rust::Notification;
use objc2::rc::Id;
use objc2_app_kit::NSPasteboard;

struct Handler {
    pasteboard: Id<NSPasteboard>,
}

impl<'a> ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        let contents = get_clipboard_contents(&self.pasteboard);
        match contents {
            Ok(contents) => {
                println!("🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣🤣");
                println!("contents length: {}", contents.len());
                for c in contents {
                    // println!("count: {:?}, {}", c.len(), c.display());
                    c.display_all();
                    println!("count: {}", c.len());
                }
                CallbackResult::Next
            }
            Err(e) => CallbackResult::StopWithError(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn sleep_interval(&self) -> core::time::Duration {
        core::time::Duration::from_millis(300)
    }
}

fn get_clipboard_contents(pasteboard: &Id<NSPasteboard>) -> Result<Vec<ClipboardContent>, String> {
    let items = unsafe { pasteboard.pasteboardItems() };
    if let None = items {
        return Err(String::from("Failed to get pasteboard items"));
    }
    let items = items.unwrap();

    let mut contents = Vec::new();
    if items.is_empty() {
        return Ok(contents);
    }

    for item in items {
        let mut content = ClipboardContent::new();
        let types = unsafe { item.types() };
        for pb_type in types {
            let uti = pb_type.to_string();
            match unsafe { item.dataForType(&pb_type) } {
                None => {
                    println!("type: {:?}, data is None", uti);
                    continue;
                }
                Some(data) => {
                    let bytes = data.bytes().to_vec();
                    content.on_data(uti, bytes);
                }
            }
        }
        contents.push(content);
    }

    Ok(contents)
}

fn main() {
    let handler = Handler {
        pasteboard: unsafe { NSPasteboard::generalPasteboard() },
    };

    let mut master = Master::new(handler).unwrap();
    match master.run() {
        Ok(_) => (),
        Err(e) => {
            Notification::new()
                .summary("Clipboard Monitor Error during running")
                .body(&e.to_string())
                .show()
                .unwrap();
        }
    };
}
