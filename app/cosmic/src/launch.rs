//! Opening documents named on the command line or handed over by another
//! program (file manager "Open with", Stack's "Open in Ami Pro").
//!
//! The mechanism is libcosmic's single-instance support, which is the
//! desktop's own D-Bus activation (`org.freedesktop.DbusActivation` on the
//! session bus, name and path taken from the app id):
//!
//! - a launch with file arguments, while a window is running, sends the
//!   paths to that window (`ActivateAction("open", paths)`) and exits;
//! - a launcher that speaks the Desktop Entry `DBusActivatable` protocol
//!   (the file manager) calls `Open(uris)` on the same object.
//!
//! Relative paths are made absolute here, in the launching process, because
//! the running window has a different working directory.

use std::path::{Path, PathBuf};

use cosmic::app::CosmicFlags;

/// The action name sent to a running window.
pub const OPEN_ACTION: &str = "open";

/// What the command line asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Launch {
    paths: Vec<PathBuf>,
    /// The paths as text, kept for the D-Bus call.
    texts: Vec<String>,
}

impl Launch {
    /// The files named by the arguments after the program name. `--` and
    /// empty arguments are skipped; the rest are taken as paths (or
    /// `file://` addresses, which a launcher may pass for `%u`).
    pub fn from_args<I, S>(args: I, current_dir: &Path) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString>,
    {
        let mut paths = Vec::new();
        for arg in args {
            let arg: std::ffi::OsString = arg.into();
            if arg.is_empty() || arg == "--" {
                continue;
            }
            paths.push(absolute(&from_address(&arg), current_dir));
        }
        let texts = paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        Self { paths, texts }
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

impl CosmicFlags for Launch {
    type SubCommand = String;
    type Args = Vec<String>;

    fn action(&self) -> Option<&String> {
        // Only a launch that names files is an "open"; a bare launch just
        // raises the running window.
        static OPEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        (!self.paths.is_empty()).then(|| OPEN.get_or_init(|| OPEN_ACTION.to_owned()))
    }

    fn args(&self) -> Vec<&str> {
        self.texts.iter().map(String::as_str).collect()
    }
}

/// A `file://` address as a path; anything else is the path itself.
fn from_address(arg: &std::ffi::OsStr) -> PathBuf {
    arg.to_str()
        .filter(|text| text.starts_with("file:"))
        .and_then(|text| url::Url::parse(text).ok())
        .and_then(|url| url.to_file_path().ok())
        .unwrap_or_else(|| PathBuf::from(arg))
}

fn absolute(path: &Path, current_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

/// The paths a running window is asked to open: from `Open(uris)` (only
/// `file:` addresses mean a file) or from the `"open"` action's arguments.
pub fn paths_from_uris(uris: &[url::Url]) -> Vec<PathBuf> {
    uris.iter()
        .filter_map(|url| url.to_file_path().ok())
        .collect()
}

/// What an incoming D-Bus activation asks to open.
pub fn paths_from_activation(details: &cosmic::dbus_activation::Details) -> Vec<PathBuf> {
    use cosmic::dbus_activation::Details;
    match details {
        Details::Open { url } => paths_from_uris(url),
        Details::ActivateAction { action, args } => paths_from_action(action, args),
        Details::Activate => Vec::new(),
    }
}

pub fn paths_from_action(action: &str, args: &[String]) -> Vec<PathBuf> {
    if action != OPEN_ACTION {
        return Vec::new();
    }
    args.iter()
        .filter(|arg| !arg.is_empty())
        .map(|arg| absolute(&from_address(arg.as_ref()), Path::new("/")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_and_absolute_paths_both_become_absolute() {
        let launch = Launch::from_args(["a/b.sam", "/x/y.SAM"], Path::new("/work"));
        assert_eq!(
            launch.paths(),
            [PathBuf::from("/work/a/b.sam"), PathBuf::from("/x/y.SAM")]
        );
        assert_eq!(launch.action().map(String::as_str), Some("open"));
        assert_eq!(launch.args(), ["/work/a/b.sam", "/x/y.SAM"]);
    }

    #[test]
    fn no_files_is_a_plain_activation() {
        let launch = Launch::from_args(Vec::<String>::new(), Path::new("/work"));
        assert!(launch.action().is_none());
        assert!(launch.args().is_empty());
        assert!(
            Launch::from_args(["--", ""], Path::new("/w"))
                .paths()
                .is_empty()
        );
    }

    #[test]
    fn file_addresses_are_decoded() {
        let launch = Launch::from_args(["file:///tmp/My%20Doc.sam"], Path::new("/work"));
        assert_eq!(launch.paths(), [PathBuf::from("/tmp/My Doc.sam")]);
        let url = url::Url::parse("file:///tmp/a%20b.sam").unwrap();
        assert_eq!(paths_from_uris(&[url]), [PathBuf::from("/tmp/a b.sam")]);
        assert!(paths_from_uris(&[url::Url::parse("https://x/y.sam").unwrap()]).is_empty());
    }

    #[test]
    fn activations_map_to_paths() {
        use cosmic::dbus_activation::Details;
        let open = Details::Open {
            url: vec![url::Url::parse("file:///d/e.sam").unwrap()],
        };
        assert_eq!(paths_from_activation(&open), [PathBuf::from("/d/e.sam")]);
        let action = Details::ActivateAction {
            action: "open".into(),
            args: vec!["/f/g.sam".into()],
        };
        assert_eq!(paths_from_activation(&action), [PathBuf::from("/f/g.sam")]);
        assert!(paths_from_activation(&Details::Activate).is_empty());
    }

    #[test]
    fn only_the_open_action_names_files() {
        let args = vec!["/a/b.sam".to_owned()];
        assert_eq!(
            paths_from_action("open", &args),
            [PathBuf::from("/a/b.sam")]
        );
        assert!(paths_from_action("other", &args).is_empty());
    }
}
