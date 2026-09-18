// Cuts a release: version, checks, both architectures, signatures, update feed.
//
// Every step here was a manual one that went wrong at least once. The order
// matters: the checks run before anything is built, and nothing is published
// until the artefacts exist and have been signed.
//
//   npm run release -- 0.1.7             build and stage, publish nothing
//   npm run release -- 0.1.7 --publish   the above, then commit, push, release
//
// Signing needs a key the repository does not carry:
//
//   TAURI_SIGNING_PRIVATE_KEY           path to the key, or its contents
//   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  the password it was generated with
//
// On Windows the password is read from the user's environment in the registry
// when the variable is not already set, because `setx` does not reach a shell
// that was already running.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OWNER_REPO = "1935138/claude-usage-widget";

const ARCHITECTURES = [
  { label: "x64", target: null, platform: "windows-x86_64" },
  { label: "x86", target: "i686-pc-windows-msvc", platform: "windows-i686" },
];

/** Where a version number lives, besides the two READMEs. */
const VERSION_FILES = [
  { file: "package.json", pattern: (v) => `"version": "${v}",` },
  { file: "src-tauri/tauri.conf.json", pattern: (v) => `"version": "${v}",` },
  { file: "src-tauri/Cargo.toml", pattern: (v) => `version = "${v}"` },
];

const READMES = ["README.md", "docs/README.ko.md"];

function die(message) {
  console.error(`\n  ${message}\n`);
  process.exit(1);
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: options.quiet ? "pipe" : "inherit",
    shell: process.platform === "win32",
    ...options,
  });
}

function read(file) {
  return readFileSync(join(ROOT, file), "utf8");
}

/** Writes UTF-8 with no BOM. A feed with one is a feed the updater cannot read. */
function write(file, contents) {
  writeFileSync(join(ROOT, file), contents, { encoding: "utf8" });
}

function currentVersion() {
  return JSON.parse(read("package.json")).version;
}

function refuseUnlessReady() {
  const dirty = run("git", ["status", "--porcelain"], { quiet: true }).trim();
  if (dirty) die("The working tree has changes. Commit or stash them first.");

  const branch = run("git", ["rev-parse", "--abbrev-ref", "HEAD"], { quiet: true }).trim();
  if (branch !== "main") die(`On ${branch}. Releases are cut from main.`);

  // Windows locks a running .exe, and the build fails at the link step with a
  // message that does not mention the widget at all.
  if (process.platform === "win32") {
    const tasks = run("tasklist", ["/FI", "IMAGENAME eq claude-usage-widget.exe"], { quiet: true });
    if (tasks.includes("claude-usage-widget.exe")) {
      die("The widget is running. Close it: the build cannot replace a locked .exe.");
    }
  }
}

function signingEnvironment() {
  const key = process.env.TAURI_SIGNING_PRIVATE_KEY;
  if (!key) die("TAURI_SIGNING_PRIVATE_KEY is not set. An unsigned release cannot be installed as an update.");

  let password = process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD;
  if (password === undefined && process.platform === "win32") {
    try {
      const query = run(
        "reg",
        ["query", "HKCU\\Environment", "/v", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"],
        { quiet: true },
      );
      password = query.trim().split(/\s{2,}/).pop();
    } catch {
      // Not there either; fall through to the error below.
    }
  }
  if (password === undefined) die("TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set.");

  return { ...process.env, TAURI_SIGNING_PRIVATE_KEY: key, TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password };
}

function bump(from, to) {
  for (const { file, pattern } of VERSION_FILES) {
    const before = read(file);
    const after = before.replace(pattern(from), pattern(to));
    if (after === before) die(`${file} does not name version ${from}.`);
    write(file, after);
  }
  for (const file of READMES) {
    write(file, read(file).replaceAll(from, to));
  }
  console.log(`  version ${from} -> ${to}`);
}

function check() {
  run("cargo", ["fmt", "--check"]);
  run("cargo", ["test", "--workspace"]);
  run("npm", ["test"]);
}

/** Builds one architecture and answers where its installer and signature landed. */
function build(architecture, version, env) {
  // `npx` rather than `npm run tauri --`: PowerShell eats the `--`, and the
  // target argument then arrives at the wrong command.
  const args = ["tauri", "build"];
  if (architecture.target) args.push("--target", architecture.target);
  run("npx", args, { env });

  const bundle = architecture.target
    ? `target/${architecture.target}/release/bundle/nsis`
    : "target/release/bundle/nsis";
  const installer = join(ROOT, bundle, `Claude Usage Widget_${version}_${architecture.label}-setup.exe`);
  return { installer, signature: `${installer}.sig` };
}

function stage(version, built) {
  const out = join(ROOT, "target", `release-${version}`);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });

  const base = `https://github.com/${OWNER_REPO}/releases/download/v${version}`;
  const platforms = {};
  const sums = [];

  for (const [architecture, artefacts] of built) {
    const name = `claude-usage-widget_${version}_${architecture.label}-setup.exe`;
    copyFileSync(artefacts.installer, join(out, name));
    platforms[architecture.platform] = {
      signature: readFileSync(artefacts.signature, "utf8").trim(),
      url: `${base}/${name}`,
    };
    const digest = createHash("sha256").update(readFileSync(join(out, name))).digest("hex");
    sums.push(`${digest} *${name}`);
  }

  writeFileSync(join(out, "SHA256SUMS"), `${sums.join("\n")}\n`, "utf8");
  writeFileSync(
    join(out, "latest.json"),
    `${JSON.stringify(
      {
        version,
        notes: `See https://github.com/${OWNER_REPO}/releases/tag/v${version}`,
        pub_date: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
        platforms,
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
  return out;
}

function publish(version, out) {
  run("git", ["commit", "-am", `chore(release): bump version to ${version}`]);
  run("git", ["push", "origin", "main"]);
  run("gh", [
    "release",
    "create",
    `v${version}`,
    "--title",
    `v${version}`,
    "--target",
    "main",
    "--generate-notes",
    join(out, `claude-usage-widget_${version}_x64-setup.exe`),
    join(out, `claude-usage-widget_${version}_x86-setup.exe`),
    join(out, "SHA256SUMS"),
    join(out, "latest.json"),
  ]);
}

const [version, ...flags] = process.argv.slice(2);
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  die("Usage: npm run release -- <version> [--publish]");
}

const from = currentVersion();
if (from === version) die(`Already at ${version}.`);

refuseUnlessReady();
const env = signingEnvironment();
bump(from, version);
check();

const built = ARCHITECTURES.map((architecture) => [architecture, build(architecture, version, env)]);
const out = stage(version, built);

console.log(`\n  staged in ${out}`);
if (flags.includes("--publish")) {
  publish(version, out);
  console.log(`\n  published https://github.com/${OWNER_REPO}/releases/tag/v${version}\n`);
} else {
  console.log("\n  nothing published. Re-run with --publish, or upload what is staged by hand.\n");
}
