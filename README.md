<h1 align="center">Claude Usage Widget</h1>

<p align="center">
  Your Claude Code plan limits, always on screen.
</p>

<p align="center">
  <a href="../../releases"><img alt="Release" src="https://img.shields.io/github/v/release/1935138/claude-usage-widget"></a>
  <a href="../../releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/1935138/claude-usage-widget/total"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-blue">
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/1935138/claude-usage-widget"></a>
</p>

<p align="center">
  <img src="docs/screenshot.png" alt="The widget showing a session meter and two weekly meters" width="400">
</p>

<p align="center">
  English · <a href="docs/README.ko.md">한국어</a>
</p>

`/usage` tells you where you stand, but only when you stop and ask. This widget
keeps the same meters on screen: the rolling five-hour session window, the
weekly limit, and any per-model weekly limit, each with the time it resets.

## Features

- **Live figures.** Reads the same usage endpoint Claude Code uses, so the
  numbers are current rather than whatever was cached when you last typed
  `/usage`.
- **Pace marker.** The red rule on each bar is where the clock has got to. A bar
  ahead of its marker will run out before the window resets.
- **Windows and WSL.** Finds every Claude Code install on the machine, including
  ones inside WSL distributions.
- **Multiple accounts.** Quota is per account. Installs signed in as different
  accounts are offered in a dropdown.
- **Signed out? One click.** With no credentials on the machine, the card offers
  a button that runs `claude login` for you.
- **Stays out of the way.** Always on top, no taskbar entry, sizes itself to its
  content, and follows the Windows light/dark theme.

## Install

Download an installer and run it.

| File | For |
| --- | --- |
| [claude-usage-widget_0.1.1_x64-setup.exe](../../releases/download/v0.1.1/claude-usage-widget_0.1.1_x64-setup.exe) | 64-bit Windows |
| [claude-usage-widget_0.1.1_x86-setup.exe](../../releases/download/v0.1.1/claude-usage-widget_0.1.1_x86-setup.exe) | 32-bit Windows |

Windows 11 already has the WebView2 runtime. On Windows 10 the installer fetches
it if needed. Older versions and `SHA256SUMS` are on the
[releases page](../../releases).

You also need Claude Code installed and signed in. Nothing else to configure.

## Usage

Drag the window by its title bar. Close it with the ✕. The sliders icon opens
the settings panel:

| Setting | Options |
| --- | --- |
| Meters | current session, weekly limits |
| Cache & credits info | on / off |
| Pace marker | on / off |
| Refresh every | manual, 3m – 1h (default 5m) |
| Clock | shown / hidden |
| Corners | square, slight, rounded (default), very rounded |

Settings are stored as `settings.json` in the app's config directory and can be
edited by hand. Values out of range are corrected on load instead of resetting
the file, and a UTF-8 BOM is tolerated. `cornerRadius` accepts any value up to
24px, including ones the panel does not list.

The window is undecorated and transparent, so Windows draws no frame around it.
The card paints its own outline, which is what **Corners** shapes.

## How it works

### Where the numbers come from

The percentages are not in Claude Code's session logs. They come from
`GET https://api.anthropic.com/api/oauth/usage`, the endpoint Claude Code itself
calls, authorised with the OAuth access token in `~/.claude/.credentials.json`.

Claude Code caches its last answer in `~/.claude.json` under
`cachedUsageUtilization`, and the widget falls back to that when a live read is
not possible. The cache is only ever a fallback: an install with working
credentials shows live figures even if `/usage` has never run on it. When the
figures are cached, the widget says why rather than leaving you to guess: the
sign-in expired, the API is rate-limiting, or it did not answer.

This endpoint is not a documented or supported API. If it changes, the widget
falls back to cached figures.

### Credentials

**The refresh token is never read or written.** Claude Code rotates it, and a
second process writing that file could log you out of Claude Code itself. The
access token is used as-is; once it lapses the widget falls back to the cache
until Claude Code renews it during normal use.

Nothing leaves the machine except the one request above. Errors carry no detail
from the credentials file, so no token material can reach a log or the UI.
Signing in is delegated to `claude login` in its own console for the same
reason.

### Finding installs

Discovery looks, in order:

1. `CLAUDE_CONFIG_DIR`, if set (comma- or semicolon-separated).
2. The native home directory, under both `.claude` and `.config/claude`.
3. Every installed WSL distribution. Names come from the `Lxss` registry key,
   because the network share lists only *running* distros. `\\wsl.localhost` is
   tried first, `\\wsl$` as a fallback.

Two installs signed into the same account are shown once, keyed by email. Who an
install is signed in as comes from `oauthAccount`, not from the cache block,
which records whoever was signed in when it was last written.

## Building

Requires MSVC build tools, WebView2, Node and Rust
(`winget install Rustlang.Rustup`).

```sh
npm install
npm run tauri dev                # live-reloading window
npm run tauri build              # installer
npm run tauri build --no-bundle  # just the .exe
```

Three things worth knowing:

- Keep the project on the Windows filesystem. Windows `node.exe` cannot resolve
  a `/home/...` path, and cargo over `\\wsl.localhost` is unusably slow.
- Build through the Tauri CLI, never plain `cargo build`. `tauri-build` sets
  `cfg(dev)` unless the CLI says otherwise, so a bare `cargo build --release`
  produces a binary that still points at the dev server.
- Run it from a Windows shell. From WSL it fails with
  `cargo metadata ... program not found`, because the Windows Tauri CLI inherits
  WSL's Linux `PATH`.

### Cross-compiling from Linux

```sh
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc
cargo install --locked cargo-xwin
sudo apt install llvm clang lld     # llvm-rc, clang-cl, lld-link

export XWIN_ACCEPT_LICENSE=1
export XWIN_ARCH=x86_64,x86         # x86 import libs, or the 32-bit link fails
npm install
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

`npm run tauri dev` still has to run on Windows. Give the cross-build its own
`CARGO_TARGET_DIR`, or host and cross builds will fight over `target/debug`.

### Checks

```sh
cargo test -p claude-usage-core
cargo run -p claude-usage-core --bin probe           # what the widget would show
cargo run -p claude-usage-core --bin probe -- --json # exactly what the UI receives
npm run build                                        # types, bundle, CSS check
```

`npm run build` ends by checking that the CSS fully minified. A malformed rule
makes esbuild pass the rest of the file through verbatim, dropping every rule
after it while the build still reports success.

## Project layout

```
core/        pure Rust: install discovery, the usage API client, settings.
             No Tauri dependency, so it builds and tests anywhere.
core/src/bin/probe.rs   prints what the widget would show, without a window.
src-tauri/   the Tauri 2 shell: commands and window sizing.
src/         the UI (TypeScript + Vite, no framework).
```

`core` is kept free of Tauri on purpose. Tauri pulls in GTK and dbus on Linux,
which would make the logic untestable anywhere but a fully provisioned Windows
machine.

## Colours

Surfaces and ink come from Anthropic's brand palette. The bars do not: the brand
accents fail a colour-blindness check against each other, so the bars use
validated hues instead, led by the orange nearest the brand clay. Colour carries
identity only: a bar keeps its hue however full it is, and how close it is to
its cap is left to the percentage and the pace marker.

## License

MIT. See [LICENSE](LICENSE).
