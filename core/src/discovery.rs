//! Locating every `projects` directory Claude Code writes session logs into.
//!
//! On this machine the same user drives Claude Code from two places, and each
//! keeps its own log tree:
//!
//!   * Windows  -> `C:\Users\<user>\.claude\projects`
//!   * WSL      -> `/home/<user>/.claude/projects`, reachable from Windows
//!     as `\\wsl.localhost\<distro>\home\<user>\.claude\projects`
//!
//! Both trees use the same JSONL record format, but they encode project
//! directory names differently (`d--sandbox-foo` vs `-home-user-sandbox-foo`),
//! so callers must treat a root as opaque and aggregate across roots.

use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The two spellings Windows offers for the WSL file share. `wsl.localhost` is
/// the modern one; `wsl$` still works on older builds.
#[cfg(windows)]
const WSL_UNC_PREFIXES: [&str; 2] = [r"\\wsl.localhost", r"\\wsl$"];

/// Config directory layouts Claude Code has shipped, relative to a home dir.
const CONFIG_BASES: [&str; 2] = [".claude", ".config/claude"];

/// Where a set of session logs came from. Surfaced to the UI so a total can be
/// broken down per source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// The OS the widget itself is running on.
    Native,
    /// A WSL distribution, read across the filesystem boundary.
    Wsl,
    /// Named explicitly by `CLAUDE_CONFIG_DIR`.
    Explicit,
}

/// One discovered `projects` directory.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataRoot {
    /// Directory holding one subdirectory per project.
    pub projects_dir: PathBuf,
    pub origin: Origin,
    /// Short label for the UI, e.g. `Windows` or `WSL: <distro>`.
    pub label: String,
}

/// Every readable log root on this machine, in priority order: explicit
/// override first, then the native home, then each WSL distribution.
///
/// Roots are de-duplicated by canonical path, so an override that happens to
/// name the native directory is not counted twice.
pub fn discover() -> Vec<DataRoot> {
    discover_with(&[])
}

/// As [`discover`], plus config directories the caller knows about.
///
/// The widget keeps its own list of these, because `CLAUDE_CONFIG_DIR` cannot
/// carry one: the CLI reads the whole value as a single path, so setting it to
/// several would give this widget its accounts and take the CLI's away.
///
/// They go first, like the environment override, so a directory named twice is
/// counted once and labelled by the caller's spelling of it.
pub fn discover_with(extra: &[PathBuf]) -> Vec<DataRoot> {
    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();

    for root in extra.iter().filter_map(|dir| config_root(dir)) {
        push_unique(&mut roots, &mut seen, root);
    }
    for root in explicit_roots() {
        push_unique(&mut roots, &mut seen, root);
    }
    for root in native_roots() {
        push_unique(&mut roots, &mut seen, root);
    }
    for root in foreign_roots() {
        push_unique(&mut roots, &mut seen, root);
    }
    roots
}

/// One root for a directory named outright, or nothing if it holds no
/// `projects` tree yet. A config directory the CLI has never written a session
/// into has none, which is why anything adding one creates it.
fn config_root(dir: &Path) -> Option<DataRoot> {
    let projects = dir.join("projects");
    projects.is_dir().then(|| DataRoot {
        label: format!("Config: {}", dir.display()),
        projects_dir: projects,
        origin: Origin::Explicit,
    })
}

fn push_unique(roots: &mut Vec<DataRoot>, seen: &mut BTreeSet<PathBuf>, root: DataRoot) {
    // UNC paths do not always canonicalize, so fall back to the literal path
    // rather than dropping the root.
    let key = root
        .projects_dir
        .canonicalize()
        .unwrap_or_else(|_| root.projects_dir.clone());
    if seen.insert(key) {
        roots.push(root);
    }
}

/// `CLAUDE_CONFIG_DIR` as this widget reads it: one directory, or several
/// separated by a comma or a semicolon.
///
/// The CLI itself takes the whole value as a single path and does not split it,
/// so a list here is this widget's convention alone. Setting one for the
/// widget's benefit would leave the CLI looking for a directory named after
/// both, which reads as being signed out. Semicolon is accepted alongside comma
/// because it is the native Windows list separator, and a bare `:` cannot be
/// one (drive letters).
fn explicit_roots() -> Vec<DataRoot> {
    let Ok(raw) = std::env::var("CLAUDE_CONFIG_DIR") else {
        return Vec::new();
    };
    raw.split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|dir| config_root(Path::new(dir)))
        .collect()
}

/// Log roots under the home directory of the OS we are running on.
fn native_roots() -> Vec<DataRoot> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    projects_dirs_under(&home)
        .into_iter()
        .map(|projects_dir| DataRoot {
            projects_dir,
            origin: Origin::Native,
            label: native_label(),
        })
        .collect()
}

#[cfg(windows)]
fn native_label() -> String {
    "Windows".to_string()
}

#[cfg(not(windows))]
fn native_label() -> String {
    match std::env::var("WSL_DISTRO_NAME") {
        Ok(distro) if !distro.is_empty() => format!("WSL: {distro}"),
        _ => "Linux".to_string(),
    }
}

/// The home directory of the OS the widget is running on.
///
/// Public because the CLI itself is installed under it, and finding that is
/// the same question asked of the same two variables.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let keys = ["USERPROFILE", "HOME"];
    #[cfg(not(windows))]
    let keys = ["HOME"];

    keys.iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .find(|p| p.is_dir())
}

/// Each config layout under `home` that actually contains a `projects` tree.
fn projects_dirs_under(home: &Path) -> Vec<PathBuf> {
    CONFIG_BASES
        .iter()
        .map(|base| home.join(base).join("projects"))
        .filter(|p| p.is_dir())
        .collect()
}

/// Log roots that live on the *other* side of the Windows/WSL boundary.
#[cfg(windows)]
fn foreign_roots() -> Vec<DataRoot> {
    let mut roots = Vec::new();
    for distro in wsl_distro_names() {
        // Only the first prefix that resolves is used, so a distro reachable
        // under both spellings is not reported twice.
        for prefix in WSL_UNC_PREFIXES {
            let share = PathBuf::from(prefix).join(&distro);
            if !share.is_dir() {
                continue;
            }
            let found: Vec<_> = wsl_home_dirs(&share)
                .iter()
                .flat_map(|home| projects_dirs_under(home))
                .map(|projects_dir| DataRoot {
                    projects_dir,
                    origin: Origin::Wsl,
                    label: format!("WSL: {distro}"),
                })
                .collect();
            if !found.is_empty() {
                roots.extend(found);
                break;
            }
        }
    }
    roots
}

/// Candidate home directories inside a mounted WSL distribution: every entry
/// under `/home`, plus `/root` for distros used as root.
#[cfg(windows)]
fn wsl_home_dirs(share: &Path) -> Vec<PathBuf> {
    let mut homes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(share.join("home")) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                homes.push(entry.path());
            }
        }
    }
    let root_home = share.join("root");
    if root_home.is_dir() {
        homes.push(root_home);
    }
    homes
}

/// Installed distribution names, read from the Lxss registry key. Falls back to
/// enumerating the share root, which lists only *running* distros — hence the
/// registry is tried first.
#[cfg(windows)]
fn wsl_distro_names() -> Vec<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let mut names = BTreeSet::new();
    if let Ok(lxss) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Lxss")
    {
        for guid in lxss.enum_keys().flatten() {
            if let Ok(distro) = lxss.open_subkey(&guid) {
                if let Ok(name) = distro.get_value::<String, _>("DistributionName") {
                    names.insert(name);
                }
            }
        }
    }

    if names.is_empty() {
        for prefix in WSL_UNC_PREFIXES {
            if let Ok(entries) = std::fs::read_dir(prefix) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        names.insert(name.to_string());
                    }
                }
                if !names.is_empty() {
                    break;
                }
            }
        }
    }
    names.into_iter().collect()
}

/// Running under WSL, the Windows logs are reachable through the DrvFs mounts.
/// This keeps the widget symmetric when developed or run from inside Linux.
#[cfg(not(windows))]
fn foreign_roots() -> Vec<DataRoot> {
    let mut roots = Vec::new();
    for drive in ["/mnt/c", "/mnt/d"] {
        let users = Path::new(drive).join("Users");
        let Ok(entries) = std::fs::read_dir(&users) else {
            continue;
        };
        for entry in entries.flatten() {
            let home = entry.path();
            if !home.is_dir() || is_system_profile(&entry.file_name().to_string_lossy()) {
                continue;
            }
            for projects_dir in projects_dirs_under(&home) {
                roots.push(DataRoot {
                    projects_dir,
                    origin: Origin::Native,
                    label: "Windows".to_string(),
                });
            }
        }
    }
    roots
}

/// Built-in Windows profiles that never hold a real user's logs.
#[cfg(not(windows))]
fn is_system_profile(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "public" | "default" | "default user" | "all users"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_roots_are_collapsed() {
        let mut roots = Vec::new();
        let mut seen = BTreeSet::new();
        let root = |origin| DataRoot {
            projects_dir: PathBuf::from("/tmp"),
            origin,
            label: "x".into(),
        };
        push_unique(&mut roots, &mut seen, root(Origin::Explicit));
        push_unique(&mut roots, &mut seen, root(Origin::Native));
        assert_eq!(roots.len(), 1);
        // First writer wins, so the priority order in `discover` is preserved.
        assert_eq!(roots[0].origin, Origin::Explicit);
    }

    #[test]
    fn distinct_roots_are_both_kept() {
        let mut roots = Vec::new();
        let mut seen = BTreeSet::new();
        for dir in ["/tmp", "/usr"] {
            push_unique(
                &mut roots,
                &mut seen,
                DataRoot {
                    projects_dir: PathBuf::from(dir),
                    origin: Origin::Native,
                    label: "x".into(),
                },
            );
        }
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn missing_config_dir_override_is_ignored() {
        // A non-existent directory must not produce a root.
        std::env::set_var("CLAUDE_CONFIG_DIR", "/nonexistent-claude-dir-xyz");
        assert!(explicit_roots().is_empty());
        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn a_named_config_directory_becomes_a_root() {
        let dir = std::env::temp_dir().join("cuw-discovery-named");
        std::fs::create_dir_all(dir.join("projects")).unwrap();
        let roots = discover_with(std::slice::from_ref(&dir));
        let named = roots
            .iter()
            .find(|r| r.projects_dir == dir.join("projects"))
            .expect("the named directory should be a root");
        assert_eq!(named.origin, Origin::Explicit);
    }

    #[test]
    fn a_named_directory_without_a_projects_tree_is_skipped() {
        // A config directory the CLI has only ever signed into has no
        // `projects`, which is exactly the case that must not look like a
        // working account.
        let dir = std::env::temp_dir().join("cuw-discovery-bare");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_dir_all(dir.join("projects"));
        let roots = discover_with(std::slice::from_ref(&dir));
        assert!(!roots.iter().any(|r| r.projects_dir.starts_with(&dir)));
    }

    #[test]
    fn naming_the_same_directory_twice_yields_one_root() {
        let dir = std::env::temp_dir().join("cuw-discovery-twice");
        std::fs::create_dir_all(dir.join("projects")).unwrap();
        let roots = discover_with(&[dir.clone(), dir.clone()]);
        let hits = roots
            .iter()
            .filter(|r| r.projects_dir == dir.join("projects"))
            .count();
        assert_eq!(hits, 1);
    }
}
