//! Master documents in the app (`UI_SPEC.md` §37; MS1–MS11): a master's
//! files found (MS2), read and laid out off the interface's thread, and the
//! numbers each one continues from, worked out in order (MS3).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use amipro_document::Document;
use amipro_document::contents::Sequence;
use amipro_editing::master::{Check, by_name, checks, file_name, seq_counters};
use amipro_layout::DocumentLayout;
use amipro_layout::master::next_sequence_rebased;

/// A member read from its file and laid out with the page base `base`.
#[derive(Clone, Debug)]
pub struct Laid {
    pub document: Document,
    pub layout: Arc<DocumentLayout>,
    pub base: u32,
    pub modified: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberState {
    /// Found, not read yet (or being read).
    Unread,
    Ready,
    /// Neither its path nor its name in the master's folder exists.
    Missing,
    /// Read but not a document the app opens (MS8), or damaged.
    Unreadable(String),
}

/// One file of a master document and its place in the set.
#[derive(Clone, Debug)]
pub struct Member {
    /// As `[master]` stores it.
    pub stored: String,
    /// Where it was found (MS2).
    pub path: Option<PathBuf>,
    pub state: MemberState,
    pub laid: Option<Arc<Laid>>,
    /// What it continues from (none for the first file).
    pub incoming: Option<Sequence>,
    /// The page numbers its first and last pages show in the set.
    pub pages: Option<(u32, u32)>,
    pub footnotes: u32,
    pub checks: Vec<Check>,
}

impl Member {
    pub fn new(stored: &str, folder: Option<&Path>) -> Self {
        let path = resolve(stored, folder);
        Self {
            stored: stored.to_string(),
            state: if path.is_some() {
                MemberState::Unread
            } else {
                MemberState::Missing
            },
            path,
            laid: None,
            incoming: None,
            pages: None,
            footnotes: 0,
            checks: Vec::new(),
        }
    }

    /// The name the panel shows.
    pub fn name(&self) -> &str {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| file_name(&self.stored))
    }
}

/// A master document's files and their numbers.
#[derive(Clone, Debug, Default)]
pub struct MasterSet {
    pub members: Vec<Member>,
    /// The panel's selected file.
    pub selected: Option<usize>,
    /// The files being read off the interface's thread.
    pub reading: Vec<PathBuf>,
    /// The master's folder the files were looked for in (MS2).
    pub folder: Option<PathBuf>,
}

impl MasterSet {
    /// The set for `files` (the master's `[master]` list), keeping what
    /// is already read for files still listed.
    pub fn refresh(&mut self, files: &[String], folder: Option<&Path>) {
        // A master saved into a folder (or moved) looks for the files it
        // could not find there.
        if self.folder.as_deref() != folder {
            self.folder = folder.map(Path::to_path_buf);
            for member in &mut self.members {
                if member.path.is_none() {
                    *member = Member::new(&member.stored, folder);
                }
            }
        }
        if self.members.len() == files.len()
            && self
                .members
                .iter()
                .zip(files)
                .all(|(member, file)| member.stored == *file)
        {
            return;
        }
        let mut old = std::mem::take(&mut self.members);
        self.members = files
            .iter()
            .map(
                |file| match old.iter().position(|member| member.stored == *file) {
                    Some(index) => old.remove(index),
                    None => Member::new(file, folder),
                },
            )
            .collect();
        if self
            .selected
            .is_some_and(|selected| selected >= self.members.len())
        {
            self.selected = None;
        }
    }

    pub fn total_pages(&self) -> u32 {
        self.members
            .iter()
            .filter_map(|member| member.pages)
            .map(|(_, last)| last)
            .max()
            .unwrap_or(0)
    }

    pub fn total_footnotes(&self) -> u32 {
        self.members.iter().map(|member| member.footnotes).sum()
    }
}

/// Where a stored file is (MS2): its own path when it exists, else the
/// file of the same name, without case, in the master's folder.
pub fn resolve(stored: &str, folder: Option<&Path>) -> Option<PathBuf> {
    let own = PathBuf::from(stored);
    if own.is_absolute() && own.is_file() {
        return Some(own);
    }
    let folder = folder?;
    let joined = folder.join(stored);
    if !stored.contains(['\\', ':']) && joined.is_file() {
        return Some(joined);
    }
    let entries: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    let name = by_name(stored, entries.iter().map(String::as_str))?;
    Some(folder.join(name))
}

/// Reads a member (MS8: SAM or the app's own) and lays it out with its
/// own sequence, off the interface's thread.
pub fn read_member(path: &Path, style_folders: &[PathBuf]) -> Result<Laid, String> {
    let modified = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok();
    let document = read_document(path, style_folders)?;
    let base = document
        .contents()
        .sequence
        .as_ref()
        .map_or(0, |sequence| sequence.page_base);
    let layout = crate::page_layout::with_private_shaper(|shaper| {
        amipro_layout::layout_document(&document, shaper, &mut amipro_layout::ShapeCache::default())
    });
    Ok(Laid {
        document,
        layout: Arc::new(layout),
        base,
        modified,
    })
}

/// A SAM or `.amipro` document read from `path`.
pub fn read_document(path: &Path, style_folders: &[PathBuf]) -> Result<Document, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("sam") => amipro_formats::import_sam_path_with_style_directories(path, style_folders)
            .map(|import| import.document)
            .map_err(|error| error.to_string()),
        Some("amipro") => Document::load_native(path).map_err(|error| error.to_string()),
        _ => Err("not an Ami Pro or AmiPro-Linux document".into()),
    }
}

/// A member's document and layout as the carry needs them: an open tab's,
/// or the one read from its file.
pub struct MemberView<'a> {
    pub document: &'a Document,
    pub layout: &'a DocumentLayout,
    /// The page base it was laid out with.
    pub base: u32,
}

/// Works the set's numbers out in order (MS3), as a master run passes them
/// on (`1660:054d`): each file continues from what the ones before it pass
/// on. A file not read stops the carry, as Ami Pro's run stops at a file
/// it cannot open.
#[derive(Debug, Default)]
pub struct Carrier {
    incoming: Option<Sequence>,
    stopped: bool,
}

impl Carrier {
    /// What the next file continues from (none for the first).
    pub fn incoming(&self) -> Option<&Sequence> {
        self.incoming.as_ref()
    }

    /// Takes the next file: `shown` is its document and layout once it
    /// continues from [`Self::incoming`] (an open tab laid out again with
    /// it), or `None` for a file not read.
    pub fn step(&mut self, member: &mut Member, shown: Option<MemberView<'_>>) {
        member.incoming = self.incoming.clone();
        let shown = match shown {
            Some(shown) if !self.stopped => shown,
            _ => {
                member.pages = None;
                member.footnotes = 0;
                self.stopped = true;
                return;
            }
        };
        let mut document = shown.document.clone();
        if document.contents().sequence != self.incoming {
            let mut contents = document.contents().clone();
            contents.sequence = self.incoming.clone();
            document.set_contents(contents);
        }
        let next = next_sequence_rebased(
            &document,
            shown.layout,
            shown.base,
            &seq_counters(&document),
        );
        let base = self
            .incoming
            .as_ref()
            .map_or(0, |sequence| sequence.page_base);
        let first = shown.layout.pages.first().map_or(base + 1, |page| {
            page.shown - shown.base.min(page.shown) + base
        });
        member.pages = Some((first, next.page_base));
        member.footnotes = amipro_layout::master::footnote_count(&document);
        member.checks = checks(&document);
        self.incoming = Some(next);
    }
}

/// A master in `folder` that lists `member` (MS4): the first SAM or
/// `.amipro` document there whose `[master]` names it, by path or by name.
pub fn master_listing(member: &Path, style_folders: &[PathBuf]) -> Option<PathBuf> {
    let folder = member.parent()?;
    let name = member.file_name()?.to_str()?;
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path != member)
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("sam")
                        || extension.eq_ignore_ascii_case("amipro")
                })
        })
        .collect();
    candidates.sort();
    candidates.into_iter().find(|path| {
        // A cheap look first: only files with a [master] section are read.
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let text = String::from_utf8_lossy(&bytes);
        if !text.contains("[master]") && !text.contains("\nmaster\t") {
            return false;
        }
        read_document(path, style_folders).is_ok_and(|document| {
            document.contents().master.as_ref().is_some_and(|master| {
                master
                    .files
                    .iter()
                    .any(|stored| file_name(stored).eq_ignore_ascii_case(name))
            })
        })
    })
}
