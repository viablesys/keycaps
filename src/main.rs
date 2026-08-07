//! keycaps — an on-screen reference for a keyboard with blank keycaps.
//!
//! Renders every legend at once: shifted symbols above the primary, Fn
//! secondaries below in an accent colour. No modes, no toggles — a reference
//! you read rather than operate.
//!
//! This is a GPUI pilot. The shape to notice: `layout.rs` is plain data with
//! no framework types in it, and everything below is a pure function from that
//! data to elements. Swapping the renderer would not touch the model.

mod layout;

// Imported explicitly rather than via `gpui::prelude::*` so it's visible which
// traits the builder syntax actually depends on: Styled for the style helpers,
// ParentElement for .child/.children, AppContext for cx.new.
use gpui::{
    AnyElement, App, AppContext, Application, Bounds, Context, FocusHandle, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, Styled, TitlebarOptions, Window, WindowBounds,
    WindowOptions, actions, div, px, rgb, size,
};

use layout::Key;

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
const MAIN_FG: u32 = 0xf2f2f4;
const SHIFT_FG: u32 = 0x9a9aa2;
const FN_FG: u32 = 0x6fb6ff;

struct Keyboard {
    rows: Vec<Vec<Key>>,
    /// Actions only reach an element that holds focus, so the root needs a
    /// handle even though nothing here is interactive.
    focus_handle: FocusHandle,
}

/// One keycap. Legend order top-to-bottom mirrors how they're physically
/// printed: shifted above, primary in the middle, Fn below.
fn key(k: &Key) -> AnyElement {
    // A stacked pair is two independent caps sharing one column, not one cap
    // with two legends — so it returns early rather than falling through to
    // the legend-tier logic below.
    if let Some(bottom) = k.stack {
        let half = (KEY_H - GAP) / 2.0;
        let half_cap = |glyph: &'static str| {
            div()
                .w(px(k.width * UNIT))
                .h(px(half))
                .rounded(px(9.0))
                .bg(rgb(KEY_BG))
                .border_1()
                .border_color(rgb(KEY_BORDER))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.0))
                .text_color(rgb(MAIN_FG))
                .child(glyph)
        };

        return div()
            .w(px(k.width * UNIT))
            .h(px(KEY_H))
            .flex()
            .flex_col()
            .gap(px(GAP))
            .child(half_cap(k.main))
            .child(half_cap(bottom))
            .into_any_element();
    }

    let mut cap = div()
        .w(px(k.width * UNIT))
        .h(px(KEY_H))
        .rounded(px(9.0))
        .bg(rgb(KEY_BG))
        .border_1()
        .border_color(rgb(KEY_BORDER))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.0));

    if let Some(shift) = k.shift {
        cap = cap.child(
            div()
                .text_size(px(15.0))
                .text_color(rgb(SHIFT_FG))
                .child(shift),
        );
    }

    if !k.main.is_empty() {
        cap = cap.child(
            div()
                .text_size(px(20.0))
                .text_color(rgb(MAIN_FG))
                .child(k.main),
        );
    }

    if let Some(fn_legend) = k.fn_legend {
        cap = cap.child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(FN_FG))
                .child(fn_legend),
        );
    }

    cap.into_any_element()
}

impl Render for Keyboard {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(|_: &Close, window, _| window.remove_window())
            .flex()
            .flex_col()
            .gap(px(GAP))
            .p(px(PAD))
            .size_full()
            .bg(rgb(BG))
            .children(self.rows.iter().map(|row| {
                div().flex().gap(px(GAP)).children(row.iter().map(key))
            }))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(board_width()), px(board_height())), cx);

        // No titlebar is drawn on Wayland, so without these the window has no
        // dismiss affordance at all.
        cx.bind_keys([
            KeyBinding::new("escape", Close, None),
            KeyBinding::new("ctrl-q", Close, None),
        ]);

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
                    Keyboard { rows: layout::framework13(), focus_handle }
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
