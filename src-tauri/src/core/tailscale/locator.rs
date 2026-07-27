//! Finding the `tailscale` executable.
//!
//! The installer does not reliably put it on `PATH` on either platform, and
//! macOS has two very different layouts depending on whether Tailscale came
//! from the App Store or the standalone package. Both the dependency check
//! and the client resolve the path through here so they can never disagree.

use std::path::PathBuf;

use crate::core::process;

#[cfg(windows)]
const FALLBACK_PATHS: &[&str] = &[
    r"C:\Program Files\Tailscale\tailscale.exe",
    r"C:\Program Files (x86)\Tailscale\tailscale.exe",
];

#[cfg(target_os = "macos")]
const FALLBACK_PATHS: &[&str] = &[
    "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
    "/usr/local/bin/tailscale",
    "/opt/homebrew/bin/tailscale",
];

#[cfg(not(any(windows, target_os = "macos")))]
const FALLBACK_PATHS: &[&str] = &["/usr/bin/tailscale", "/usr/local/bin/tailscale"];

/// Returns the path to the Tailscale CLI, or `None` when it is not installed.
pub fn find() -> Option<PathBuf> {
    process::locate("tailscale", FALLBACK_PATHS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fallback_is_an_absolute_path() {
        assert!(!FALLBACK_PATHS.is_empty());
        assert!(FALLBACK_PATHS
            .iter()
            .all(|path| std::path::Path::new(path).is_absolute()));
    }

    #[test]
    fn find_does_not_panic_regardless_of_machine_state() {
        let _ = find();
    }
}
