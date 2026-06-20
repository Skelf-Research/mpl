//! Small utility helpers shared across crates.

use std::path::PathBuf;

/// Expand a leading `~/` to the user's home directory.
///
/// Paths that don't start with `~/`, or runs where `$HOME` is unset, pass
/// through unchanged. Used so that `--data-dir "~/.mpl"` doesn't create a
/// literal `./~/.mpl` directory wherever the binary happens to be invoked.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_leading_tilde_slash() {
        // SAFETY: tests are single-threaded by default; set HOME locally.
        unsafe { std::env::set_var("HOME", "/tmp/fakehome") };
        assert_eq!(expand_tilde("~/.mpl"), PathBuf::from("/tmp/fakehome/.mpl"));
        assert_eq!(
            expand_tilde("~/.mpl/qom/events.jsonl"),
            PathBuf::from("/tmp/fakehome/.mpl/qom/events.jsonl")
        );
    }

    #[test]
    fn passes_absolute_paths_through() {
        assert_eq!(expand_tilde("/var/lib/mpl"), PathBuf::from("/var/lib/mpl"));
    }

    #[test]
    fn passes_relative_paths_through() {
        assert_eq!(expand_tilde("data/mpl"), PathBuf::from("data/mpl"));
    }

    #[test]
    fn bare_tilde_without_slash_is_not_expanded() {
        // We deliberately only expand `~/`, not `~` alone or `~user`.
        assert_eq!(expand_tilde("~foo"), PathBuf::from("~foo"));
    }
}
