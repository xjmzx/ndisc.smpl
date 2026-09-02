//! Resolving the external tools this suite shells out to — ffmpeg, ffprobe
//! and aubio — to an absolute path before spawning them.
//!
//! VENDORED: byte-identical in nplay, nsmpl and ntree. Change it in one and
//! copy it to the others; `diff` the three before finishing.
//!
//! Why this exists. `Command::new("ffmpeg")` searches the *process* PATH, and
//! a macOS .app launched from Finder, Spotlight or the Dock does not inherit a
//! login shell's PATH. It inherits launchd's, which on a stock system is
//! `/usr/bin:/bin:/usr/sbin:/sbin` — Homebrew's `/opt/homebrew/bin` is not on
//! it. So `brew install ffmpeg` puts the binary somewhere the app cannot see,
//! and the app then reports "ffmpeg not found on PATH": true, and useless. The
//! user did install it.
//!
//! The reason this stayed hidden is that every way a developer runs the app
//! DOES work. `cargo run`, `npm run tauri dev` and launching the binary from a
//! terminal all inherit the shell's PATH. The fault appears only once the app
//! is in /Applications and launched the way a user launches it.
//!
//! Linux is unaffected in practice — ffmpeg installs to /usr/bin, which is on
//! every PATH including a .desktop launch's. These apps were developed on
//! Linux, which is why nobody hit this.
//!
//! Windows is left to PATH alone: there is no equivalent well-known location
//! to guess at, and installers there do put their tools on PATH.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Directories searched *after* PATH, for the case above. Order matters:
/// Homebrew's arm64 prefix first, then its Intel/x86 prefix, then MacPorts.
#[cfg(target_os = "macos")]
const EXTRA_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"];

/// Kept deliberately short. These are already on any sane PATH; they are here
/// only so a stripped environment (a .desktop entry with a scrubbed PATH, a
/// container) still resolves rather than failing with a confusing message.
#[cfg(target_os = "linux")]
const EXTRA_DIRS: &[&str] = &["/usr/local/bin", "/usr/bin", "/bin", "/snap/bin"];

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const EXTRA_DIRS: &[&str] = &[];

/// Only *successful* resolutions are cached. A negative result is not, on
/// purpose: someone who hits the "not found" message will go and install the
/// tool, and caching the miss would keep the app lying to them until they
/// restarted it. A miss costs a handful of `stat` calls.
fn cache() -> &'static Mutex<HashMap<String, PathBuf>> {
    static CACHE: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// An env var to force one tool's location, e.g. `NDISC_TOOL_FFMPEG=/path/ffmpeg`.
/// The escape hatch for a machine where the tool lives somewhere unguessable,
/// and the way to test a specific build. Checked before anything else.
fn env_override(name: &str) -> Option<PathBuf> {
    let var = format!("NDISC_TOOL_{}", name.to_uppercase());
    let raw = std::env::var_os(var)?;
    let p = PathBuf::from(raw);
    is_executable(&p).then_some(p)
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// `name` is bare ("ffmpeg"); the platform's executable suffix is appended, so
/// this resolves ffmpeg.exe on Windows without every call site knowing.
fn candidate(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// Absolute path to `name`, or None. PATH first so a user's own build or a
/// version manager still wins over whatever is in /opt/homebrew.
pub fn resolve(name: &str) -> Option<PathBuf> {
    if let Some(hit) = cache().lock().ok().and_then(|c| c.get(name).cloned()) {
        return Some(hit);
    }
    let found = env_override(name)
        .or_else(|| {
            std::env::var_os("PATH").and_then(|path| {
                std::env::split_paths(&path)
                    .map(|dir| candidate(&dir, name))
                    .find(|p| is_executable(p))
            })
        })
        .or_else(|| {
            EXTRA_DIRS
                .iter()
                .map(|dir| candidate(Path::new(dir), name))
                .find(|p| is_executable(p))
        })?;
    if let Ok(mut c) = cache().lock() {
        c.insert(name.to_string(), found.clone());
    }
    Some(found)
}

/// What to tell the user to install. `ffprobe` ships inside the ffmpeg
/// package, so it must not send them looking for a package called "ffprobe".
fn install_hint(name: &str) -> &'static str {
    match name {
        "ffmpeg" | "ffprobe" => {
            if cfg!(target_os = "macos") {
                "install it with `brew install ffmpeg`"
            } else {
                "install it with your package manager (Debian/Ubuntu: `apt install ffmpeg`)"
            }
        }
        "aubio" => {
            if cfg!(target_os = "macos") {
                "install it with `brew install aubio`"
            } else {
                "install it with your package manager (Debian/Ubuntu: `apt install aubio-tools`)"
            }
        }
        _ => "install it and make sure it is on PATH",
    }
}

/// The message shown when a tool cannot be found. It names the directories
/// actually searched, because on macOS the interesting case is a user who HAS
/// installed the tool and needs to see that the app looked somewhere else.
pub fn not_found_message(name: &str) -> String {
    let mut msg = format!("{name} not found — {}", install_hint(name));
    if !EXTRA_DIRS.is_empty() {
        msg.push_str(&format!(
            ". Searched PATH and {}. If it is installed somewhere else, set NDISC_TOOL_{}=/full/path/to/{name}",
            EXTRA_DIRS.join(", "),
            name.to_uppercase(),
        ));
    }
    msg
}

/// A `Command` for `name` with its path already resolved, or a message fit to
/// show the user. Call sites keep their own failure handling for everything
/// that can go wrong *after* a successful spawn.
///
/// `allow(dead_code)` because this module is vendored across three apps that
/// use different parts of it: nplay and nsmpl call this, while ntree's spawn
/// sites sit in functions returning outcome enums rather than `Result`, so it
/// wraps `resolve` in its own infallible `tool_cmd` instead. Unused here is
/// expected, not rot.
#[allow(dead_code)]
pub fn command(name: &str) -> Result<Command, String> {
    match resolve(name) {
        Some(path) => Ok(Command::new(path)),
        None => Err(not_found_message(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `sh` is on PATH on every unix, so this asserts real resolution rather
    // than mocking one.
    #[cfg(unix)]
    #[test]
    fn resolves_a_binary_that_exists() {
        let p = resolve("sh").expect("sh should resolve on unix");
        assert!(p.is_absolute(), "resolved path must be absolute: {p:?}");
        assert!(is_executable(&p));
    }

    #[test]
    fn missing_binary_resolves_to_none() {
        assert!(resolve("ndisc-definitely-not-a-real-binary").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn env_override_wins_over_path() {
        // Safety: single-threaded within this test; the var is unique to it.
        std::env::set_var("NDISC_TOOL_NDISCTESTTOOL", "/bin/sh");
        let p = resolve("ndisctesttool").expect("override should resolve");
        assert_eq!(p, PathBuf::from("/bin/sh"));
        std::env::remove_var("NDISC_TOOL_NDISCTESTTOOL");
    }

    #[test]
    fn message_names_the_package_not_the_binary() {
        // ffprobe ships inside ffmpeg; pointing at a package called "ffprobe"
        // would send the user somewhere that does not exist.
        assert!(not_found_message("ffprobe").contains("ffmpeg"));
    }
}
