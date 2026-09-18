# Contributing

Issues and pull requests are welcome. This page is what you need to build the
widget and check your work; [docs/architecture.md](docs/architecture.md)
explains how the code is laid out and why.

## Building it

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
- Close the widget before building. Windows locks a running `.exe`, and the
  build fails at the link step with `failed to remove file ... Access is
  denied. (os error 5)`.

## Cross-compiling from Linux

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

## Checks

```sh
cargo test --workspace
cargo run -p claude-usage-core --bin probe           # what the widget would show
cargo run -p claude-usage-core --bin probe -- --json # exactly what the UI receives
npm test                                             # the pure frontend functions
npm run build                                        # types, bundle, CSS check
```

`npm run build` ends by checking that the CSS fully minified. A malformed rule
makes esbuild pass the rest of the file through verbatim, dropping every rule
after it while the build still reports success.

## Releasing

The version lives in three files - `package.json`, `src-tauri/tauri.conf.json`
and `src-tauri/Cargo.toml` - plus the installer links in both READMEs. Bump all
five, then build both architectures **with the widget closed**, since Windows
locks a running `.exe` and the build fails at the link step:

```sh
npm run tauri build                                  # x64
npx tauri build --target i686-pc-windows-msvc        # x86
```

Builds have to be signed, or the update will be refused by everyone who already
has the widget. The private key is not in this repository; point the build at it:

```sh
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/claude-usage-widget.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="…" npm run tauri build
```

Each installer then comes with a `.sig` beside it. The release carries four
assets: the two installers renamed to `claude-usage-widget_<version>_x64-setup.exe`
and `…_x86-setup.exe`, a `SHA256SUMS` over both, and `latest.json`, which is what
installed widgets read to find out a release exists:

```json
{
  "version": "0.1.5",
  "notes": "…",
  "pub_date": "2026-09-18T07:24:58Z",
  "platforms": {
    "windows-x86_64": { "signature": "<the .sig file's contents>", "url": "<the x64 installer's download URL>" },
    "windows-i686":   { "signature": "…",                          "url": "…" }
  }
}
```

Write `latest.json` as UTF-8 **without a BOM**. PowerShell's `Set-Content
-Encoding utf8` adds one, and the updater cannot parse it - the same trap
`settings.json` has. GitHub also caches `/releases/latest/download/latest.json`
for a few minutes, so a corrected file does not take effect immediately.

## Sending a change

- Keep commits to one change each, with a message that says why rather than
  what. The history is the only place the reasoning survives.
- `cargo fmt` and clippy are enforced by CI, so run them before pushing.
- A change to what the card shows wants a screenshot in the pull request.
