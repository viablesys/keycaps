# keycaps

An on-screen reference for a keyboard with blank keycaps — specifically a
Framework Laptop 13 (AMD Ryzen AI 300 Series), US ANSI.

Every legend renders at once: shifted symbols above the primary, Fn
secondaries below in blue. There are no modes and no toggles, because this is
something you read, not something you operate.

Also a **GPUI pilot** — a first look at building Linux desktop apps in Rust
with Zed's UI framework.

## Why not just use Tecla

We forked GNOME Tecla first (`~/dev/viablesys/tecla`) and it works. But its
model is one label per key, sourced from the XKB keymap, which forces two
compromises this app doesn't have:

- **Fn secondaries need a toggle.** `TeclaKey` holds a single `label` string,
  so showing the Fn function means hiding the primary. Here a keycap is a
  column of up to three legends, which is how a real keycap works.
- **Shifted symbols need a mode too**, for the same reason — Tecla puts them
  behind numbered level buttons.

The fork remains the better choice if you want the *live* keymap. This one
trades that for showing everything at once.

## The GPUI part

What's worth copying from this as a template:

**The model has no framework types in it.** `src/layout.rs` is `Vec<Vec<Key>>`
and knows nothing about GPUI. `src/main.rs` is a pure function from that data
to elements. Swapping renderers wouldn't touch the model.

**Layout is Tailwind-shaped.** `div().flex().flex_col().gap(px(6.)).items_center()`
— the style helpers are macro-generated in `gpui_macros::style_helpers!()`, which
is why grepping the `Styled` trait for `fn w` finds nothing. The names mirror
Tailwind's, so Tailwind's docs are usable as a reference.

**Everything scales from one constant.** `UNIT` is the width of a 1u key;
every other dimension derives from it, and rows sum to 15u so they align
without measurement.

### Linux build dependencies

GPUI's README documents macOS dependencies only — the Linux section is absent.
On Fedora these are what it actually needs:

```
sudo dnf install vulkan-loader-devel vulkan-headers wayland-devel \
    libxkbcommon-devel libxkbcommon-x11-devel fontconfig-devel \
    freetype-devel libX11-devel libxcb-devel cmake gcc-c++ perl
```

GPUI renders through Vulkan on Linux. The first build compiles ~700 crates and
takes a while; incremental builds after that are quick.

### Version caution

`gpui = "0.2.2"` is published on crates.io, but the crate is early and tracks
Zed's needs — expect breaking changes between releases. Pin it.

## Provenance of the Fn legends

Captured from evdev on the actual machine, not read off a spec sheet:

```
sudo libinput debug-events --show-keycodes
```

Sweep the top row, each key bare and then with Fn held, and transcribe. Note
this logs every keystroke system-wide while it runs.

Two keys emit nothing at all, because the embedded controller handles them
internally and never tells the OS: **Fn+Esc** (Fn lock) and **Fn+Space**
(keyboard backlight). They're labelled anyway — the keycap is what's being
documented, not the event stream.

## Layout accuracy

The ISO/ANSI distinction matters more than it looks. A PC105 ISO layout draws
three keys this machine doesn't have — the 102nd key beside left Shift, plus
two extra modifiers right of the space bar. On a keyboard with legends that's
untidy; on a blank one it shifts every subsequent keycap, so the drawing
actively lies about which key is which.

This machine has exactly two modifiers right of the space bar, RAlt and RCtrl,
then the arrow cluster.

## Repo map

`repo-map` generates a VSM-shaped structural map under `map-output/`:

```
repo-map --project keycaps
```

## Building

```
cargo build --release
./target/release/keycaps
```
