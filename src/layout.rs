//! Physical description of one keyboard.
//!
//! This is the whole "model" of the app: a keyboard is rows of keys, and a key
//! is up to three legends, a width, and the evdev codes it emits. Nothing here
//! knows about rendering.
//!
//! Widths are in *units*, where 1u is a normal alphanumeric key. Every row
//! sums to 15u, which is what makes the rows line up without any measuring.
//!
//! The Fn legends were captured from evdev on the actual machine rather than
//! read off a spec sheet — see README.md. Two of them never reach the OS at
//! all (Fn+Esc, Fn+Space) because the embedded controller handles them
//! internally; they are listed because the keycap is what we're documenting.

/// A single keycap.
///
/// `shift` and `fn_legend` are separate fields rather than a list because they
/// render differently: shifted symbols sit above the primary legend the way
/// they're printed on a real keycap, while Fn secondaries sit below in an
/// accent colour.
pub struct Key {
    pub main: &'static str,
    pub shift: Option<&'static str>,
    pub fn_legend: Option<&'static str>,
    pub width: f32,
    /// Renders as two half-height caps sharing one column footprint, `main`
    /// above and this below. Only the arrow cluster needs it: ↑ and ↓ split a
    /// single 1u slot, which is why a keyboard drawn on an integer grid can
    /// never be quite right.
    pub stack: Option<Box<Key>>,
    /// evdev code of the primary legend. `None` for Fn, which the embedded
    /// controller never reports.
    pub code: Option<u16>,
    /// evdev code of the Fn secondary. Which of `code`/`fn_code` a bare press
    /// sends depends on Fn lock, so a top-row key lights for either.
    pub fn_code: Option<u16>,
}

impl Key {
    pub fn codes(&self) -> impl Iterator<Item = u16> {
        self.code.into_iter().chain(self.fn_code)
    }

    fn shift(mut self, legend: &'static str) -> Self {
        self.shift = Some(legend);
        self
    }

    fn wide(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Fn secondary. `code` is `None` where the EC swallows the press.
    fn fn_(mut self, legend: &'static str, code: Option<u16>) -> Self {
        self.fn_legend = Some(legend);
        self.fn_code = code;
        self
    }
}

fn k(main: &'static str, code: u16) -> Key {
    Key { main, shift: None, fn_legend: None, width: 1.0, stack: None, code: Some(code), fn_code: None }
}

/// Two half-height caps in one 1u column — the ↑/↓ pair. `top` is drawn
/// above, `bottom` below; each is a full key with its own legends and codes.
fn kstack(top: Key, bottom: Key) -> Key {
    Key { stack: Some(Box::new(bottom)), ..top }
}

/// evdev codes (linux/input-event-codes.h) for every key on this board.
pub mod code {
    pub const ESC: u16 = 1;
    pub const MINUS: u16 = 12;
    pub const EQUAL: u16 = 13;
    pub const BACKSPACE: u16 = 14;
    pub const TAB: u16 = 15;
    pub const LEFTBRACE: u16 = 26;
    pub const RIGHTBRACE: u16 = 27;
    pub const ENTER: u16 = 28;
    pub const LEFTCTRL: u16 = 29;
    pub const SEMICOLON: u16 = 39;
    pub const APOSTROPHE: u16 = 40;
    pub const GRAVE: u16 = 41;
    pub const LEFTSHIFT: u16 = 42;
    pub const BACKSLASH: u16 = 43;
    pub const COMMA: u16 = 51;
    pub const DOT: u16 = 52;
    pub const SLASH: u16 = 53;
    pub const RIGHTSHIFT: u16 = 54;
    pub const LEFTALT: u16 = 56;
    pub const SPACE: u16 = 57;
    pub const CAPSLOCK: u16 = 58;
    pub const F1: u16 = 59; // F1..F10 are contiguous
    pub const F11: u16 = 87;
    pub const F12: u16 = 88;
    pub const RIGHTCTRL: u16 = 97;
    pub const SYSRQ: u16 = 99;
    pub const RIGHTALT: u16 = 100;
    pub const HOME: u16 = 102;
    pub const UP: u16 = 103;
    pub const PAGEUP: u16 = 104;
    pub const LEFT: u16 = 105;
    pub const RIGHT: u16 = 106;
    pub const END: u16 = 107;
    pub const DOWN: u16 = 108;
    pub const PAGEDOWN: u16 = 109;
    pub const INSERT: u16 = 110;
    pub const DELETE: u16 = 111;
    pub const MUTE: u16 = 113;
    pub const VOLUMEDOWN: u16 = 114;
    pub const VOLUMEUP: u16 = 115;
    pub const LEFTMETA: u16 = 125;
    pub const RIGHTMETA: u16 = 126;
    pub const NEXTSONG: u16 = 163;
    pub const PLAYPAUSE: u16 = 164;
    pub const PREVIOUSSONG: u16 = 165;
    pub const BRIGHTNESSDOWN: u16 = 224;
    pub const BRIGHTNESSUP: u16 = 225;
    pub const MEDIA: u16 = 226;
    pub const RFKILL: u16 = 247;

    /// Letter and digit codes follow the physical rows, not the alphabet.
    pub fn letter(c: char) -> Option<u16> {
        const ROWS: [(&str, u16); 3] = [("qwertyuiop", 16), ("asdfghjkl", 30), ("zxcvbnm", 44)];
        let c = c.to_ascii_lowercase();
        ROWS.iter().find_map(|(row, start)| row.find(c).map(|i| start + i as u16))
    }

    /// `1`..`9` are 2..10, `0` is 11.
    pub fn digit(c: char) -> Option<u16> {
        let d = c.to_digit(10)? as u16;
        Some(if d == 0 { 11 } else { d + 1 })
    }
}

/// Framework Laptop 13 (AMD Ryzen AI 300 Series), US ANSI.
///
/// Note what is *absent*: no ISO 102nd key beside the left Shift, and exactly
/// two modifiers right of the space bar. Getting those wrong shifts every
/// subsequent keycap, which on a legend-less keyboard makes the whole drawing
/// lie rather than merely look untidy.
///
/// F9's Fn function has no code: the EC sends Super+P for it, so pressing it
/// lights Super and P, which is the truth about what the OS sees.
pub fn framework13() -> Vec<Vec<Key>> {
    use code::*;
    let letters = |s: &'static str| -> Vec<Key> {
        // Each letter is its own one-byte &'static str slice of the input.
        (0..s.len()).map(|i| k(&s[i..i + 1], letter(s.as_bytes()[i] as char).unwrap())).collect()
    };
    let digits = ["1!", "2@", "3#", "4$", "5%", "6^", "7&", "8*", "9(", "0)"]
        .map(|p| k(&p[..1], digit(p.as_bytes()[0] as char).unwrap()).shift(&p[1..]));

    vec![
        vec![
            k("Esc", ESC).wide(1.5).fn_("FnLk", None),
            k("F1", F1).fn_("Mute", Some(MUTE)),
            k("F2", F1 + 1).fn_("Vol −", Some(VOLUMEDOWN)),
            k("F3", F1 + 2).fn_("Vol +", Some(VOLUMEUP)),
            k("F4", F1 + 3).fn_("Prev", Some(PREVIOUSSONG)),
            k("F5", F1 + 4).fn_("Play", Some(PLAYPAUSE)),
            k("F6", F1 + 5).fn_("Next", Some(NEXTSONG)),
            k("F7", F1 + 6).fn_("Bright −", Some(BRIGHTNESSDOWN)),
            k("F8", F1 + 7).fn_("Bright +", Some(BRIGHTNESSUP)),
            k("F9", F1 + 8).fn_("Display", None),
            k("F10", F1 + 9).fn_("Airplane", Some(RFKILL)),
            k("F11", F11).fn_("PrtSc", Some(SYSRQ)),
            k("F12", F12).fn_("Media", Some(MEDIA)),
            k("Del", DELETE).wide(1.5).fn_("Insert", Some(INSERT)),
        ],
        [
            vec![k("`", GRAVE).shift("~")],
            digits.into(),
            vec![
                k("-", MINUS).shift("_"),
                k("=", EQUAL).shift("+"),
                k("Backspace", BACKSPACE).wide(2.0),
            ],
        ]
        .into_iter().flatten().collect(),
        [
            vec![k("Tab", TAB).wide(1.5)],
            letters("QWERTYUIOP"),
            vec![
                k("[", LEFTBRACE).shift("{"),
                k("]", RIGHTBRACE).shift("}"),
                k("\\", BACKSLASH).shift("|").wide(1.5),
            ],
        ]
        .into_iter().flatten().collect(),
        [
            vec![k("Caps", CAPSLOCK).wide(1.75)],
            letters("ASDFGHJKL"),
            vec![
                k(";", SEMICOLON).shift(":"),
                k("'", APOSTROPHE).shift("\""),
                k("Enter", ENTER).wide(2.25),
            ],
        ]
        .into_iter().flatten().collect(),
        [
            vec![k("Shift", LEFTSHIFT).wide(2.25)],
            letters("ZXCVBNM"),
            vec![
                k(",", COMMA).shift("<"),
                k(".", DOT).shift(">"),
                k("/", SLASH).shift("?"),
                // Right Shift runs to the edge — nothing sits beside it. The up
                // arrow lives in the bottom row, stacked over the down arrow.
                k("Shift", RIGHTSHIFT).wide(2.75),
            ],
        ]
        .into_iter().flatten().collect(),
        vec![
            k("Ctrl", LEFTCTRL).wide(1.25),
            Key { code: None, ..k("Fn", 0) },
            k("Super", LEFTMETA).wide(1.25),
            k("Alt", LEFTALT).wide(1.25),
            k("", SPACE).wide(5.25).fn_("Backlight", None),
            k("Alt", RIGHTALT),
            k("Ctrl", RIGHTCTRL),
            // Fn+arrows captured from evdev 2026-10-31, same as the F-row.
            k("←", LEFT).fn_("Home", Some(HOME)),
            kstack(k("↑", UP).fn_("PgUp", Some(PAGEUP)), k("↓", DOWN).fn_("PgDn", Some(PAGEDOWN))),
            k("→", RIGHT).fn_("End", Some(END)),
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_15u() {
        for row in framework13() {
            let width: f32 = row.iter().map(|k| k.width).sum();
            assert!((width - 15.0).abs() < 1e-4, "row sums to {width}");
        }
    }

    #[test]
    fn codes_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for key in framework13().iter().flatten() {
            for c in key.codes().chain(key.stack.iter().flat_map(|s| s.codes())) {
                assert!(seen.insert(c), "code {c} on {} is duplicated", key.main);
            }
        }
    }

    #[test]
    fn letters_follow_rows() {
        assert_eq!(code::letter('q'), Some(16));
        assert_eq!(code::letter('L'), Some(38));
        assert_eq!(code::letter('m'), Some(50));
        assert_eq!(code::digit('0'), Some(11));
        assert_eq!(code::digit('1'), Some(2));
    }
}
