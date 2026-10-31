//! keycaps — an on-screen reference for a keyboard with blank keycaps.
//!
//! At rest it renders every legend at once: shifted symbols above the
//! primary, Fn secondaries below in an accent colour. Keys light up as they
//! are pressed anywhere on the system, and holding modifiers switches the
//! legends to GNOME's global shortcuts for that combination.
//!
//! This is a GPUI pilot. The shape to notice: `layout.rs` is plain data with
//! no framework types in it, and everything below is a pure function from
//! that data plus the current key state to elements.

mod input;
mod layout;
mod shortcuts;

use std::collections::HashSet;

use futures::StreamExt;
// Imported explicitly rather than via `gpui::prelude::*` so it's visible which
// traits the builder syntax actually depends on: Styled for the style helpers,
// ParentElement for .child/.children, AppContext for cx.new.
use gpui::{
    AnyElement, App, AppContext, Application, Bounds, Context, Div, FocusHandle,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, Styled, TitlebarOptions,
    Window, WindowBounds, WindowOptions, actions, div, px, rgb, size,
};

use input::KeyEvent;
use layout::Key;
use shortcuts::{Mods, Shortcuts};

actions!(keycaps, [Close]);

/// Width of a 1u key, in pixels. Everything else is derived from this, so the
/// whole board scales by changing one number.
const UNIT: f32 = 88.0;
const KEY_H: f32 = 88.0;
const GAP: f32 = 6.0;
const PAD: f32 = 20.0;

/// Rows are 15u wide; add the inter-key gaps and the outer padding.
fn board_width() -> f32 {
    15.0 * UNIT + 13.0 * GAP + 2.0 * PAD
}

fn board_height() -> f32 {
    6.0 * KEY_H + 5.0 * GAP + 2.0 * PAD
}

const BG: u32 = 0x1c1c1e;
const KEY_BG: u32 = 0x38383c;
const KEY_BORDER: u32 = 0x4d4d52;
const PRESSED_BG: u32 = 0x24507a;
const PRESSED_BORDER: u32 = 0x6fb6ff;
const MAIN_FG: u32 = 0xf2f2f4;
const SHIFT_FG: u32 = 0x9a9aa2;
const DIM_FG: u32 = 0x5c5c63;
const FN_FG: u32 = 0x6fb6ff;
const WARN_FG: u32 = 0xe0a040;

struct Keyboard {
    rows: Vec<Vec<Key>>,
    /// evdev codes currently held, across every keyboard device.
    pressed: HashSet<u16>,
    shortcuts: Shortcuts,
    /// Why key detection is off, if it is. Shown along the bottom edge.
    input_failures: Vec<String>,
    /// Actions only reach an element that holds focus, so the root needs a
    /// handle even though nothing here is interactive.
    focus_handle: FocusHandle,
}

impl Keyboard {
    fn apply(&mut self, event: KeyEvent) {
        match event {
            KeyEvent::Down(code) => {
                self.pressed.insert(code);
            }
            KeyEvent::Up(code) => {
                self.pressed.remove(&code);
            }
            KeyEvent::Reset => self.pressed.clear(),
        }
    }

    /// The layer is whichever modifiers are held. Left and right count alike.
    fn mods(&self) -> Mods {
        self.pressed.iter().filter_map(|&c| Mods::of(c)).fold(Mods::NONE, Mods::with)
    }
}

/// What one render pass needs to know beyond the key itself.
struct View<'a> {
    pressed: &'a HashSet<u16>,
    mods: Mods,
    shortcuts: &'a Shortcuts,
}

impl View<'_> {
    fn label(&self, code: Option<u16>) -> Option<&str> {
        self.shortcuts.label(self.mods, code?)
    }
}

/// The three legend tiers of a cap, top to bottom, for the current layer.
struct Legends<'a> {
    top: Option<&'a str>,
    main: Option<&'a str>,
    bottom: Option<&'a str>,
    /// In a shortcut layer: the key does nothing global, so it recedes.
    idle: bool,
    /// Main legend is a shortcut name rather than a key name: smaller type.
    action: bool,
}

fn legends<'a>(k: &'a Key, view: &'a View) -> Legends<'a> {
    let is_modifier = k.code.and_then(Mods::of).is_some();
    if view.mods == Mods::NONE || is_modifier || k.code.is_none() {
        let main = Some(k.main).filter(|m| !m.is_empty());
        return Legends { top: k.shift, main, bottom: k.fn_legend, idle: false, action: false };
    }
    let label = view.label(k.code);
    let fn_label = view.label(k.fn_code);
    if view.mods == Mods::SHIFT && label.is_none() {
        // Shift alone types the shifted symbol; that becomes the legend.
        let main = Some(k.shift.unwrap_or(k.main)).filter(|m| !m.is_empty());
        return Legends { top: None, main, bottom: fn_label, idle: false, action: false };
    }
    Legends {
        top: Some(k.main).filter(|m| !m.is_empty()),
        main: label,
        bottom: fn_label,
        idle: label.is_none() && fn_label.is_none(),
        action: true,
    }
}

fn cap(width: f32, height: f32, pressed: bool) -> Div {
    let (bg, border) = if pressed { (PRESSED_BG, PRESSED_BORDER) } else { (KEY_BG, KEY_BORDER) };
    div()
        .w(px(width))
        .h(px(height))
        .rounded(px(9.0))
        .bg(rgb(bg))
        .border_1()
        .border_color(rgb(border))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(4.0))
        .overflow_hidden()
}

/// A legend that may be a multi-word shortcut name: centred and wrapped.
fn text(s: &str, size: f32, color: u32) -> Div {
    div()
        .w_full()
        .text_center()
        .text_size(px(size))
        .text_color(rgb(color))
        .child(s.to_string())
}

/// One keycap. Legend order top-to-bottom mirrors how they're physically
/// printed: shifted above, primary in the middle, Fn below. `compact` is a
/// half-height cap, which only has room for the main and Fn tiers.
fn keycap(k: &Key, view: &View, height: f32, compact: bool) -> Div {
    let pressed = k.codes().any(|c| view.pressed.contains(&c));
    let l = legends(k, view);
    let fg = |color| if l.idle { DIM_FG } else { color };
    let mut cap = cap(k.width * UNIT, height, pressed).gap(px(if compact { 0.0 } else { 1.0 }));
    if compact {
        // No room for the key name above an action, so an idle key keeps it.
        if let Some(main) = l.main.or(l.top) {
            cap = cap.child(text(main, if l.action { 11.0 } else { 16.0 }, fg(MAIN_FG)));
        }
        if let Some(bottom) = l.bottom {
            cap = cap.child(text(bottom, 10.0, fg(FN_FG)));
        }
        return cap;
    }
    if let Some(top) = l.top {
        cap = cap.child(text(top, if l.action { 12.0 } else { 15.0 }, fg(SHIFT_FG)));
    }
    if let Some(main) = l.main {
        cap = cap.child(text(main, if l.action { 13.0 } else { 20.0 }, fg(MAIN_FG)));
    }
    if let Some(bottom) = l.bottom {
        cap = cap.child(text(bottom, 12.0, fg(FN_FG)));
    }
    cap
}

/// A stacked pair is two independent caps sharing one column, not one cap
/// with two legends.
fn key(k: &Key, view: &View) -> AnyElement {
    let Some(bottom) = &k.stack else {
        return keycap(k, view, KEY_H, false).into_any_element();
    };
    let half = (KEY_H - GAP) / 2.0;
    div()
        .w(px(k.width * UNIT))
        .h(px(KEY_H))
        .flex()
        .flex_col()
        .gap(px(GAP))
        .child(keycap(k, view, half, true))
        .child(keycap(bottom, view, half, true))
        .into_any_element()
}

impl Render for Keyboard {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let view = View { pressed: &self.pressed, mods: self.mods(), shortcuts: &self.shortcuts };
        let mut root = div()
            .track_focus(&self.focus_handle)
            .on_action(|_: &Close, window, _| window.remove_window())
            .relative()
            .flex()
            .flex_col()
            .gap(px(GAP))
            .p(px(PAD))
            .size_full()
            .bg(rgb(BG))
            .children(self.rows.iter().map(|row| {
                div().flex().gap(px(GAP)).children(row.iter().map(|k| key(k, &view)))
            }));
        if !self.input_failures.is_empty() {
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(2.0))
                    .left(px(PAD))
                    .text_size(px(12.0))
                    .text_color(rgb(WARN_FG))
                    .whitespace_nowrap()
                    .child(format!(
                        "Key detection off — {}{}. Install data/70-keycaps.rules (see README).",
                        self.input_failures[0],
                        match self.input_failures.len() {
                            1 => String::new(),
                            n => format!(" (+{} more)", n - 1),
                        }
                    )),
            );
        }
        root
    }
}

fn main() {
    if std::env::args().any(|a| a == "--shortcuts") {
        shortcuts::load().dump().iter().for_each(|line| println!("{line}"));
        return;
    }
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(board_width()), px(board_height())), cx);

        // No titlebar is drawn on Wayland, so without this the window has no
        // dismiss affordance at all. Escape is deliberately not bound: it is a
        // key you may press just to watch it light up.
        cx.bind_keys([KeyBinding::new("ctrl-q", Close, None)]);

        // Closing the last window does not end the process by default.
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Keycaps — Framework 13".into()),
                    ..Default::default()
                }),
                // Wayland matches this against the .desktop file's StartupWMClass.
                app_id: Some("com.viablesys.Keycaps".into()),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window);

                    let (tx, mut events) = futures::channel::mpsc::unbounded();
                    let input_failures = input::start(tx);
                    for failure in &input_failures {
                        eprintln!("keycaps: key detection: {failure}");
                    }
                    cx.spawn(async move |this, cx| {
                        while let Some(event) = events.next().await {
                            let applied = this.update(cx, |keyboard: &mut Keyboard, cx| {
                                keyboard.apply(event);
                                cx.notify();
                            });
                            if applied.is_err() {
                                break; // window closed
                            }
                        }
                    })
                    .detach();

                    let loading = cx.background_spawn(async { shortcuts::load() });
                    cx.spawn(async move |this, cx| {
                        let shortcuts = loading.await;
                        this.update(cx, |keyboard: &mut Keyboard, cx| {
                            keyboard.shortcuts = shortcuts;
                            cx.notify();
                        })
                        .ok();
                    })
                    .detach();

                    Keyboard {
                        rows: layout::framework13(),
                        pressed: HashSet::new(),
                        shortcuts: Shortcuts::default(),
                        input_failures,
                        focus_handle,
                    }
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
