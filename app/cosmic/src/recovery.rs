//! Crash recovery's store (P6.3, `docs/Design/UI_SPEC.md` §44): hidden
//! copies of open documents with unsaved changes, kept apart from the
//! user's own files, which they never touch.
//!
//! Layout, under `$XDG_DATA_HOME/amipro-linux/recovery/`:
//!
//! ```text
//! <pid>-<started>/        one folder per running instance (its session)
//!     lock                the instance's pid and process start time
//!     <id>.amipro-recovery  the document, in the lossless native format
//!     <id>.meta           title, original path, kind, time, size
//! ```
//!
//! A session whose process is gone (the pid is not running, or runs with
//! another start time) is stale, and its documents are offered at the next
//! start; a live session's are never offered, so two instances do not take
//! each other's files. Files are written whole beside their final name and
//! renamed into place. Nothing here knows the interface: the clock and the
//! process probe are a trait, so the logic is tested with fakes.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use amipro_document::Document;

/// The extension of a recovery copy.
pub const EXTENSION: &str = "amipro-recovery";
/// The largest copy written (the native reader's own limit).
pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;
/// The most one instance keeps in its session folder.
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// Stale sessions older than this are removed at start.
pub const MAX_AGE_DAYS: u64 = 30;

/// The clock and the processes, which tests replace.
pub trait Probe: Send + Sync {
    /// Unix seconds.
    fn now(&self) -> u64;
    /// This process's id.
    fn pid(&self) -> u32;
    /// When process `pid` started (an opaque number that is the same for
    /// the process's whole life), or `None` when it is not running.
    fn process_start(&self, pid: u32) -> Option<u64>;
}

/// The real clock and `/proc`.
pub struct SystemProbe;

impl Probe for SystemProbe {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |time| time.as_secs())
    }

    fn pid(&self) -> u32 {
        std::process::id()
    }

    fn process_start(&self, pid: u32) -> Option<u64> {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // The command name may hold spaces and parentheses: the fields
        // after the last ")" start at field 3; start time is field 22.
        let rest = &stat[stat.rfind(')')? + 1..];
        rest.split_whitespace().nth(19)?.parse().ok()
    }
}

/// Where the recovery folders live: `$XDG_DATA_HOME/amipro-linux/recovery`.
pub fn default_root() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|base| base.join("amipro-linux").join("recovery"))
}

/// Where the document came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Untitled,
    Sam,
    Native,
    /// Another program's file, or a data file.
    Legacy,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Self::Untitled => "untitled",
            Self::Sam => "sam",
            Self::Native => "native",
            Self::Legacy => "legacy",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "untitled" => Self::Untitled,
            "sam" => Self::Sam,
            "native" => Self::Native,
            "legacy" => Self::Legacy,
            _ => return None,
        })
    }
}

/// The sidecar of a recovery copy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meta {
    pub title: String,
    /// The file the document came from or was saved to, if any.
    pub path: Option<PathBuf>,
    pub kind: Kind,
    /// When the copy was written (Unix seconds).
    pub time: u64,
    /// The copy's size in bytes.
    pub bytes: u64,
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            other => out.push(other),
        }
    }
    out
}

fn unescape(text: &str) -> String {
    text.replace("%0A", "\n")
        .replace("%0D", "\r")
        .replace("%25", "%")
}

impl Meta {
    fn encode(&self) -> String {
        let mut out = format!(
            "amipro-recovery 1\ntitle={}\nkind={}\ntime={}\nbytes={}\n",
            escape(&self.title),
            self.kind.word(),
            self.time,
            self.bytes
        );
        if let Some(path) = &self.path {
            out.push_str(&format!("path={}\n", escape(&path.to_string_lossy())));
        }
        out
    }

    fn decode(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        if lines.next() != Some("amipro-recovery 1") {
            return Err("not a recovery note".into());
        }
        let (mut title, mut kind, mut time, mut bytes, mut path) = (None, None, None, None, None);
        for line in lines {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "title" => title = Some(unescape(value)),
                "kind" => kind = Kind::from_word(value),
                "time" => time = value.parse().ok(),
                "bytes" => bytes = value.parse().ok(),
                "path" => path = Some(PathBuf::from(unescape(value))),
                _ => {}
            }
        }
        match (title, kind, time, bytes) {
            (Some(title), Some(kind), Some(time), Some(bytes)) => Ok(Self {
                title,
                path,
                kind,
                time,
                bytes,
            }),
            _ => Err("the recovery note is incomplete".into()),
        }
    }
}

/// One recovery copy found on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    /// The session folder's name.
    pub session: String,
    /// The editing session's document id inside that instance.
    pub id: u64,
    pub meta: Meta,
    pub data: PathBuf,
}

impl Entry {
    /// A name for the copy that is unique among those found.
    pub fn key(&self) -> String {
        format!("{}/{}", self.session, self.id)
    }

    fn meta_path(&self) -> PathBuf {
        self.data.with_extension("meta")
    }
}

/// What a scan found: the copies offered, and a line for each that could
/// not be read (those stay on disk).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Scan {
    pub entries: Vec<Entry>,
    pub problems: Vec<String>,
}

/// The store of one running instance.
#[derive(Clone)]
pub struct Store {
    root: PathBuf,
    probe: Arc<dyn Probe>,
    own: String,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

impl Store {
    /// A store under `root` for the process `probe` describes.
    pub fn new(root: PathBuf, probe: Arc<dyn Probe>) -> Self {
        let own = format!("{}-{}", probe.pid(), probe.now());
        Self { root, probe, own }
    }

    /// This instance's session folder (it exists once something was written).
    pub fn own_dir(&self) -> PathBuf {
        self.root.join(&self.own)
    }

    /// Makes this instance's folder and its lock, once. The folder is built
    /// under a hidden name and renamed, so that no scan sees it without its
    /// lock.
    fn ensure_session(&self) -> std::io::Result<PathBuf> {
        use std::os::unix::fs::DirBuilderExt;
        let dir = self.own_dir();
        if dir.is_dir() {
            return Ok(dir);
        }
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(&self.root)?;
        let building = self.root.join(format!(".{}.new", self.own));
        let _ = fs::remove_dir_all(&building);
        builder.recursive(false).create(&building)?;
        let pid = self.probe.pid();
        let lock = format!(
            "pid={pid}\nstart={}\ncreated={}\n",
            self.probe.process_start(pid).unwrap_or(0),
            self.probe.now()
        );
        let built = write_atomic(&building.join("lock"), lock.as_bytes())
            .and_then(|()| fs::rename(&building, &dir));
        if let Err(error) = built {
            let _ = fs::remove_dir_all(&building);
            return Err(error);
        }
        Ok(dir)
    }

    /// Writes document `id`'s copy (`text` is its native encoding) and its
    /// note; returns the copy's size. Refused when it is over the caps.
    pub fn write(
        &self,
        id: u64,
        title: &str,
        path: Option<&Path>,
        kind: Kind,
        text: &str,
    ) -> Result<u64, String> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "the document is over {} MB",
                MAX_DOCUMENT_BYTES / (1024 * 1024)
            ));
        }
        let dir = self.ensure_session().map_err(|error| error.to_string())?;
        let data = dir.join(format!("{id}.{EXTENSION}"));
        let others = folder_bytes(&dir).saturating_sub(fs::metadata(&data).map_or(0, |m| m.len()));
        if others + text.len() as u64 > MAX_TOTAL_BYTES {
            return Err("the recovery copies already take the space allowed".into());
        }
        let meta = Meta {
            title: title.to_owned(),
            path: path.map(Path::to_path_buf),
            kind,
            time: self.probe.now(),
            bytes: text.len() as u64,
        };
        write_atomic(&data, text.as_bytes()).map_err(|error| error.to_string())?;
        write_atomic(&data.with_extension("meta"), meta.encode().as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(meta.bytes)
    }

    /// Removes document `id`'s copy from this instance's folder.
    pub fn remove(&self, id: u64) {
        let data = self.own_dir().join(format!("{id}.{EXTENSION}"));
        let _ = fs::remove_file(data.with_extension("meta"));
        let _ = fs::remove_file(data);
    }

    /// Removes a copy found by a scan.
    pub fn remove_entry(&self, entry: &Entry) {
        let _ = fs::remove_file(entry.meta_path());
        let _ = fs::remove_file(&entry.data);
        let _ = fs::remove_dir(entry.data.parent().unwrap_or(&self.root));
    }

    /// A clean exit: this instance's folder goes, with whatever it holds.
    pub fn finish(&self) {
        let _ = fs::remove_dir_all(self.own_dir());
    }

    /// Whether the session folder `dir` belongs to a running instance.
    fn session_alive(&self, dir: &Path) -> bool {
        let Ok(lock) = fs::read_to_string(dir.join("lock")) else {
            return false;
        };
        let (mut pid, mut start) = (None, None);
        for line in lock.lines() {
            match line.split_once('=') {
                Some(("pid", value)) => pid = value.parse::<u32>().ok(),
                Some(("start", value)) => start = value.parse::<u64>().ok(),
                _ => {}
            }
        }
        match (pid, start) {
            (Some(pid), Some(start)) => self.probe.process_start(pid) == Some(start),
            _ => false,
        }
    }

    /// The session folders of other instances that are gone.
    fn stale_sessions(&self) -> Vec<(String, PathBuf)> {
        let Ok(read) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut out: Vec<(String, PathBuf)> = read
            .flatten()
            .filter_map(|item| {
                let name = item.file_name().into_string().ok()?;
                let path = item.path();
                (!is_hidden(&name)
                    && name != self.own
                    && path.is_dir()
                    && !self.session_alive(&path))
                .then_some((name, path))
            })
            .collect();
        out.sort();
        out
    }

    /// The copies of gone instances, newest first. A folder with nothing
    /// in it is removed; a copy that cannot be read is reported and kept.
    pub fn scan(&self) -> Scan {
        let mut scan = Scan::default();
        for (session, dir) in self.stale_sessions() {
            let Ok(read) = fs::read_dir(&dir) else {
                continue;
            };
            let mut ids = BTreeSet::new();
            for item in read.flatten() {
                let name = item.file_name().to_string_lossy().into_owned();
                if let Some(stem) = name.strip_suffix(&format!(".{EXTENSION}"))
                    && let Ok(id) = stem.parse::<u64>()
                {
                    ids.insert(id);
                }
            }
            if ids.is_empty() {
                // Only the lock, notes, or leftovers: nothing to offer.
                let only_scraps = fs::read_dir(&dir).is_ok_and(|read| {
                    read.flatten().all(|item| {
                        let name = item.file_name().to_string_lossy().into_owned();
                        name == "lock" || name.ends_with(".tmp") || name.ends_with(".meta")
                    })
                });
                if only_scraps {
                    let _ = fs::remove_dir_all(&dir);
                }
                continue;
            }
            for id in ids {
                let data = dir.join(format!("{id}.{EXTENSION}"));
                let meta = fs::read_to_string(data.with_extension("meta"))
                    .map_err(|error| error.to_string())
                    .and_then(|text| Meta::decode(&text));
                match meta {
                    Ok(meta) => scan.entries.push(Entry {
                        session: session.clone(),
                        id,
                        meta,
                        data,
                    }),
                    Err(error) => scan.problems.push(format!(
                        "A recovery copy in {session} was skipped ({error}); it is kept in {}",
                        dir.display()
                    )),
                }
            }
        }
        scan.entries.sort_by(|a, b| {
            b.meta
                .time
                .cmp(&a.meta.time)
                .then_with(|| a.key().cmp(&b.key()))
        });
        scan
    }

    /// Removes the folders of gone instances that were last written more
    /// than [`MAX_AGE_DAYS`] ago; returns how many went.
    pub fn cleanup(&self) -> usize {
        let limit = self.probe.now().saturating_sub(MAX_AGE_DAYS * 86_400);
        let mut removed = 0;
        for (_, dir) in self.stale_sessions() {
            if newest_time(&dir) < limit && fs::remove_dir_all(&dir).is_ok() {
                removed += 1;
            }
        }
        removed
    }
}

/// The newest of a session folder's recorded times (its lock's, its notes'),
/// else the folder's own modification time.
fn newest_time(dir: &Path) -> u64 {
    let mut newest = 0;
    if let Ok(lock) = fs::read_to_string(dir.join("lock")) {
        for line in lock.lines() {
            if let Some(("created", value)) = line.split_once('=') {
                newest = newest.max(value.parse().unwrap_or(0));
            }
        }
    }
    if let Ok(read) = fs::read_dir(dir) {
        for item in read.flatten() {
            if item.path().extension().is_some_and(|ext| ext == "meta")
                && let Ok(text) = fs::read_to_string(item.path())
                && let Ok(meta) = Meta::decode(&text)
            {
                newest = newest.max(meta.time);
            }
        }
    }
    if newest == 0 {
        newest = fs::metadata(dir)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |time| time.as_secs());
    }
    newest
}

fn folder_bytes(dir: &Path) -> u64 {
    fs::read_dir(dir).map_or(0, |read| {
        read.flatten()
            .filter_map(|item| item.metadata().ok())
            .map(|meta| meta.len())
            .sum()
    })
}

/// Reads a copy back. An unreadable one is an error and stays on disk.
pub fn read(entry: &Entry) -> Result<Document, String> {
    let length = fs::metadata(&entry.data)
        .map_err(|error| error.to_string())?
        .len();
    if length > MAX_DOCUMENT_BYTES as u64 {
        return Err("the recovery copy is too large".into());
    }
    let text = fs::read_to_string(&entry.data).map_err(|error| error.to_string())?;
    Document::decode_native(&text).map_err(|error| error.to_string())
}

#[cfg(test)]
pub mod fake {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::Probe;

    /// A clock and process table the test sets. Siblings share both, as
    /// instances on one machine do.
    pub struct Fake {
        pub pid: u32,
        pub now: Arc<Mutex<u64>>,
        /// pid -> start time of the processes that are running.
        pub running: Arc<Mutex<HashMap<u32, u64>>>,
    }

    impl Fake {
        pub fn new(pid: u32, now: u64) -> Self {
            let fake = Self {
                pid,
                now: Arc::new(Mutex::new(now)),
                running: Arc::new(Mutex::new(HashMap::new())),
            };
            fake.running.lock().unwrap().insert(pid, now);
            fake
        }

        /// Another process on the same machine, started now.
        pub fn sibling(&self, pid: u32) -> Self {
            let now = *self.now.lock().unwrap();
            self.running.lock().unwrap().insert(pid, now);
            Self {
                pid,
                now: self.now.clone(),
                running: self.running.clone(),
            }
        }

        pub fn advance(&self, seconds: u64) {
            *self.now.lock().unwrap() += seconds;
        }

        pub fn stop(&self, pid: u32) {
            self.running.lock().unwrap().remove(&pid);
        }
    }

    impl Probe for Fake {
        fn now(&self) -> u64 {
            *self.now.lock().unwrap()
        }

        fn pid(&self) -> u32 {
            self.pid
        }

        fn process_start(&self, pid: u32) -> Option<u64> {
            self.running.lock().unwrap().get(&pid).copied()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::Fake;
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/recovery-tests")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn store(root: &Path, pid: u32, now: u64) -> (Store, Arc<Fake>) {
        let fake = Arc::new(Fake::new(pid, now));
        (Store::new(root.to_path_buf(), fake.clone()), fake)
    }

    /// A second instance, sharing the first one's process table.
    fn peer(root: &Path, fake: &Arc<Fake>, pid: u32) -> Store {
        Store::new(root.to_path_buf(), Arc::new(fake.sibling(pid)))
    }

    #[test]
    fn a_note_survives_awkward_text() {
        let meta = Meta {
            title: "100% done\nline two".into(),
            path: Some(PathBuf::from("/tmp/a b/%0A.sam")),
            kind: Kind::Sam,
            time: 42,
            bytes: 7,
        };
        assert_eq!(Meta::decode(&meta.encode()).unwrap(), meta);
        assert!(Meta::decode("garbage").is_err());
        assert!(Meta::decode("amipro-recovery 1\ntitle=x\n").is_err());
    }

    #[test]
    fn a_running_instance_is_not_offered_but_a_gone_one_is() {
        let root = scratch("live-and-gone");
        let (first, fake) = store(&root, 100, 1_000);
        let second = peer(&root, &fake, 200);
        first
            .write(1, "One", None, Kind::Untitled, "text one")
            .unwrap();
        second
            .write(
                7,
                "Two",
                Some(Path::new("/home/x/two.sam")),
                Kind::Sam,
                "text two",
            )
            .unwrap();
        // Both run: neither offers the other's.
        assert!(first.scan().entries.is_empty());
        assert!(second.scan().entries.is_empty());
        // The first one is killed.
        fake.stop(100);
        let found = second.scan();
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.entries[0].meta.title, "One");
        assert_eq!(found.entries[0].id, 1);
        assert!(found.problems.is_empty());
        // A new process with the dead one's pid is not the same process.
        fake.running.lock().unwrap().insert(100, 999_999);
        assert_eq!(second.scan().entries.len(), 1);
    }

    #[test]
    fn writes_are_whole_files_and_removal_leaves_nothing() {
        let root = scratch("write-remove");
        let (store, fake) = store(&root, 100, 1_000);
        store.write(3, "T", None, Kind::Untitled, "abc").unwrap();
        store.write(3, "T", None, Kind::Untitled, "abcdef").unwrap();
        let dir = store.own_dir();
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|item| item.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().all(|name| !name.ends_with(".tmp")),
            "{names:?}"
        );
        assert_eq!(
            fs::read_to_string(dir.join("3.amipro-recovery")).unwrap(),
            "abcdef"
        );
        store.remove(3);
        assert!(!dir.join("3.amipro-recovery").exists());
        assert!(!dir.join("3.meta").exists());
        store.finish();
        assert!(!dir.exists());
        // Nothing was left for the next start.
        let next = peer(&root, &fake, 300);
        assert!(next.scan().entries.is_empty());
    }

    #[test]
    fn a_bad_note_is_reported_and_kept() {
        let root = scratch("bad-note");
        let (first, fake) = store(&root, 100, 1_000);
        first.write(1, "Good", None, Kind::Untitled, "a").unwrap();
        first.write(2, "Bad", None, Kind::Untitled, "b").unwrap();
        fs::write(first.own_dir().join("2.meta"), "nonsense").unwrap();
        fake.stop(100);
        let next = peer(&root, &fake, 300);
        let found = next.scan();
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.problems.len(), 1);
        assert!(found.problems[0].contains("kept"));
        assert!(first.own_dir().join("2.amipro-recovery").exists());
    }

    #[test]
    fn copies_over_the_caps_are_refused() {
        let root = scratch("caps");
        let (store, _fake) = store(&root, 100, 1_000);
        let big = "x".repeat(MAX_DOCUMENT_BYTES + 1);
        assert!(store.write(1, "Big", None, Kind::Untitled, &big).is_err());
        assert!(!store.own_dir().join("1.amipro-recovery").exists());
    }

    #[test]
    fn sessions_older_than_thirty_days_are_removed_at_start() {
        let root = scratch("cleanup");
        let (old, fake) = store(&root, 100, 1_000);
        old.write(1, "Old", None, Kind::Untitled, "a").unwrap();
        fake.stop(100);
        fake.advance(10 * 86_400);
        let recent = peer(&root, &fake, 200);
        recent
            .write(1, "Recent", None, Kind::Untitled, "b")
            .unwrap();
        fake.stop(200);
        fake.advance(25 * 86_400); // old: 35 days, recent: 25
        let now = peer(&root, &fake, 300);
        assert_eq!(now.cleanup(), 1);
        let found = now.scan();
        assert_eq!(found.entries.len(), 1);
        assert_eq!(found.entries[0].meta.title, "Recent");
        assert!(!old.own_dir().exists());
    }

    #[test]
    fn a_copy_reads_back_and_a_damaged_one_is_an_error() {
        let root = scratch("read-back");
        let (first, fake) = store(&root, 100, 1_000);
        let document = Document::default();
        let text = document.encode_native().unwrap();
        first.write(1, "Doc", None, Kind::Untitled, &text).unwrap();
        first
            .write(2, "Broken", None, Kind::Untitled, "not native")
            .unwrap();
        fake.stop(100);
        let next = peer(&root, &fake, 300);
        let found = next.scan();
        let good = found.entries.iter().find(|e| e.id == 1).unwrap();
        assert_eq!(read(good).unwrap().plain_text(), document.plain_text());
        let broken = found.entries.iter().find(|e| e.id == 2).unwrap();
        assert!(read(broken).is_err());
        assert!(broken.data.exists());
    }

    #[test]
    fn an_emptied_session_folder_is_tidied() {
        let root = scratch("tidy");
        let (first, fake) = store(&root, 100, 1_000);
        first.write(1, "Doc", None, Kind::Untitled, "a").unwrap();
        first.remove(1);
        fake.stop(100);
        let next = peer(&root, &fake, 300);
        assert!(next.scan().entries.is_empty());
        assert!(!first.own_dir().exists());
    }
}
