//! System-wide key state, read straight from evdev.
//!
//! Wayland only gives key events to the focused window, but this is a
//! reference you glance at while typing elsewhere, so it reads the keyboard
//! devices directly. Read-only and never grabbed: other apps see every key as
//! normal. Nothing is logged or stored — the UI only keeps "which keys are
//! down now".
//!
//! Access needs `data/70-keycaps.rules`; without it the devices fail to open
//! and the app degrades to a static reference.

use std::{fs::File, io::Read, path::PathBuf, thread};

use futures::channel::mpsc::UnboundedSender;

#[derive(Debug, PartialEq)]
pub enum KeyEvent {
    Down(u16),
    Up(u16),
    /// The kernel dropped events, so any key may have been released unseen.
    Reset,
}

/// The four devices a Framework 13 spreads its keys across: letters and the
/// F-row on the i8042 keyboard, brightness on the ACPI video bus and the EC's
/// consumer-control HID device, airplane mode on the radio-control device.
const DEFAULT_DEVICES: [&str; 4] = [
    "AT Translated Set 2 keyboard",
    "Video Bus",
    "FRMW0004:00 32AC:0006 Consumer Control",
    "FRMW0004:00 32AC:0006 Wireless Radio Control",
];

/// `KEYCAPS_DEVICES` overrides the device list: comma-separated device names,
/// or absolute paths (which is also how tests feed events through a FIFO).
pub fn configured_devices() -> Vec<String> {
    match std::env::var("KEYCAPS_DEVICES") {
        Ok(list) => list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        Err(_) => DEFAULT_DEVICES.map(String::from).to_vec(),
    }
}

/// Resolve device names to `/dev/input/eventN` via `/proc/bus/input/devices`,
/// whose blocks pair an `N: Name="..."` line with an `H: Handlers=... eventN`.
fn resolve(entries: &[String], proc_devices: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in entries {
        if entry.starts_with('/') {
            paths.push(PathBuf::from(entry));
            continue;
        }
        let wanted = format!("N: Name=\"{entry}\"");
        for block in proc_devices.split("\n\n") {
            if !block.lines().any(|l| l == wanted) {
                continue;
            }
            let handlers = block.lines().find_map(|l| l.strip_prefix("H: Handlers="));
            if let Some(event) = handlers.and_then(|h| h.split_whitespace().find(|w| w.starts_with("event"))) {
                paths.push(PathBuf::from("/dev/input").join(event));
            }
        }
    }
    paths
}

/// `struct input_event` on 64-bit Linux: 16 bytes of timeval, then type, code
/// and value. Only key presses/releases and dropped-event markers matter;
/// autorepeat (value 2) would only re-assert a key already down.
const EVENT_SIZE: usize = 24;
const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const SYN_DROPPED: u16 = 3;

fn parse(raw: &[u8; EVENT_SIZE]) -> Option<KeyEvent> {
    let kind = u16::from_ne_bytes([raw[16], raw[17]]);
    let code = u16::from_ne_bytes([raw[18], raw[19]]);
    let value = i32::from_ne_bytes([raw[20], raw[21], raw[22], raw[23]]);
    match (kind, value) {
        (EV_KEY, 1) => Some(KeyEvent::Down(code)),
        (EV_KEY, 0) => Some(KeyEvent::Up(code)),
        (EV_SYN, _) if code == SYN_DROPPED => Some(KeyEvent::Reset),
        _ => None,
    }
}

/// Open every configured device and start one blocking reader thread per
/// device. Returns the devices that could not be opened, with the reason, so
/// the caller can say why detection is off.
pub fn start(tx: UnboundedSender<KeyEvent>) -> Vec<String> {
    let proc_devices = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
    let entries = configured_devices();
    let paths = resolve(&entries, &proc_devices);
    let mut failures = Vec::new();
    if paths.is_empty() {
        failures.push(format!("none of {entries:?} found"));
    }
    for path in paths {
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                failures.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        let tx = tx.clone();
        thread::spawn(move || {
            let mut raw = [0u8; EVENT_SIZE];
            // Ends on EOF (a test FIFO closing) or ENODEV (device removed),
            // and when the UI side of the channel is gone.
            while file.read_exact(&mut raw).is_ok() {
                if let Some(event) = parse(&raw) {
                    if tx.unbounded_send(event).is_err() {
                        return;
                    }
                }
            }
            eprintln!("keycaps: stopped reading {}", path.display());
        });
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(kind: u16, code: u16, value: i32) -> [u8; EVENT_SIZE] {
        let mut raw = [0u8; EVENT_SIZE];
        raw[16..18].copy_from_slice(&kind.to_ne_bytes());
        raw[18..20].copy_from_slice(&code.to_ne_bytes());
        raw[20..24].copy_from_slice(&value.to_ne_bytes());
        raw
    }

    #[test]
    fn parses_key_events() {
        assert_eq!(parse(&raw(EV_KEY, 30, 1)), Some(KeyEvent::Down(30)));
        assert_eq!(parse(&raw(EV_KEY, 30, 0)), Some(KeyEvent::Up(30)));
        assert_eq!(parse(&raw(EV_KEY, 30, 2)), None, "autorepeat is ignored");
        assert_eq!(parse(&raw(EV_SYN, 0, 0)), None, "SYN_REPORT is ignored");
        assert_eq!(parse(&raw(EV_SYN, SYN_DROPPED, 0)), Some(KeyEvent::Reset));
        assert_eq!(parse(&raw(4, 4, 30)), None, "EV_MSC scancodes are ignored");
    }

    #[test]
    fn resolves_names_and_paths() {
        let proc = "I: Bus=0011\nN: Name=\"AT Translated Set 2 keyboard\"\nH: Handlers=sysrq kbd leds event2 \n\n\
                    I: Bus=0019\nN: Name=\"Video Bus\"\nH: Handlers=kbd event3 \n";
        let entries = ["Video Bus", "/tmp/fifo", "Missing", "AT Translated Set 2 keyboard"].map(String::from);
        let paths = resolve(&entries, proc);
        assert_eq!(paths, ["/dev/input/event3", "/tmp/fifo", "/dev/input/event2"].map(PathBuf::from));
    }
}
