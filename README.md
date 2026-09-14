# Claude Usage Widget

A small always-on-top desktop widget for Windows that shows your Claude Code
plan limits — the same meters `/usage` prints, without leaving them behind in a
terminal.

<p align="center">
  <img src="docs/screenshot.png" alt="The widget showing a session meter and two weekly meters, each with a pace marker" width="400">
</p>

<p align="center">
  <em>The red rule on each bar is the pace marker: where the clock has got to.
  A bar to the left of it is comfortably within pace.</em>
</p>

## Why

`/usage` tells you where you stand, but only when you stop and ask. This keeps
the answer on screen: the rolling five-hour session window, the weekly limit,
and any per-model weekly limit, each with the time it resets.

## Features

- **Live figures.** Reads the same usage endpoint Claude Code uses, so the
  numbers are current rather than whatever was cached the last time you typed
  `/usage`.
- **Both Windows and WSL.** Finds every Claude Code install on the machine,
  including ones inside WSL distributions, and lets you pick between them.
- **Multiple accounts.** Quota is per account; if your installs are signed in as
  different accounts, each is offered in a dropdown.
- **Colour that means something.** Bars carry a hue per limit, and turn amber
  then red as they approach the cap.
- **Fits its content.** The window sizes itself to what it is showing, clamped
  to the work area of whichever monitor it is on.
- **Pace marker.** A red rule on each bar marks how far through the window the
  clock has got. A bar ahead of its marker is burning the window faster than the
  window is passing, and will run out before the reset. Window lengths are
  derived from the payload rather than assumed, by matching each limit's reset
  instant against the `five_hour` and `seven_day` entries beside it.
- **Light and dark**, following the Windows theme.

## Install

Download the installer for your architecture from
[Releases](../../releases) and run it. Windows 11 already has the WebView2
runtime; on Windows 10 the installer will fetch it if needed.

## Settings

The sliders icon opens a panel:

| Setting | Options |
| --- | --- |
| Meters | current session, weekly limits |
| Cache & credits info | on / off |
| Pace marker | on / off |
| Refresh every | manual, 1m – 1h (default 5m) |
| Clock | hidden, bottom left / centre / right |

Choices are stored in `settings.json` in the app's config directory and can be
edited by hand. An out-of-range interval or an unrecognised clock position is
corrected on load rather than rejected, so a bad edit never resets everything
else.

Drag the window by its title bar; close it with the ✕.

## How it works

### Where the numbers come from

The percentages are not derivable from Claude Code's session logs. They come
from `GET https://api.anthropic.com/api/oauth/usage`, the endpoint Claude Code
itself calls, authorised with the OAuth access token already stored in
`~/.claude/.credentials.json`.

Claude Code caches its last answer in `~/.claude.json` under
`cachedUsageUtilization`, and the widget falls back to that when a live read
isn't possible. The cache alone is not enough: Claude Code only rewrites it when
`/usage` runs. Measured during development, a session ran for three days while
the cache stayed pinned to the moment `/usage` was last typed — 46% against a
real 23%.

### Credentials

**The refresh token is never read or written.** Claude Code rotates it, and a
second process writing that file could log you out of Claude Code itself. The
access token is used as-is; once it lapses the widget falls back to the cache
until Claude Code renews it in the course of normal use. Nothing leaves the
machine except the one request above, and error values carry no detail from the
credentials file so no token material can reach a log or the UI.

Note that this endpoint is not a documented, supported API. It may change
without notice, at which point the widget quietly falls back to cached figures.

### Finding installs

Discovery looks, in order:

1. `CLAUDE_CONFIG_DIR`, if set — comma- or semicolon-separated.
2. The native home directory, under both `.claude` and `.config/claude`.
3. Every installed WSL distribution. Names come from the `Lxss` registry key
   rather than by listing the share, because the share lists only *running*
   distros. `\\wsl.localhost` is tried first, `\\wsl$` as a fallback.

Running inside WSL the lookup is symmetric, picking up the Windows side through
`/mnt/c`, which is what makes the `probe` binary useful during development.

## Layout

```
core/        pure Rust: install discovery, the usage API client, settings.
             No Tauri dependency, so it builds and tests anywhere.
core/src/bin/probe.rs   prints what the widget would show, without a window.
src-tauri/   the Tauri 2 shell: commands and window sizing.
src/         the UI (TypeScript + Vite, no framework).
```

`core` is kept free of Tauri on purpose: Tauri pulls in GTK and dbus on Linux,
which would make the logic untestable anywhere but a fully provisioned Windows
machine.

## Building

### On Windows

Requires MSVC build tools, WebView2, Node and Rust
(`winget install Rustlang.Rustup`). The project must sit on the Windows
filesystem — Windows `node.exe` cannot resolve a `/home/...` path, and cargo over
the `\\wsl.localhost` share is slow enough to be unusable.

```sh
npm install
npm run tauri dev                # live-reloading window
npm run tauri build              # installer
npm run tauri build --no-bundle  # just the .exe
```

Build through the Tauri CLI, never plain `cargo build`: `tauri-build` sets
`cfg(dev)` unless the CLI says otherwise, so a bare `cargo build --release`
produces a binary that still points at the dev server and shows WebView2's
"cannot connect" page instead of the app.

Running these from a WSL shell fails with `cargo metadata ... program not
found`: the Windows Tauri CLI inherits WSL's Linux `PATH` rather than the
persistent Windows one.

### Cross-compiling from Linux

Tauri supports `cargo-xwin`, which supplies the MSVC CRT and Windows SDK so the
MSVC targets link on Linux.

```sh
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc
cargo install --locked cargo-xwin
sudo apt install llvm clang lld     # llvm-rc, clang-cl, lld-link

export XWIN_ACCEPT_LICENSE=1
export XWIN_ARCH=x86_64,x86         # x86 import libs, or the 32-bit link fails
npm install
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

`npm run tauri dev` still has to run on Windows — it launches the window.

Don't share `target/` between a Linux host and a Windows host: a host build
lands in `target/debug` either way, so alternating makes cargo rebuild the world
each time. Give the cross-build its own with `CARGO_TARGET_DIR`.

### Checks

```sh
cargo test -p claude-usage-core
cargo run -p claude-usage-core --bin probe           # what the widget would show
cargo run -p claude-usage-core --bin probe -- --json # exactly what the UI receives
```

`npm run build` ends with a check that the emitted CSS fully minified. A
malformed rule — an unclosed `var(`, say — makes esbuild pass the rest of the
file through verbatim, which silently drops every rule after it while the build
still reports success.

## Colours

Surfaces and ink are Anthropic's brand palette: the warm off-white, the
near-black and the clay greys.

The bars are deliberately **not** the brand accents. Run through the data-viz
palette validator, `#d97757` / `#6a9bcc` / `#788c5d` fail hard — the orange and
the green sit ΔE 0.7 apart under simulated protanopia, indistinguishable to a
red-blind viewer, and two of the three fall under the chroma floor. The bars use
validated hues instead, led by the orange nearest the brand clay, and pass
all-pairs against these surfaces in both light and dark. Status colours are
fixed in both modes and only ever mean status, never identity.

## Licence

MIT
