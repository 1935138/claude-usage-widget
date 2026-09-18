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
  `/usage`. The footer says when they were last updated, and says so plainly
  when it is showing a cache instead.
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
| [claude-usage-widget_0.1.3_x64-setup.exe](../../releases/download/v0.1.3/claude-usage-widget_0.1.3_x64-setup.exe) | 64-bit Windows |
| [claude-usage-widget_0.1.3_x86-setup.exe](../../releases/download/v0.1.3/claude-usage-widget_0.1.3_x86-setup.exe) | 32-bit Windows |

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
| Time format | 24-hour (default) / 12-hour |
| Language | follows the machine (default) / English / 한국어 |
| Corners | square, slight, rounded (default), very rounded |

Settings are stored as `settings.json` in the app's config directory and can be
edited by hand. Values out of range are corrected on load instead of resetting
the file, and a UTF-8 BOM is tolerated. `cornerRadius` accepts any value up to
24px, including ones the panel does not list.

The window is undecorated and transparent, so Windows draws no frame around it.
The card paints its own outline, which is what **Corners** shapes.

The card is written in English or Korean, following the machine's display
language unless you pick one. Times are not part of that choice: they follow the
machine's own region, with **Time format** deciding only the 12- or 24-hour
clock.

## Credentials

The widget reads the OAuth **access** token Claude Code keeps in
`~/.claude/.credentials.json`, and nothing else from that file. **The refresh
token is never read or written.** Claude Code rotates it, and a second process
writing that file could log you out of Claude Code itself. The access token is
used as-is; once it lapses the widget falls back to Claude Code's cached figures
until Claude Code renews the token during normal use.

Nothing leaves the machine except the one request that fetches the figures.
Errors carry no detail from the credentials file, so no token material can reach
a log or the UI. Signing in is delegated to `claude login` in its own console
for the same reason.

## Contributing

Building it, the checks, and how the code is laid out:
[CONTRIBUTING.md](CONTRIBUTING.md) and
[docs/architecture.md](docs/architecture.md).

## License

MIT. See [LICENSE](LICENSE).
