use evdev::{Device, EventType, Key};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gdk::WindowTypeHint;
use std::fs;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

fn find_keyboards() -> Vec<Device> {
    let mut keyboards = Vec::new();
    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            if let Ok(device) = Device::open(entry.path()) {
                if device.supported_keys().map_or(false, |k| k.contains(Key::KEY_ESC)) {
                    keyboards.push(device);
                }
            }
        }
    }
    keyboards
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WindowCmd {
    Show,
    Hide,
}

fn monitor_keys(sender: mpsc::Sender<WindowCmd>) {
    let ctrl_held = Arc::new(Mutex::new(false));

    for mut device in find_keyboards() {
        let sender = sender.clone();
        let ctrl_held = ctrl_held.clone();

        thread::spawn(move || loop {
            if let Ok(events) = device.fetch_events() {
                for event in events {
                    if event.event_type() != EventType::KEY {
                        continue;
                    }

                    let key = Key::new(event.code());
                    let value = event.value();

                    if key == Key::KEY_LEFTCTRL || key == Key::KEY_RIGHTCTRL {
                        *ctrl_held.lock().unwrap() = value == 1;
                    }

                    if value != 1 {
                        continue;
                    }

                    let ctrl = *ctrl_held.lock().unwrap();

                    if key == Key::KEY_K && ctrl {
                        let _ = sender.send(WindowCmd::Show);
                    } else if key == Key::KEY_ESC {
                        let _ = sender.send(WindowCmd::Hide);
                    }
                }
            }
        });
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.splash")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Splash")
            .default_width(600)
            .default_height(300)
            .decorated(false)
            .resizable(false)
            .build();

        window.set_type_hint(WindowTypeHint::Splashscreen);
        window.set_position(gtk::WindowPosition::Center);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.set_border_width(20);

        let label = gtk::Label::new(Some("Hello, GTK on Linux!"));
        vbox.pack_start(&label, true, true, 0);

        let button = gtk::Button::with_label("Click Me");
        button.connect_clicked(|_| println!("Button clicked!"));
        vbox.pack_start(&button, false, false, 0);

        window.add(&vbox);
        window.hide();

        let (sender, receiver) = mpsc::channel::<WindowCmd>();
        monitor_keys(sender);

        // Poll the channel on the GTK main loop using a timeout
        let window_clone = window.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
            while let Ok(cmd) = receiver.try_recv() {
                match cmd {
                    WindowCmd::Show => {
                        window_clone.show_all();
                        window_clone.present();
                    }
                    WindowCmd::Hide => {
                        window_clone.hide();
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    });

    app.hold();
    app.run();
}