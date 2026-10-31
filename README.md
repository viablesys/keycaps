# keycaps

An on-screen reference for a keyboard with blank keycaps — specifically a
Framework Laptop 13 (AMD Ryzen AI 300 Series), US ANSI.

At rest every legend renders at once: shifted symbols above the primary, Fn
secondaries below in blue. There are still no toggles — it changes only in
response to the keyboard:

- **Keys light up as you press them**, anywhere on the system, modifiers
  included. Close with Ctrl+Q (Escape no longer closes it: you may press it
  just to watch it light).
- **Holding modifiers switches the legends** to GNOME's global shortcuts for
  exactly that combination — Super shows Lock on L and App 1–9 on the digits,
  Ctrl+Alt shows the TTYs on the F-row. Keys with no global shortcut fade.
  Shift alone shows the shifted symbols as the main legend. App shortcuts
  (Ctrl+W…) are never shown: they differ per app, so one legend would lie.

## Key detection

Wayland only gives key events to the focused window, so keycaps reads the
keyboard's evdev devices directly — read-only, never grabbed, nothing logged.
A Framework 13 spreads its keys over four devices (letters and F-row on the
i8042 keyboard; brightness on the ACPI Video Bus and the EC's Consumer
Control; airplane mode on Wireless Radio Control). Grant your desktop session
read access to just those four:

```
sudo install -m644 data/70-keycaps.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=input --action=change
```

The trade-off: any program in your session can then read keystrokes from
those devices, much as under X11. Without the rule the app still runs as a
static reference and says why detection is off along the bottom edge.

Override the device list with `KEYCAPS_DEVICES` — comma-separated device
names as in `/proc/bus/input/devices`, or absolute paths. A path to a FIFO is
how the layers were tested without touching the real keyboard.

Two things the hardware dictates: **Fn never lights** (the EC never reports
it), and **Fn+F9 lights Super and P**, because that is literally what the EC
sends for "Display".

## Shortcut layers

Read once at startup from gsettings: `org.gnome.shell.keybindings`,
`org.gnome.desktop.wm.keybindings`, both mutter schemas, media-keys, and your
custom shortcuts (which take precedence and use their own names — keep them
short, a keycap fits about three short lines). Restart to pick up changes.

`keycaps --shortcuts` prints every binding it found as `mods code label` —
the first place to look when a key shows blank. Legends for GNOME's action ids
live in `data/labels.tsv`; unlisted ids fall back to the id made readable.

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
plus the held keys to elements. Key events arrive from reader threads over a
`futures` channel drained by a `cx.spawn` task. Swapping renderers wouldn't touch the model.

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
