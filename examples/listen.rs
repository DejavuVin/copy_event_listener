use paste_stack::clipboard::ClipboardListener;
use paste_stack::event::Event;

fn on_clipboard_event(event: Event) {
    println!("{:?}", event);
}

fn main() {
    let listener = ClipboardListener::new().with_interval(1000);
    listener.run(on_clipboard_event);
}
