//! Physical description of one keyboard.
//!
//! This is the whole "model" of the app: a keyboard is rows of keys, and a key
//! is up to three legends plus a width. Nothing here knows about rendering.
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
    pub stack: Option<&'static str>,
}

/// Plain 1u key.
fn k(main: &'static str) -> Key {
    Key { main, shift: None, fn_legend: None, width: 1.0, stack: None }
}

/// 1u key carrying a shifted legend.
fn ks(main: &'static str, shift: &'static str) -> Key {
    Key { main, shift: Some(shift), fn_legend: None, width: 1.0, stack: None }
}

/// Wider key, no secondary legends.
fn kw(main: &'static str, width: f32) -> Key {
    Key { main, shift: None, fn_legend: None, width, stack: None }
}

/// 1u key carrying an Fn secondary.
fn kf(main: &'static str, fn_legend: &'static str) -> Key {
    Key { main, shift: None, fn_legend: Some(fn_legend), width: 1.0, stack: None }
}

/// Wider key carrying an Fn secondary.
fn kfw(main: &'static str, fn_legend: &'static str, width: f32) -> Key {
    Key { main, shift: None, fn_legend: Some(fn_legend), width, stack: None }
}

/// Two half-height caps in one 1u column — the ↑/↓ pair.
fn kstack(top: &'static str, bottom: &'static str) -> Key {
    Key { main: top, shift: None, fn_legend: None, width: 1.0, stack: Some(bottom) }
}

/// Framework Laptop 13 (AMD Ryzen AI 300 Series), US ANSI.
///
/// Note what is *absent*: no ISO 102nd key beside the left Shift, and exactly
/// two modifiers right of the space bar. Getting those wrong shifts every
/// subsequent keycap, which on a legend-less keyboard makes the whole drawing
/// lie rather than merely look untidy.
pub fn framework13() -> Vec<Vec<Key>> {
    vec![
        vec![
            kfw("Esc", "FnLk", 1.5),
            kf("F1", "Mute"),
            kf("F2", "Vol −"),
            kf("F3", "Vol +"),
            kf("F4", "Prev"),
            kf("F5", "Play"),
            kf("F6", "Next"),
            kf("F7", "Bright −"),
            kf("F8", "Bright +"),
            kf("F9", "Display"),
            kf("F10", "Airplane"),
            kf("F11", "PrtSc"),
            kf("F12", "Media"),
            kfw("Del", "Insert", 1.5),
        ],
        vec![
            ks("`", "~"),
            ks("1", "!"),
            ks("2", "@"),
            ks("3", "#"),
            ks("4", "$"),
            ks("5", "%"),
            ks("6", "^"),
            ks("7", "&"),
            ks("8", "*"),
            ks("9", "("),
            ks("0", ")"),
            ks("-", "_"),
            ks("=", "+"),
            kw("Backspace", 2.0),
        ],
        vec![
            kw("Tab", 1.5),
            k("Q"), k("W"), k("E"), k("R"), k("T"),
            k("Y"), k("U"), k("I"), k("O"), k("P"),
            ks("[", "{"),
            ks("]", "}"),
            Key { main: "\\", shift: Some("|"), fn_legend: None, width: 1.5, stack: None },
        ],
        vec![
            kw("Caps", 1.75),
            k("A"), k("S"), k("D"), k("F"), k("G"),
            k("H"), k("J"), k("K"), k("L"),
            ks(";", ":"),
            ks("'", "\""),
            kw("Enter", 2.25),
        ],
        vec![
            kw("Shift", 2.25),
            k("Z"), k("X"), k("C"), k("V"), k("B"), k("N"), k("M"),
            ks(",", "<"),
            ks(".", ">"),
            ks("/", "?"),
            // Right Shift runs to the edge — nothing sits beside it. The up
            // arrow lives in the bottom row, stacked over the down arrow.
            kw("Shift", 2.75),
        ],
        vec![
            kw("Ctrl", 1.25),
            kw("Fn", 1.0),
            kw("Super", 1.25),
            kw("Alt", 1.25),
            kfw("", "Backlight", 5.25),
            kw("Alt", 1.0),
            kw("Ctrl", 1.0),
            kw("←", 1.0),
            kstack("↑", "↓"),
            kw("→", 1.0),
        ],
    ]
}
