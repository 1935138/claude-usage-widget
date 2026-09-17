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

## Sending a change

- Keep commits to one change each, with a message that says why rather than
  what. The history is the only place the reasoning survives.
- `cargo fmt` and clippy are enforced by CI, so run them before pushing.
- A change to what the card shows wants a screenshot in the pull request.
