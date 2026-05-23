use evdev::{Device, EventType, Key};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use gdk::WindowTypeHint;
use std::fs;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use pulldown_cmark::{Parser, Event, Tag, TagEnd};

fn markdown_to_pango(markdown_input: &str) -> String {
    let parser = Parser::new(markdown_input);
    let mut pango_output = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Make headings larger based on their level
                match level {
                    pulldown_cmark::HeadingLevel::H1 => pango_output.push_str("<span weight=\"bold\" size=\"xx-large\">"),
                    pulldown_cmark::HeadingLevel::H2 => pango_output.push_str("<span weight=\"bold\" size=\"x-large\">"),
                    _ => pango_output.push_str("<span weight=\"bold\" size=\"large\">"),
                }
            }
            Event::End(TagEnd::Heading(_)) => pango_output.push_str("</span>\n\n"),
            
            Event::Start(Tag::Strong) => pango_output.push_str("<span weight=\"bold\">"),
            Event::End(TagEnd::Strong) => pango_output.push_str("</span>"),
            
            Event::Start(Tag::Emphasis) => pango_output.push_str("<span style=\"italic\">"),
            Event::End(TagEnd::Emphasis) => pango_output.push_str("</span>"),

            Event::Start(Tag::Paragraph) => {
                pango_output.push_str("<p>");
            }

            Event::End(TagEnd::Paragraph) => {
                pango_output.push_str("</p>\n\n");
            }

            Event::Start(Tag::Item) => {
                pango_output.push_str("• ");
            }
            Event::End(TagEnd::Item) => {
                pango_output.push_str("\n");
            }

            Event::SoftBreak => pango_output.push_str("\n"),
            Event::HardBreak => pango_output.push_str("\n"),

            Event::Text(text) => {
                // This safely turns raw "&" into "&amp;", "<" into "&lt;", etc.
                let escaped_text = glib::markup_escape_text(&text);
                pango_output.push_str(&escaped_text);
            }

            Event::Code(text) => {
                // 1. Clean the text inside the backticks so special characters don't break Pango
                let escaped_text = glib::markup_escape_text(&text).replace("&apos;", "'");
                
                // 2. Wrap it in a monospace font span (no black background!)
                let pango_code = format!("<span font_family=\"monospace\">{}</span>", escaped_text);
                
                pango_output.push_str(&pango_code);
            }

            // 2. Fix for multi-line code blocks (triple backticks)
            Event::Start(Tag::CodeBlock(_)) => {
                pango_output.push_str("<span font_family=\"monospace\">");
            }
            Event::End(TagEnd::CodeBlock) => {
                pango_output.push_str("</span>\n");
            }

            _ => {} // Skip items like images or raw HTML for simplicity

            
        }
    }
    pango_output
}

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
    

    for mut device in find_keyboards() {
        let sender = sender.clone();
        let ctrl_held = Arc::new(Mutex::new(false));
        let visible = Arc::new(Mutex::new(false));

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

                    let ctrl = *ctrl_held.lock().unwrap();

                    if key == Key::KEY_K && ctrl && value == 1 {
                        let _ = sender.send(WindowCmd::Show);
                        *visible.lock().unwrap() = true;
                    } 
                    
                    if (key == Key::KEY_K || key == Key::KEY_LEFTCTRL) && *visible.lock().unwrap() && value == 0 {
                        let _ = sender.send(WindowCmd::Hide);
                        *visible.lock().unwrap() = false;
                    }
                }
            }
        });
    }
}

fn main() {
    println!("App Running");

    

    let app = Application::builder()
        .application_id("com.example.splash")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Keybind Flash")
            .default_width(600)
            .default_height(300)
            .decorated(false)
            .resizable(false)
            .build();

        window.set_type_hint(WindowTypeHint::Splashscreen);
        window.set_position(gtk::WindowPosition::Center);

        let md_content: &'static str = include_str!("../assets/keybinds.md");

        // 2. Convert to GTK markup format
        let pango_markup = markdown_to_pango(&md_content);

        // 3. Create a TextView widget to render the text
        let text_view = gtk::TextView::new();
        text_view.set_editable(false); // Make it a read-only document viewer
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(gtk::WrapMode::Word);

        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(b"textview, text { background-color: transparent; }")
            .unwrap();

        text_view.style_context().add_provider(
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // 4. Inject the markup into the text buffer
        if let Some(buffer) = text_view.buffer() {
            // Create an anonymous tag table to hold the styles
            let mut iter = buffer.start_iter();
            buffer.insert_markup(&mut iter, &pango_markup);
        }

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        vbox.set_border_width(20);
        vbox.pack_start(&text_view, true, true, 0);

        window.add(&vbox);
        window.hide();

        window.connect_delete_event(|win, _| {
            // Hide the window instead of destroying it
            win.hide();
            
            // CRITICAL: Return Propagation::Stop (or true in older gtk-rs versions) 
            // to stop GTK from proceeding with the default "destroy" signal.
            Inhibit(true)
        });

        let (sender, receiver) = mpsc::channel::<WindowCmd>();
        monitor_keys(sender);

        let window_clone = window.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
            let mut last_cmd = None;

            // 1. Drain the channel completely to get the absolute newest state
            while let Ok(cmd) = receiver.try_recv() {
                last_cmd = Some(cmd);
            }

            // 2. Only execute the final UI command once per 10ms frame
            if let Some(cmd) = last_cmd {
                match cmd {
                    WindowCmd::Show => {
                        // Check if it's already visible to prevent redundant state triggering
                        if !window_clone.is_visible() {
                            window_clone.show_all();
                            window_clone.present();
                        }
                    }
                    WindowCmd::Hide => {
                        if window_clone.is_visible() {
                            window_clone.hide();
                        }
                    }
                }
            }

            glib::ControlFlow::Continue
        });
    });
    app.hold();
    app.run();
}