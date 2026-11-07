//! Help › Check for Updates… (P6.2): the logic, with no window in it.
//!
//! Updates are the GitHub Releases of the public repository (no token). A
//! release carries `amipro-linux_<version>_amd64.deb` and `SHA256SUMS`. The
//! program checks only when asked (or once a day at start-up when
//! Preferences says so), downloads the package into a private directory,
//! verifies its SHA-256 against the release's own list (mandatory), and only
//! then, after the user's confirmation, hands it to the system installer
//! with `pkexec apt-get install -y <file>`. It runs nothing else.
//!
//! Everything that touches the network or the system goes through
//! [`Backend`], so tests use a fake. [`RealBackend`] uses `curl` as a
//! subprocess (HTTPS only, timeouts, a size cap, redirects followed by hand
//! and only to GitHub's hosts).

use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Where the newest release is described.
pub const LATEST_URL: &str = "https://api.github.com/repos/nchiker/AmiPro-Linux/releases/latest";
/// The only place a release's files may come from.
pub const DOWNLOAD_PREFIX: &str = "https://github.com/nchiker/AmiPro-Linux/releases/download/";
/// The checksum list's asset name.
pub const SUMS_NAME: &str = "SHA256SUMS";

/// Size caps: the release description, the checksum list, the package.
pub const MAX_JSON: u64 = 1 << 20;
pub const MAX_SUMS: u64 = 64 << 10;
pub const MAX_DEB: u64 = 300 << 20;

/// At most one start-up check a day.
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
/// The release notes are shown from their first lines.
pub const NOTES_LINES: usize = 20;

/// The version this program is.
pub fn installed_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate version is a version")
}

// ---------------------------------------------------------------- versions

/// One dot-separated pre-release identifier.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Ident {
    // Numeric identifiers sort before alphanumeric ones (semver 11.4.3).
    Number(u64),
    Text(String),
}

/// A semantic version (`1.2.3-rc.1+build`); a leading `v` is accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pre: Vec<Ident>,
    /// Build metadata (`+g<rev>`, `+git<rev>`): kept, never compared.
    pub build: String,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
        let (rest, build) = match text.split_once('+') {
            Some((rest, build)) => (rest, build),
            None => (text, ""),
        };
        if text.contains('+') && !valid_idents(build) {
            return None;
        }
        let (core, pre) = match rest.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (rest, None),
        };
        let mut numbers = core.split('.');
        let mut number = || -> Option<u64> {
            let part = numbers.next()?;
            let plain = !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            plain.then(|| part.parse().ok()).flatten()
        };
        let (major, minor, patch) = (number()?, number()?, number()?);
        if numbers.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => {
                if !valid_idents(pre) {
                    return None;
                }
                pre.split('.')
                    .map(|part| {
                        if part.bytes().all(|byte| byte.is_ascii_digit()) {
                            part.parse().ok().map(Ident::Number)
                        } else {
                            Some(Ident::Text(part.to_owned()))
                        }
                    })
                    .collect::<Option<Vec<_>>>()?
            }
        };
        Some(Version {
            major,
            minor,
            patch,
            pre,
            build: build.to_owned(),
        })
    }

    /// Semver precedence: build metadata is ignored, and a pre-release sorts
    /// before its release.
    pub fn precedence(&self, other: &Version) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }

    pub fn is_newer_than(&self, other: &Version) -> bool {
        self.precedence(other) == Ordering::Greater
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            let pre: Vec<String> = self
                .pre
                .iter()
                .map(|ident| match ident {
                    Ident::Number(number) => number.to_string(),
                    Ident::Text(text) => text.clone(),
                })
                .collect();
            write!(out, "-{}", pre.join("."))?;
        }
        if !self.build.is_empty() {
            write!(out, "+{}", self.build)?;
        }
        Ok(())
    }
}

/// Dot-separated identifiers of `[0-9A-Za-z-]`, none empty.
fn valid_idents(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

// ---------------------------------------------------------------- releases

/// One file of a release.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// What the release API says, as far as the program uses it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    pub tag: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

/// A newer release the program can install.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Offer {
    pub version: String,
    /// The first [`NOTES_LINES`] lines of the release notes.
    pub notes: String,
    pub deb: Asset,
    pub sums: Asset,
}

/// What a check found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    UpToDate { latest: String },
    Newer(Offer),
}

/// Reads the `releases/latest` JSON.
pub fn parse_release(bytes: &[u8]) -> Result<Release, String> {
    let json: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "GitHub's answer could not be read".to_owned())?;
    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(|found| found.as_str())
            .map(str::to_owned)
    };
    let tag = text(&json, "tag_name").ok_or("GitHub's answer names no release")?;
    let assets = json
        .get("assets")
        .and_then(|assets| assets.as_array())
        .map(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    Some(Asset {
                        name: text(asset, "name")?,
                        url: text(asset, "browser_download_url")?,
                        size: asset
                            .get("size")
                            .and_then(|size| size.as_u64())
                            .unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release {
        tag,
        body: text(&json, "body").unwrap_or_default(),
        draft: json.get("draft").and_then(|v| v.as_bool()).unwrap_or(false),
        prerelease: json
            .get("prerelease")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        assets,
    })
}

/// The release notes' first lines, without control characters.
pub fn notes_excerpt(body: &str) -> String {
    let clean: String = body
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect();
    let lines: Vec<&str> = clean.lines().map(str::trim_end).collect();
    let mut shown = lines
        .iter()
        .take(NOTES_LINES)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if lines.len() > NOTES_LINES {
        shown.push_str("\n…");
    }
    shown.trim().to_owned()
}

/// The package's name for a version: `amipro-linux_<version>_amd64.deb`.
pub fn deb_name(version: &str) -> String {
    format!("amipro-linux_{version}_amd64.deb")
}

/// A name safe to use as a file name here.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+~-".contains(&byte))
}

/// Decides what a release means for the installed version.
pub fn judge(release: &Release, installed: &Version) -> Result<Outcome, String> {
    if release.draft {
        return Err("The latest release is a draft".into());
    }
    let latest = Version::parse(&release.tag)
        .ok_or_else(|| format!("The latest release's tag is not a version: {}", release.tag))?;
    if !latest.is_newer_than(installed) {
        return Ok(Outcome::UpToDate {
            latest: latest.to_string(),
        });
    }
    let version = release.tag.trim().trim_start_matches(['v', 'V']).to_owned();
    let wanted = deb_name(&version);
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name && asset.url.starts_with(DOWNLOAD_PREFIX))
            .cloned()
    };
    let deb = find(&wanted).filter(|asset| plain_name(&asset.name));
    let sums = find(SUMS_NAME);
    match (deb, sums) {
        (Some(deb), Some(sums)) => Ok(Outcome::Newer(Offer {
            version,
            notes: notes_excerpt(&release.body),
            deb,
            sums,
        })),
        (None, _) => Err(format!(
            "Version {version} is out, but its release has no {wanted}"
        )),
        (_, None) => Err(format!(
            "Version {version} is out, but its release has no {SUMS_NAME}, so it cannot be verified"
        )),
    }
}

// ---------------------------------------------------------------- checksums

/// SHA-256 of `bytes`, in lower-case hex.
#[cfg(test)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 of a file, read in pieces.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 << 10];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The digest `sums` (`sha256sum`'s output: `<hex>  <name>` or
/// `<hex> *<name>`) lists for `name`.
pub fn find_checksum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (digest, file) = line.trim_end().split_once(char::is_whitespace)?;
        let file = file.trim_start().trim_start_matches('*');
        (file == name && digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| digest.to_ascii_lowercase())
    })
}

// ---------------------------------------------------------------- the world

/// The network and the system, so that tests can stand in for them.
pub trait Backend: Send + Sync {
    /// A small answer, at most `max` bytes.
    fn get(&self, url: &str, max: u64) -> Result<Vec<u8>, String>;
    /// A file written to `dest`, at most `max` bytes.
    fn download(&self, url: &str, dest: &Path, max: u64) -> Result<(), String>;
    /// Whether `pkexec` and `apt-get` are there.
    fn installer_available(&self) -> bool;
    /// `pkexec apt-get install -y <deb>`.
    fn install(&self, deb: &Path) -> Result<(), String>;
    /// Starts a new instance of the program.
    fn restart(&self) -> Result<(), String>;
}

/// Asks the release API what the newest release is.
pub fn check(backend: &dyn Backend, installed: &Version) -> Result<Outcome, String> {
    let bytes = backend.get(LATEST_URL, MAX_JSON)?;
    judge(&parse_release(&bytes)?, installed)
}

/// Downloads the offer's package into `dir` and verifies it against the
/// release's checksum list. A package that does not match is deleted.
pub fn download_and_verify(
    backend: &dyn Backend,
    offer: &Offer,
    dir: &Path,
) -> Result<PathBuf, String> {
    if !plain_name(&offer.deb.name)
        || !offer.deb.url.starts_with(DOWNLOAD_PREFIX)
        || !offer.sums.url.starts_with(DOWNLOAD_PREFIX)
    {
        return Err("The release's files are not where they should be".into());
    }
    if offer.deb.size > MAX_DEB {
        return Err("The package is larger than this program will download".into());
    }
    let sums = backend.get(&offer.sums.url, MAX_SUMS)?;
    let sums = String::from_utf8_lossy(&sums);
    let expected = find_checksum(&sums, &offer.deb.name)
        .ok_or_else(|| format!("{SUMS_NAME} does not list {}", offer.deb.name))?;
    let dest = dir.join(&offer.deb.name);
    let discard = |message: String| {
        let _ = std::fs::remove_file(&dest);
        message
    };
    backend
        .download(&offer.deb.url, &dest, MAX_DEB)
        .map_err(discard)?;
    let length = std::fs::metadata(&dest)
        .map_err(|error| discard(format!("The download is missing: {error}")))?
        .len();
    if length > MAX_DEB || (offer.deb.size > 0 && length != offer.deb.size) {
        return Err(discard(
            "The download is not the size the release states; it was discarded".into(),
        ));
    }
    let found =
        sha256_file(&dest).map_err(|error| discard(format!("Reading it failed: {error}")))?;
    if found != expected {
        return Err(discard(
            "The download's checksum does not match the release's; it was discarded and nothing was installed"
                .into(),
        ));
    }
    Ok(dest)
}

/// A new directory only the user can enter, for the download.
pub fn private_dir() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    let dir = base.join(format!("amipro-update-{}-{stamp}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&dir)?;
    Ok(dir)
}

/// The exact command that installs `deb`, quoted for a shell.
pub fn install_command(deb: &Path) -> String {
    format!(
        "pkexec apt-get install -y {}",
        shell_quote(&deb.to_string_lossy())
    )
}

fn shell_quote(text: &str) -> String {
    if !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._+~:-".contains(&byte))
    {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', "'\\''"))
    }
}

/// Whether a start-up check is due: none yet, a day since the last, or a
/// last check in the future (the clock moved).
pub fn check_due(last: u64, now: u64) -> bool {
    last > now || now - last >= CHECK_INTERVAL_SECS
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_secs())
}

// ---------------------------------------------------------------- real world

/// Hosts a redirect may lead to.
pub fn allowed_host(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if host.contains('@') || host.contains(':') {
        return false;
    }
    host == "github.com"
        || host == "api.github.com"
        || host.ends_with(".githubusercontent.com")
        || host.ends_with(".github.com")
}

/// Where `name` is among `dirs`.
pub fn find_in(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    dirs.iter().map(|dir| dir.join(name)).find(|path| {
        std::fs::metadata(path)
            .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    })
}

fn path_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    dirs.extend(["/usr/bin", "/bin", "/usr/sbin"].map(PathBuf::from));
    dirs
}

/// `curl` and `pkexec`.
pub struct RealBackend;

impl RealBackend {
    /// One transfer into `dest`, redirects followed by hand (at most five,
    /// each to an allowed host, always HTTPS).
    fn transfer(url: &str, dest: &Path, max: u64, seconds: u32) -> Result<(), String> {
        let curl = find_in("curl", &path_dirs())
            .ok_or("curl is not installed, so updates cannot be checked")?;
        let mut url = url.to_owned();
        for _ in 0..=5 {
            if !allowed_host(&url) {
                return Err("The update server sent the program somewhere it will not go".into());
            }
            let output = std::process::Command::new(&curl)
                .args(["--silent", "--show-error", "--proto", "=https", "--tlsv1.2"])
                .args([
                    "--connect-timeout",
                    "15",
                    "--max-time",
                    &seconds.to_string(),
                ])
                .args(["--max-filesize", &max.to_string()])
                .args(["-H", "Accept: application/vnd.github+json"])
                .args(["-A", concat!("amipro-linux/", env!("CARGO_PKG_VERSION"))])
                .arg("-o")
                .arg(dest)
                .args(["-w", "%{http_code} %{redirect_url}"])
                .arg("--")
                .arg(&url)
                .stdin(std::process::Stdio::null())
                .output()
                .map_err(|error| format!("curl could not start: {error}"))?;
            let report = String::from_utf8_lossy(&output.stdout).into_owned();
            let (code, next) = report.split_once(' ').unwrap_or((report.as_str(), ""));
            if !output.status.success() {
                let _ = std::fs::remove_file(dest);
                let error = String::from_utf8_lossy(&output.stderr);
                let error = error.trim();
                return Err(if error.is_empty() {
                    "The connection failed".into()
                } else {
                    format!("The connection failed: {error}")
                });
            }
            match code {
                "200" => {
                    let length = std::fs::metadata(dest).map(|meta| meta.len()).unwrap_or(0);
                    if length > max {
                        let _ = std::fs::remove_file(dest);
                        return Err("The answer is larger than this program accepts".into());
                    }
                    return Ok(());
                }
                "301" | "302" | "303" | "307" | "308" if !next.trim().is_empty() => {
                    url = next.trim().to_owned();
                }
                "403" | "429" => {
                    let _ = std::fs::remove_file(dest);
                    return Err("GitHub is limiting requests just now; try again later".into());
                }
                "404" => {
                    let _ = std::fs::remove_file(dest);
                    return Err("There is no published release yet".into());
                }
                other => {
                    let _ = std::fs::remove_file(dest);
                    return Err(format!("GitHub answered with status {other}"));
                }
            }
        }
        Err("Too many redirects".into())
    }
}

impl Backend for RealBackend {
    fn get(&self, url: &str, max: u64) -> Result<Vec<u8>, String> {
        let dir = private_dir().map_err(|error| format!("No place for the answer: {error}"))?;
        let file = dir.join("answer");
        let result = Self::transfer(url, &file, max, 30)
            .and_then(|()| std::fs::read(&file).map_err(|error| error.to_string()));
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    fn download(&self, url: &str, dest: &Path, max: u64) -> Result<(), String> {
        Self::transfer(url, dest, max, 900)
    }

    fn installer_available(&self) -> bool {
        let dirs = path_dirs();
        find_in("pkexec", &dirs).is_some() && find_in("apt-get", &dirs).is_some()
    }

    fn install(&self, deb: &Path) -> Result<(), String> {
        let dirs = path_dirs();
        let pkexec = find_in("pkexec", &dirs).ok_or("pkexec is not installed")?;
        let apt = find_in("apt-get", &dirs).ok_or("apt-get is not installed")?;
        if !deb.is_absolute() {
            return Err("The package's path is not absolute".into());
        }
        let output = std::process::Command::new(pkexec)
            .arg(apt)
            .args(["install", "-y"])
            .arg(deb)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|error| format!("pkexec could not start: {error}"))?;
        match output.status.code() {
            Some(0) => Ok(()),
            Some(126) | Some(127) => Err("The installation was not authorised".into()),
            _ => {
                let error = String::from_utf8_lossy(&output.stderr);
                let last = error.lines().rev().find(|line| !line.trim().is_empty());
                Err(format!(
                    "The installer failed: {}",
                    last.unwrap_or("no message")
                ))
            }
        }
    }

    fn restart(&self) -> Result<(), String> {
        let installed = PathBuf::from("/usr/bin/amipro-linux");
        let program = if installed.is_file() {
            installed
        } else {
            std::env::current_exe().map_err(|error| error.to_string())?
        };
        std::process::Command::new(program)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|error| format!("The new version could not start: {error}"))
    }
}

/// The backend tests start with: nothing reaches the network or the system.
#[cfg(test)]
pub struct OfflineBackend;

#[cfg(test)]
impl Backend for OfflineBackend {
    fn get(&self, _: &str, _: u64) -> Result<Vec<u8>, String> {
        Err("offline in tests".into())
    }
    fn download(&self, _: &str, _: &Path, _: u64) -> Result<(), String> {
        Err("offline in tests".into())
    }
    fn installer_available(&self) -> bool {
        false
    }
    fn install(&self, _: &Path) -> Result<(), String> {
        Err("offline in tests".into())
    }
    fn restart(&self) -> Result<(), String> {
        Err("offline in tests".into())
    }
}

/// A scripted backend for the tests here and in the app.
#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct Fake {
        /// url -> answer
        pub files: Mutex<HashMap<String, Result<Vec<u8>, String>>>,
        pub installer: bool,
        pub installed: Mutex<Vec<PathBuf>>,
        pub install_error: Mutex<Option<String>>,
        pub restarts: Mutex<u32>,
        pub requests: Mutex<Vec<String>>,
    }

    impl Fake {
        pub fn serve(&self, url: &str, bytes: &[u8]) {
            self.files
                .lock()
                .unwrap()
                .insert(url.to_owned(), Ok(bytes.to_vec()));
        }
        pub fn fail(&self, url: &str, message: &str) {
            self.files
                .lock()
                .unwrap()
                .insert(url.to_owned(), Err(message.to_owned()));
        }
        fn answer(&self, url: &str) -> Result<Vec<u8>, String> {
            self.requests.lock().unwrap().push(url.to_owned());
            self.files
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err("not found".into()))
        }
    }

    impl Backend for Fake {
        fn get(&self, url: &str, max: u64) -> Result<Vec<u8>, String> {
            let bytes = self.answer(url)?;
            if bytes.len() as u64 > max {
                return Err("too large".into());
            }
            Ok(bytes)
        }
        fn download(&self, url: &str, dest: &Path, _: u64) -> Result<(), String> {
            let bytes = self.answer(url)?;
            std::fs::write(dest, bytes).map_err(|error| error.to_string())
        }
        fn installer_available(&self) -> bool {
            self.installer
        }
        fn install(&self, deb: &Path) -> Result<(), String> {
            if let Some(error) = self.install_error.lock().unwrap().clone() {
                return Err(error);
            }
            self.installed.lock().unwrap().push(deb.to_owned());
            Ok(())
        }
        fn restart(&self) -> Result<(), String> {
            *self.restarts.lock().unwrap() += 1;
            Ok(())
        }
    }

    /// A release JSON for `tag` whose assets are the package and the list.
    pub fn release_json(tag: &str, size: usize) -> String {
        let version = tag.trim_start_matches('v');
        format!(
            r#"{{"tag_name":"{tag}","draft":false,"prerelease":false,
"body":"Line one\nLine two","assets":[
{{"name":"amipro-linux_{version}_amd64.deb","size":{size},
"browser_download_url":"{DOWNLOAD_PREFIX}{tag}/amipro-linux_{version}_amd64.deb"}},
{{"name":"SHA256SUMS","size":100,
"browser_download_url":"{DOWNLOAD_PREFIX}{tag}/SHA256SUMS"}}]}}"#
        )
    }

    /// A fake serving release `tag` with package bytes `deb`; the list gives
    /// `listed` as the digest (the real one when `None`).
    pub fn serve_release(fake: &Fake, tag: &str, deb: &[u8], listed: Option<&str>) {
        let version = tag.trim_start_matches('v');
        fake.serve(LATEST_URL, release_json(tag, deb.len()).as_bytes());
        fake.serve(
            &format!("{DOWNLOAD_PREFIX}{tag}/amipro-linux_{version}_amd64.deb"),
            deb,
        );
        let digest = listed.map_or_else(|| sha256_hex(deb), str::to_owned);
        fake.serve(
            &format!("{DOWNLOAD_PREFIX}{tag}/SHA256SUMS"),
            format!("{digest}  amipro-linux_{version}_amd64.deb\n").as_bytes(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap_or_else(|| panic!("{text} parses"))
    }

    #[test]
    fn versions_parse_and_reject() {
        assert_eq!(v("0.1.0").to_string(), "0.1.0");
        assert_eq!(v("v1.2.3").to_string(), "1.2.3");
        assert_eq!(v("0.1.0+g1a2b3c4").build, "g1a2b3c4");
        assert_eq!(v("1.0.0-rc.1+git5").to_string(), "1.0.0-rc.1+git5");
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.2.x",
            "1.2.3-",
            "1.2.3+",
            "1.2.3-a..b",
        ] {
            assert!(Version::parse(bad).is_none(), "{bad:?} is not a version");
        }
    }

    #[test]
    fn precedence_follows_semver() {
        // The ordering from semver.org §11.
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.1.0",
            "2.0.0",
        ];
        for pair in ordered.windows(2) {
            assert!(v(pair[1]).is_newer_than(&v(pair[0])), "{pair:?}");
            assert!(!v(pair[0]).is_newer_than(&v(pair[1])), "{pair:?}");
        }
        assert!(v("0.10.0").is_newer_than(&v("0.9.9")));
    }

    #[test]
    fn build_metadata_never_decides() {
        assert!(!v("0.1.0").is_newer_than(&v("0.1.0+g1a2b3c4")));
        assert!(!v("0.1.0+g2").is_newer_than(&v("0.1.0+g1")));
        assert_eq!(v("0.1.0+a").precedence(&v("0.1.0+b")), Ordering::Equal);
        assert!(v("0.1.1").is_newer_than(&v("0.1.0+git9")));
    }

    #[test]
    fn a_prerelease_is_not_newer_than_its_release() {
        assert!(!v("1.0.0-rc.1").is_newer_than(&v("1.0.0+g1")));
        assert!(v("1.0.0").is_newer_than(&v("1.0.0-rc.1")));
    }

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        let dir = private_dir().unwrap();
        std::fs::write(dir.join("f"), b"abc").unwrap();
        assert_eq!(sha256_file(&dir.join("f")).unwrap(), sha256_hex(b"abc"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_checksum_list_is_read_like_sha256sum_writes_it() {
        let digest = "ab".repeat(32);
        let list = format!(
            "{digest}  other.deb\n{}  amipro-linux_1.0.0_amd64.deb\n{digest} *starred.deb\nshort  bad.deb\n",
            "CD".repeat(32)
        );
        assert_eq!(
            find_checksum(&list, "amipro-linux_1.0.0_amd64.deb"),
            Some("cd".repeat(32))
        );
        assert_eq!(find_checksum(&list, "starred.deb"), Some(digest));
        assert_eq!(find_checksum(&list, "bad.deb"), None);
        assert_eq!(find_checksum(&list, "missing.deb"), None);
    }

    #[test]
    fn a_release_is_read_and_its_assets_picked() {
        let json = release_json("v0.2.0", 5);
        let release = parse_release(json.as_bytes()).unwrap();
        assert_eq!(release.tag, "v0.2.0");
        assert_eq!(release.assets.len(), 2);
        let Outcome::Newer(offer) = judge(&release, &v("0.1.0+g1")).unwrap() else {
            panic!("newer")
        };
        assert_eq!(offer.version, "0.2.0");
        assert_eq!(offer.deb.name, "amipro-linux_0.2.0_amd64.deb");
        assert_eq!(offer.deb.size, 5);
        assert_eq!(offer.sums.name, "SHA256SUMS");
        assert_eq!(offer.notes, "Line one\nLine two");
        assert_eq!(
            judge(&release, &v("0.2.0")).unwrap(),
            Outcome::UpToDate {
                latest: "0.2.0".into()
            }
        );
        assert!(matches!(
            judge(&release, &v("0.3.0")).unwrap(),
            Outcome::UpToDate { .. }
        ));
    }

    #[test]
    fn a_release_without_a_verifiable_package_is_an_error() {
        let mut release = parse_release(release_json("v0.2.0", 5).as_bytes()).unwrap();
        let mut no_sums = release.clone();
        no_sums.assets.retain(|asset| asset.name != SUMS_NAME);
        let error = judge(&no_sums, &v("0.1.0")).unwrap_err();
        assert!(error.contains("cannot be verified"), "{error}");
        release.assets.retain(|asset| !asset.name.ends_with(".deb"));
        assert!(
            judge(&release, &v("0.1.0"))
                .unwrap_err()
                .contains("no amipro-linux_0.2.0_amd64.deb")
        );
        // An asset hosted elsewhere is not used.
        let mut elsewhere = parse_release(release_json("v0.2.0", 5).as_bytes()).unwrap();
        elsewhere.assets[0].url = "https://example.com/x.deb".into();
        assert!(judge(&elsewhere, &v("0.1.0")).is_err());
        // A draft, a bad tag, and unreadable JSON.
        let mut draft = parse_release(release_json("v0.2.0", 5).as_bytes()).unwrap();
        draft.draft = true;
        assert!(judge(&draft, &v("0.1.0")).is_err());
        draft.draft = false;
        draft.tag = "nightly".into();
        assert!(judge(&draft, &v("0.1.0")).is_err());
        assert!(parse_release(b"<html>").is_err());
        assert!(parse_release(b"{}").is_err());
    }

    #[test]
    fn notes_keep_twenty_lines_and_no_control_characters() {
        let body: String = (1..=30).map(|n| format!("line {n}\n")).collect();
        let shown = notes_excerpt(&body);
        assert_eq!(shown.lines().count(), 21);
        assert!(shown.ends_with("line 20\n…"));
        assert_eq!(notes_excerpt("a\u{1b}[31mb\r\nc"), "a[31mb\nc");
    }

    #[test]
    fn check_asks_the_api_and_reports_the_outcome() {
        let fake = Fake::default();
        assert!(check(&fake, &v("0.1.0")).is_err());
        serve_release(&fake, "v0.2.0", b"deb", None);
        assert!(matches!(
            check(&fake, &v("0.1.0")).unwrap(),
            Outcome::Newer(_)
        ));
        assert!(matches!(
            check(&fake, &v("0.2.0+g1")).unwrap(),
            Outcome::UpToDate { .. }
        ));
    }

    fn offer(fake: &Fake, listed: Option<&str>) -> Offer {
        serve_release(fake, "v0.2.0", b"package bytes", listed);
        let Outcome::Newer(offer) = check(fake, &v("0.1.0")).unwrap() else {
            panic!("newer")
        };
        offer
    }

    #[test]
    fn a_verified_download_is_kept() {
        let fake = Fake::default();
        let offer = offer(&fake, None);
        let dir = private_dir().unwrap();
        let path = download_and_verify(&fake, &offer, &dir).unwrap();
        assert_eq!(path, dir.join("amipro-linux_0.2.0_amd64.deb"));
        assert_eq!(std::fs::read(&path).unwrap(), b"package bytes");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_bad_checksum_is_refused_and_the_file_deleted() {
        let fake = Fake::default();
        let offer = offer(&fake, Some(&"0".repeat(64)));
        let dir = private_dir().unwrap();
        let error = download_and_verify(&fake, &offer, &dir).unwrap_err();
        assert!(error.contains("checksum does not match"), "{error}");
        assert!(!dir.join("amipro-linux_0.2.0_amd64.deb").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_package_missing_from_the_list_or_the_wrong_size_is_refused() {
        let fake = Fake::default();
        let mut offer = offer(&fake, None);
        let dir = private_dir().unwrap();
        fake.serve(
            &offer.sums.url,
            format!("{}  other.deb\n", "0".repeat(64)).as_bytes(),
        );
        assert!(
            download_and_verify(&fake, &offer, &dir)
                .unwrap_err()
                .contains("does not list")
        );
        fake.serve(
            &offer.sums.url,
            format!("{}  {}\n", sha256_hex(b"package bytes"), offer.deb.name).as_bytes(),
        );
        offer.deb.size = 3;
        let error = download_and_verify(&fake, &offer, &dir).unwrap_err();
        assert!(error.contains("size"), "{error}");
        assert!(!dir.join(&offer.deb.name).exists());
        offer.deb.size = u64::MAX;
        assert!(
            download_and_verify(&fake, &offer, &dir)
                .unwrap_err()
                .contains("larger")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_download_is_reported() {
        let fake = Fake::default();
        let offer = offer(&fake, None);
        fake.fail(&offer.deb.url, "The connection failed");
        let dir = private_dir().unwrap();
        assert_eq!(
            download_and_verify(&fake, &offer, &dir).unwrap_err(),
            "The connection failed"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_private_directory_is_only_the_users() {
        use std::os::unix::fs::PermissionsExt;
        let one = private_dir().unwrap();
        let two = private_dir().unwrap();
        assert_ne!(one, two);
        assert_eq!(
            std::fs::metadata(&one).unwrap().permissions().mode() & 0o777,
            0o700
        );
        std::fs::remove_dir_all(one).unwrap();
        std::fs::remove_dir_all(two).unwrap();
    }

    #[test]
    fn the_install_command_is_exact_and_quoted() {
        assert_eq!(
            install_command(Path::new(
                "/run/user/1000/amipro-update-1/amipro-linux_0.2.0_amd64.deb"
            )),
            "pkexec apt-get install -y /run/user/1000/amipro-update-1/amipro-linux_0.2.0_amd64.deb"
        );
        assert_eq!(
            install_command(Path::new("/tmp/it's here/x.deb")),
            "pkexec apt-get install -y '/tmp/it'\\''s here/x.deb'"
        );
    }

    #[test]
    fn redirects_may_only_lead_to_github() {
        for good in [
            "https://github.com/nchiker/AmiPro-Linux/releases/download/v1/x",
            "https://objects.githubusercontent.com/a?b=c",
            "https://release-assets.githubusercontent.com/x",
            "https://api.github.com/repos/x",
        ] {
            assert!(allowed_host(good), "{good}");
        }
        for bad in [
            "http://github.com/x",
            "https://example.com/github.com",
            "https://github.com.evil.example/x",
            "https://evilgithubusercontent.com/x",
            "https://github.com@evil.example/x",
            "https://github.com:8443/x",
            "file:///etc/passwd",
            "",
        ] {
            assert!(!allowed_host(bad), "{bad}");
        }
    }

    #[test]
    fn the_start_up_check_is_due_once_a_day() {
        assert!(check_due(0, 1_000_000));
        assert!(!check_due(1_000_000, 1_000_000 + CHECK_INTERVAL_SECS - 1));
        assert!(check_due(1_000_000, 1_000_000 + CHECK_INTERVAL_SECS));
        assert!(check_due(2_000_000, 1_000_000), "a clock that moved back");
    }

    #[test]
    fn a_tool_is_found_only_when_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = private_dir().unwrap();
        std::fs::write(dir.join("tool"), b"#!/bin/sh\n").unwrap();
        assert_eq!(find_in("tool", std::slice::from_ref(&dir)), None);
        std::fs::set_permissions(dir.join("tool"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            find_in("tool", std::slice::from_ref(&dir)),
            Some(dir.join("tool"))
        );
        assert_eq!(find_in("absent", std::slice::from_ref(&dir)), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
