//! Running external programs without flashing console windows at the user.
//!
//! Every subprocess syncparty starts — `tailscale`, `winget`, `uv`, `python`,
//! the Syncplay client — goes through here. On Windows a bare
//! [`tokio::process::Command`] pops a console window for a fraction of a
//! second, which looks broken in a GUI app; `CREATE_NO_WINDOW` suppresses it.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::core::error::{Result, SyncPartyError};

/// Windows process creation flag that suppresses the console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Builds a [`Command`] that stays invisible and captures both output streams.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

/// Builds a [`Command`] for a process syncparty keeps running and reads from
/// line by line, rather than waiting on to completion.
pub fn spawnable(program: impl AsRef<OsStr>) -> Command {
    command(program)
}

pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program to completion and returns its output, failing on a non-zero
/// exit status.
///
/// Use [`try_capture`] when a non-zero status is an expected answer rather
/// than an error — probing for an uninstalled tool, for instance.
pub async fn capture<I, S>(program: impl AsRef<OsStr>, args: I) -> Result<CapturedOutput>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let program_name = program.as_ref().to_string_lossy().into_owned();
    let output = command(program).args(args).output().await.map_err(|error| {
        SyncPartyError::CommandFailed {
            command: program_name.clone(),
            status: "could not start".to_owned(),
            stderr: error.to_string(),
        }
    })?;

    let captured = CapturedOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };

    if !output.status.success() {
        return Err(SyncPartyError::CommandFailed {
            command: program_name,
            status: output.status.to_string(),
            stderr: first_meaningful_line(&captured.stderr, &captured.stdout),
        });
    }

    Ok(captured)
}

/// Like [`capture`], but a failed run yields `None` instead of an error.
pub async fn try_capture<I, S>(program: impl AsRef<OsStr>, args: I) -> Option<CapturedOutput>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    capture(program, args).await.ok()
}

/// Resolves an executable, preferring `PATH` and falling back to a list of
/// well-known install locations.
///
/// Tailscale, mpv and Syncplay all install somewhere predictable but do not
/// reliably add themselves to `PATH`, so looking in both places is what makes
/// detection work on a stock machine.
pub fn locate(binary: &str, fallbacks: &[&str]) -> Option<std::path::PathBuf> {
    if let Ok(found) = which::which(binary) {
        return Some(found);
    }

    fallbacks
        .iter()
        .map(Path::new)
        .find(|candidate| candidate.is_file())
        .map(Path::to_path_buf)
}

/// Asks a program for its version, returning `None` if it will not say.
///
/// Only ever used to decorate a "found it" message, so every failure mode —
/// the flag is unsupported, the tool hangs on stdin, the output is empty —
/// collapses to `None` rather than an error.
pub async fn probe_version(executable: &Path, args: &[&str]) -> Option<String> {
    let output = try_capture(executable, args).await?;

    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

/// Picks the most useful line to show the user when a command fails. Some
/// tools write the real reason to stdout and leave stderr empty.
fn first_meaningful_line(stderr: &str, stdout: &str) -> String {
    stderr
        .lines()
        .chain(stdout.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_stderr_then_falls_back_to_stdout() {
        assert_eq!(first_meaningful_line("  \nboom\n", "ignored"), "boom");
        assert_eq!(first_meaningful_line("", "  \nfallback"), "fallback");
        assert_eq!(first_meaningful_line("", ""), "no output");
    }

    #[test]
    fn locate_returns_none_when_nothing_matches() {
        assert!(locate("syncparty-does-not-exist", &["/nope/also-not-here"]).is_none());
    }
}
