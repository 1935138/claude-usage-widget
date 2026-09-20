//! Finding and starting the Claude Code CLI.
//!
//! The widget inherits its environment from whatever launched it - Explorer,
//! the Start menu, a shortcut - and that environment was captured at logon. A
//! CLI installed since then is not on that `PATH`, and the native installer
//! puts it somewhere `PATH` need never have mentioned at all. Sending a bare
//! `claude` through a shell regardless is what made the sign-in console flash
//! open and vanish: `cmd` printed "not recognized" and closed with it unread.
//!
//! So the executable is located first and then run by its full path, and a
//! machine with no CLI on it is told apart from one that simply has not signed
//! in yet.

use std::path::{Path, PathBuf};

/// File names the CLI ships under. Windows keeps two: the native build is an
/// `.exe`, an npm install is a `.cmd` shim standing in front of a script.
#[cfg(windows)]
const NAMES: [&str; 2] = ["claude.exe", "claude.cmd"];
#[cfg(not(windows))]
const NAMES: [&str; 1] = ["claude"];

/// Where the installers leave the CLI, relative to the home directory, for
/// when `PATH` does not mention it: the native installer uses the first, the
/// older local install the second.
const HOME_DIRS: [&str; 2] = [".local/bin", ".claude/local"];

/// Where to send someone who has no CLI to sign in with.
pub const INSTALL_DOCS: &str = "https://code.claude.com/docs/en/setup";

/// Do not let a console flash up for a process that has nothing to show.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// The sign-in prints a URL to open by hand, so it needs a console of its own:
/// this widget has none to lend it.
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// The Claude Code CLI on this machine, `PATH` first and then the places the
/// installers use.
pub fn locate() -> Option<PathBuf> {
    on_path().or_else(|| known_dirs().iter().find_map(|dir| named_in(dir)))
}

fn on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| named_in(&dir))
}

/// The first of the CLI's names that exists in `dir`.
fn named_in(dir: &Path) -> Option<PathBuf> {
    NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

fn known_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = claude_usage_core::discovery::home_dir() {
        dirs.extend(HOME_DIRS.iter().map(|rel| home.join(rel)));
    }
    // An npm global install lands here, and npm puts this directory on `PATH`
    // only for shells started after the install.
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }
    dirs
}

/// Starts `claude auth login` in a console window of its own.
///
/// `auth login` rather than `login`: the latter is not a subcommand, so the
/// CLI would take it for a prompt and open an ordinary chat session.
pub fn spawn_login(exe: &Path) -> std::io::Result<()> {
    let mut command = launcher(exe, "auth login");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    command.spawn().map(|_| ())
}

/// A command that runs `exe args`, through a shell only where one is needed.
///
/// A `.cmd` shim is a batch file, which `CreateProcess` will not take as its
/// target, so that one goes through `cmd`. An `.exe` is started directly,
/// which leaves the console attached to the CLI rather than to a shell.
fn launcher(exe: &Path, args: &str) -> std::process::Command {
    #[cfg(windows)]
    if is_shim(exe) {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("cmd");
        command.arg("/C");
        command.raw_arg(shim_line(exe, args));
        return command;
    }
    let mut command = std::process::Command::new(exe);
    command.args(args.split_whitespace());
    command
}

/// The whole of what follows `cmd /C`, quoted the way `cmd` wants it.
///
/// `/C` drops the outermost pair of quotes from the rest of the line, so the
/// path is given a pair of its own inside them and survives a home directory
/// with a space in its name. Built as a string rather than passed as arguments
/// because Rust quotes arguments for `CreateProcess`, whose rules are not the
/// ones `cmd` then applies to the same line.
#[cfg(windows)]
fn shim_line(exe: &Path, args: &str) -> String {
    format!("\"\"{}\" {args}\"", exe.display())
}

/// Whether Windows needs a shell to run this one.
#[cfg(windows)]
fn is_shim(exe: &Path) -> bool {
    exe.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
}

/// Opens the install instructions in the default browser.
pub fn open_install_docs() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `start` needs a title argument before a quoted URL, or it reads the
        // URL as the title and opens a console instead.
        std::process::Command::new("cmd")
            .arg("/C")
            .raw_arg(format!("start \"\" \"{INSTALL_DOCS}\""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(INSTALL_DOCS)
            .spawn()
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory with `name` in it, under the test's own temp directory.
    fn dir_holding(case: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("cuw-cli-tests").join(case);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), "").unwrap();
        dir
    }

    #[test]
    fn finds_the_executable_by_name() {
        let dir = dir_holding("found", NAMES[0]);
        assert_eq!(named_in(&dir), Some(dir.join(NAMES[0])));
    }

    #[test]
    fn a_directory_without_it_yields_nothing() {
        let dir = dir_holding("absent", "something-else");
        assert_eq!(named_in(&dir), None);
    }

    #[test]
    fn the_known_directories_are_absolute() {
        // A relative entry would resolve against the working directory, which
        // for a widget launched from a shortcut is anyone's guess.
        assert!(known_dirs().iter().all(|dir| dir.is_absolute()));
    }

    #[cfg(windows)]
    #[test]
    fn only_batch_files_need_a_shell() {
        assert!(is_shim(Path::new(r"C:\npm\claude.cmd")));
        assert!(is_shim(Path::new(r"C:\npm\CLAUDE.CMD")));
        assert!(!is_shim(Path::new(r"C:\bin\claude.exe")));
    }

    #[cfg(windows)]
    #[test]
    fn a_shim_path_with_a_space_keeps_its_quotes() {
        // `cmd /C` drops the outer pair, and must be left with the path still
        // quoted or the command breaks at the space.
        let line = shim_line(Path::new(r"C:\Users\A B\npm\claude.cmd"), "auth login");
        assert_eq!(line, r#"""C:\Users\A B\npm\claude.cmd" auth login""#);
        assert_eq!(
            strip_outer_quotes(&line),
            r#""C:\Users\A B\npm\claude.cmd" auth login"#
        );
    }

    /// What `cmd /C` does to the line before running it.
    #[cfg(windows)]
    fn strip_outer_quotes(line: &str) -> &str {
        line.strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(line)
    }

    #[cfg(windows)]
    #[test]
    fn an_executable_is_run_without_a_shell() {
        let command = launcher(Path::new(r"C:\bin\claude.exe"), "auth login");
        assert_eq!(
            command.get_program(),
            std::ffi::OsStr::new(r"C:\bin\claude.exe")
        );
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["auth", "login"]);
    }
}
