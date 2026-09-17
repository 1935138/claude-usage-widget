# Architecture

## Where things live

```
core/                    pure Rust, no Tauri dependency, so it builds and tests
                         anywhere - including the Linux CI job.
  discovery.rs           finding Claude Code installs, native and WSL
  limits.rs              the figures the card shows
    limits/payload.rs    the JSON Claude Code writes and the API answers with
    limits/labels.rs     naming a limit
    limits/windows.rs    how long a limit's window runs
  live.rs                the usage API request
    live/credentials.rs  reading the access token, never the refresh one
    live/backoff.rs      waiting out a 429
  usage.rs               token totals from the session logs
  settings.rs            what the widget shows, and the defaults
  bin/probe.rs           prints what the widget would show, without a window
src-tauri/               the Tauri 2 shell
  commands.rs            everything the page can call, and nothing else
  layout.rs              sizing the window to its content, within the display
  settings.rs            persisting settings to the app's config directory
src/                     the UI (TypeScript + Vite, no framework)
  types.ts               the shapes that cross the IPC boundary
  format.ts              values into words: times, percentages, provenance
  accounts.ts            which account's figures are on the card
  choices.ts             what the settings panel offers
  view/card.ts           the meters and the account line
  view/settings.ts       the settings panel
  main.ts                state, events and the calls to the backend
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
