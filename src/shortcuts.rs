//! GNOME's global shortcuts, read from gsettings, keyed by the physical key
//! and modifier combination that triggers them.
//!
//! Only global shortcuts are shown. App shortcuts (Ctrl+W, …) mean different
//! things in different apps, so a single legend for them would often lie.

use std::{collections::HashMap, process::Command};

use crate::layout::code;

/// A held-modifier combination. Left and right variants count the same.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Mods(u8);

impl std::fmt::Debug for Mods {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let names = [(Mods::SHIFT, "Shift"), (Mods::CTRL, "Ctrl"), (Mods::ALT, "Alt"), (Mods::SUPER, "Super")];
        let held: Vec<_> = names.iter().filter(|(m, _)| self.0 & m.0 != 0).map(|(_, n)| *n).collect();
        write!(f, "{:<16}", if held.is_empty() { "-".to_string() } else { held.join("+") })
    }
}

impl Mods {
    pub const NONE: Mods = Mods(0);
    pub const SHIFT: Mods = Mods(1);
    pub const CTRL: Mods = Mods(2);
    pub const ALT: Mods = Mods(4);
    pub const SUPER: Mods = Mods(8);

    pub fn with(self, other: Mods) -> Mods {
        Mods(self.0 | other.0)
    }

    /// Which modifier a key code is, if any.
    pub fn of(key: u16) -> Option<Mods> {
        match key {
            code::LEFTSHIFT | code::RIGHTSHIFT => Some(Mods::SHIFT),
            code::LEFTCTRL | code::RIGHTCTRL => Some(Mods::CTRL),
            code::LEFTALT | code::RIGHTALT => Some(Mods::ALT),
            code::LEFTMETA | code::RIGHTMETA => Some(Mods::SUPER),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Shortcuts {
    by_key: HashMap<(Mods, u16), String>,
}

impl Shortcuts {
    /// Every binding as `mods code label`, sorted, for `keycaps --shortcuts`.
    pub fn dump(&self) -> Vec<String> {
        let mut lines: Vec<_> =
            self.by_key.iter().map(|((m, c), l)| format!("{m:?} {c:>3} {l}")).collect();
        lines.sort();
        lines
    }

    pub fn label(&self, mods: Mods, key: u16) -> Option<&str> {
        self.by_key.get(&(mods, key)).map(String::as_str)
    }

    /// Earlier sources win, so custom shortcuts shadow built-in ones.
    fn add(&mut self, accel: &str, label: &str) {
        if let Some(combo) = parse_accel(accel) {
            self.by_key.entry(combo).or_insert_with(|| label.to_string());
        }
    }
}

/// Searched in this order; the first binding of a combination wins.
const MEDIA_KEYS: &str = "org.gnome.settings-daemon.plugins.media-keys";
const SCHEMAS: [&str; 5] = [
    "org.gnome.shell.keybindings",
    "org.gnome.desktop.wm.keybindings",
    "org.gnome.mutter.keybindings",
    "org.gnome.mutter.wayland.keybindings",
    MEDIA_KEYS,
];
const CUSTOM_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

/// Short legends for GNOME's action ids. `#` stands for a trailing number.
const LABELS: &str = include_str!("../data/labels.tsv");

/// Reads everything at startup; shortcut changes show up on the next launch.
/// A missing `gsettings` leaves the map empty and the layers blank.
pub fn load() -> Shortcuts {
    let labels = parse_labels(LABELS);
    let mut shortcuts = Shortcuts::default();

    let custom_paths = quoted(&gsettings(&["get", MEDIA_KEYS, "custom-keybindings"]));
    for path in custom_paths {
        let listing = gsettings(&["list-recursively", &format!("{CUSTOM_SCHEMA}:{path}")]);
        let fields: HashMap<_, _> = parse_listing(&listing).into_iter().collect();
        if let (Some(name), Some(binding)) = (fields.get("name"), fields.get("binding")) {
            if let (Some(name), Some(binding)) = (name.first(), binding.first()) {
                shortcuts.add(binding, name);
            }
        }
    }

    for schema in SCHEMAS {
        for (id, accels) in parse_listing(&gsettings(&["list-recursively", schema])) {
            let label = label_for(&id, &labels);
            for accel in accels {
                shortcuts.add(&accel, &label);
            }
        }
    }
    shortcuts
}

fn gsettings(args: &[&str]) -> String {
    match Command::new("gsettings").args(args).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        Ok(out) => {
            eprintln!("keycaps: gsettings {args:?}: {}", String::from_utf8_lossy(&out.stderr).trim());
            String::new()
        }
        Err(err) => {
            eprintln!("keycaps: gsettings: {err}");
            String::new()
        }
    }
}

/// `list-recursively` prints `schema key value` per line; values here are
/// string arrays like `['<Super>l', '']` or single strings like `'Name'`.
fn parse_listing(listing: &str) -> Vec<(String, Vec<String>)> {
    listing
        .lines()
        .filter_map(|line| {
            let (_schema, rest) = line.split_once(' ')?;
            let (key, value) = rest.split_once(' ')?;
            Some((key.to_string(), quoted(value)))
        })
        .collect()
}

/// The single-quoted strings in a GVariant text value, unescaped.
fn quoted(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\'' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => s.extend(chars.next()),
                '\'' => break,
                c => s.push(c),
            }
        }
        out.push(s);
    }
    out
}

/// `<Shift><Super>Page_Down` → (Shift+Super, code). `None` for bindings to
/// keys this board doesn't have (Page_Down, keypad, XF86Calculator…) and for
/// modifiers it can't detect (Hyper, Meta).
fn parse_accel(accel: &str) -> Option<(Mods, u16)> {
    let mut mods = Mods::NONE;
    let mut rest = accel;
    while let Some(tail) = rest.strip_prefix('<') {
        let (name, tail) = tail.split_once('>')?;
        mods = mods.with(match name.to_ascii_lowercase().as_str() {
            "shift" => Mods::SHIFT,
            "control" | "ctrl" | "primary" => Mods::CTRL,
            "alt" | "mod1" => Mods::ALT,
            "super" | "mod4" => Mods::SUPER,
            _ => return None,
        });
        rest = tail;
    }
    Some((mods, keysym_code(rest)?))
}

/// XKB keysym name → evdev code, for the US layout on this board.
fn keysym_code(keysym: &str) -> Option<u16> {
    use code::*;
    let lower = keysym.to_ascii_lowercase();
    let mut chars = lower.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if let Some(code) = letter(c).or_else(|| digit(c)) {
            return Some(code);
        }
    }
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
        return match n {
            1..=10 => Some(F1 + n - 1),
            11 => Some(F11),
            12 => Some(F12),
            _ => None,
        };
    }
    Some(match lower.as_str() {
        "escape" => ESC,
        "tab" => TAB,
        "return" => ENTER,
        "space" => SPACE,
        "backspace" => BACKSPACE,
        "delete" => DELETE,
        "insert" => INSERT,
        "left" => LEFT,
        "right" => RIGHT,
        "up" => UP,
        "down" => DOWN,
        "print" => SYSRQ,
        "grave" | "above_tab" => GRAVE,
        "minus" => MINUS,
        "equal" => EQUAL,
        "bracketleft" => LEFTBRACE,
        "bracketright" => RIGHTBRACE,
        "backslash" => BACKSLASH,
        "semicolon" => SEMICOLON,
        "apostrophe" => APOSTROPHE,
        "comma" => COMMA,
        "period" => DOT,
        "slash" => SLASH,
        "xf86audiomute" => MUTE,
        "xf86audiolowervolume" => VOLUMEDOWN,
        "xf86audioraisevolume" => VOLUMEUP,
        "xf86audioprev" => PREVIOUSSONG,
        "xf86audioplay" => PLAYPAUSE,
        "xf86audionext" => NEXTSONG,
        "xf86monbrightnessdown" => BRIGHTNESSDOWN,
        "xf86monbrightnessup" => BRIGHTNESSUP,
        "xf86rfkill" => RFKILL,
        "xf86audiomedia" => MEDIA,
        _ => return None,
    })
}

fn parse_labels(tsv: &str) -> HashMap<&str, &str> {
    tsv.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect()
}

/// `switch-to-application-3` finds `switch-to-application-#` → "App 3".
/// Unlisted ids fall back to the id itself, made readable.
fn label_for(id: &str, labels: &HashMap<&str, &str>) -> String {
    let id = id.strip_suffix("-static").unwrap_or(id);
    if let Some(label) = labels.get(id) {
        return label.to_string();
    }
    if let Some((base, n)) = id.rsplit_once('-').filter(|(_, n)| n.parse::<u32>().is_ok()) {
        if let Some(label) = labels.get(format!("{base}-#").as_str()) {
            return label.replace('#', n);
        }
    }
    let readable = id.replace('-', " ");
    let mut chars = readable.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_accelerators() {
        assert_eq!(parse_accel("<Super>l"), Some((Mods::SUPER, 38)));
        assert_eq!(parse_accel("<Shift><Super>Above_Tab"), Some((Mods::SHIFT.with(Mods::SUPER), code::GRAVE)));
        assert_eq!(parse_accel("<Primary><Alt>F10"), Some((Mods::CTRL.with(Mods::ALT), code::F1 + 9)));
        assert_eq!(parse_accel("<Super>XF86MonBrightnessUp"), Some((Mods::SUPER, code::BRIGHTNESSUP)));
        assert_eq!(parse_accel("Print"), Some((Mods::NONE, code::SYSRQ)));
        assert_eq!(parse_accel("<Super>Page_Down"), None, "no such key on this board");
        assert_eq!(parse_accel("<Hyper>a"), None);
        assert_eq!(parse_accel(""), None);
    }

    #[test]
    fn parses_gsettings_listing() {
        let listing = "org.gnome.shell.keybindings screenshot ['<Shift>Print']\n\
                       org.gnome.shell.keybindings toggle-message-tray ['<Super>v', '<Super>m']\n\
                       org.gnome.wm help ['', '<Super>F1']\n\
                       x.custom-keybinding name 'Bob\\'s thing'\n\
                       x.custom-keybinding binding @as []\n";
        let parsed = parse_listing(listing);
        assert_eq!(parsed[0], ("screenshot".into(), vec!["<Shift>Print".into()]));
        assert_eq!(parsed[1].1, ["<Super>v", "<Super>m"]);
        assert_eq!(parsed[2].1, ["", "<Super>F1"]);
        assert_eq!(parsed[3].1, ["Bob's thing"]);
        assert!(parsed[4].1.is_empty());
    }

    #[test]
    fn labels_resolve() {
        let labels = parse_labels(LABELS);
        assert_eq!(label_for("screensaver", &labels), "Lock");
        assert_eq!(label_for("switch-to-application-3", &labels), "App 3");
        assert_eq!(label_for("volume-up-static", &labels), "Vol +");
        assert_eq!(label_for("some-new-action", &labels), "Some new action");
    }

    #[test]
    fn first_binding_wins() {
        let mut s = Shortcuts::default();
        s.add("<Super>XF86MonBrightnessUp", "Studio +");
        s.add("<Super>XF86MonBrightnessUp", "Other");
        assert_eq!(s.label(Mods::SUPER, code::BRIGHTNESSUP), Some("Studio +"));
    }
}
