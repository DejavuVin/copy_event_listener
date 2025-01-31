use copy_event_listener::clipboard::ClipboardListener;
use copy_event_listener::event::Event;

fn on_clipboard_event(event: Event) {
    println!("{:?}", event);
}

fn main() {
    let listener = ClipboardListener::new().with_interval(1000);
    listener.run(on_clipboard_event);
}
