import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk, GLib
import threading
import os
import signal
import evdev
from evdev import ecodes
import select
import sys


def quit_app():
    os.kill(os.getpid(), signal.SIGKILL)


def monitor_keys():
    all_devices = [evdev.InputDevice(path) for path in evdev.list_devices()]

    # Print all found devices to help debug
    print(f"Found {len(all_devices)} input devices:", file=sys.stderr)
    for d in all_devices:
        print(f"  {d.path}: {d.name}", file=sys.stderr)

    # Accept any device that has KEY_ESC capability
    keyboards = [d for d in all_devices if ecodes.KEY_ESC in d.capabilities().get(ecodes.EV_KEY, [])]

    print(f"Keyboards found: {[d.name for d in keyboards]}", file=sys.stderr)

    if not keyboards:
        print("No keyboards found! Try running with sudo or check input group membership.", file=sys.stderr)
        return

    fd_map = {dev.fd: dev for dev in keyboards}

    while True:
        r, _, _ = select.select(fd_map, [], [])
        for fd in r:
            dev = fd_map[fd]
            for event in dev.read():
                if event.type == ecodes.EV_KEY and event.value == 1:
                    if event.code in (ecodes.KEY_ESC, ecodes.KEY_Q):
                        quit_app()


listener_thread = threading.Thread(target=monitor_keys, daemon=True)
listener_thread.start()


class SplashWindow(Gtk.Window):
    def __init__(self):
        super().__init__(title="My GTK Window")

        self.set_default_size(600, 300)
        self.set_border_width(20)
        self.set_type_hint(Gdk.WindowTypeHint.SPLASHSCREEN)
        self.set_decorated(False)
        self.set_resizable(False)
        self.set_position(Gtk.WindowPosition.CENTER)

        self.connect("realize", self.on_realize)
        self.connect("destroy", Gtk.main_quit)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        self.add(vbox)

        label = Gtk.Label(label="Hello, GTK on Linux!")
        vbox.pack_start(label, expand=True, fill=True, padding=0)

        button = Gtk.Button(label="Click Me")
        button.connect("clicked", self.on_button_clicked)
        vbox.pack_start(button, expand=False, fill=False, padding=0)

    def on_realize(self, widget):
        self.get_window().set_decorations(Gdk.WMDecoration(0))

    def on_button_clicked(self, widget):
        print("Button clicked!")


if __name__ == "__main__":
    win = SplashWindow()
    win.show_all()
    Gtk.main()