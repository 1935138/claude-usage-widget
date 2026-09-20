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

```sh
npm run release -- 0.1.7             # version, checks, both architectures, signatures
npm run release -- 0.1.7 --publish   # the above, then commit, push and create the release
```

The script does what used to be six manual steps, each of which went wrong at
least once: it refuses to start on a dirty tree, off `main`, or while the widget
is running (Windows locks a running `.exe`, and the build then fails at the link
step with a message that never mentions the widget). It bumps the version in the
three files and both READMEs, runs the checks, builds x64 and x86, and stages the
installers, a `SHA256SUMS` and `latest.json` under `target/release-<version>/`.

A run without `--publish` leaves the version bumped and nothing uploaded, so you
can look at what it made. `git checkout .` undoes the bump.

### Cutting it from CI instead

The same script runs on a runner, from the **Release** workflow under Actions:
give it a version and it does everything the local run does. That is the way to
release from a machine with Smart App Control switched on, which blocks the
unsigned test binaries `cargo test` builds and cannot be switched back on once
switched off. It also keeps the signing key off workstations entirely.

Two repository secrets stand in for the two environment variables:

```
TAURI_SIGNING_PRIVATE_KEY            the key file's contents, not a path
TAURI_SIGNING_PRIVATE_KEY_PASSWORD   the password it was generated with
```

The workflow pushes the version bump to `main`, so a branch protection rule
that forbids direct pushes will stop it at the last step, after the installers
have been built and before anything is published.

### The signing key

Releases are signed. An unsigned one cannot be installed as an update by anyone
who already has the widget, because the updater verifies the signature against
the public key baked into the app. The key is not in this repository:

```sh
TAURI_SIGNING_PRIVATE_KEY=~/.tauri/claude-usage-widget.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=…
```

On Windows the script reads the password from the user environment in the
registry when the variable is not already set, because `setx` does not reach a
shell that is already running.

**Losing the key means no existing install can ever be updated again** - a new
key would be rejected by every widget already out there, and everyone would have
to install by hand. Keep an encrypted copy somewhere other than the machine that
cuts releases. The key file is itself password-protected, which is what makes
that copy safe to keep.

### What `latest.json` is

The file installed widgets read to learn a release exists. It names the version,
each architecture's installer URL and the signature of that installer. It must be
UTF-8 **without a BOM** - PowerShell's `Set-Content -Encoding utf8` adds one and
the updater cannot parse the result, which is the same trap `settings.json` has.
The script writes it correctly; assembling it by hand is what to avoid.

GitHub caches `/releases/latest/download/latest.json` for a few minutes, so a
release is not visible to installed widgets the instant it is published.

## Sending a change

- Keep commits to one change each, with a message that says why rather than
  what. The history is the only place the reasoning survives.
- `cargo fmt` and clippy are enforced by CI, so run them before pushing.
- A change to what the card shows wants a screenshot in the pull request.
