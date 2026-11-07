//! Merge data files on the app's side (`docs/Design/UI_SPEC.md` §38,
//! MG2): finding the file `[recfile]` names, and reading any kind of data
//! file off the interface's thread into a [`DataTable`].

use std::path::{Path, PathBuf};

use amipro_editing::merge::data::{
    DataError, DataTable, Delimiters, DescriptionKind, read_ami_data, read_comma, read_dbase,
    read_description, read_dif, read_fixed, read_separated, read_table_data, text_of,
};

/// What kind of data file it is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKind {
    /// Ami Pro's delimited data document (kind 0): editable.
    AmiPro,
    /// A document holding only a table (kind 4).
    Table,
    /// A DIF, fixed-width, or comma file through a description file.
    Described(DescriptionKind),
    Csv,
    Tsv,
    Dbase,
    Xlsx,
    Ods,
}

impl DataKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::AmiPro => "Ami Pro data file",
            Self::Table => "table",
            Self::Described(DescriptionKind::Dif) => "DIF file",
            Self::Described(DescriptionKind::Fixed) => "fixed-width text",
            Self::Described(DescriptionKind::Comma) => "comma-delimited text",
            Self::Described(DescriptionKind::Dbase) | Self::Dbase => "dBASE file",
            Self::Csv => "CSV file",
            Self::Tsv => "TSV file",
            Self::Xlsx => "Excel workbook",
            Self::Ods => "OpenDocument spreadsheet",
        }
    }

    /// Whether the record editor may change it (MG3: Ami Pro's own data
    /// documents only; external files are read only, as in Ami Pro).
    /// Ami Pro's own data files, edited as a table (MR1).
    pub fn editable(self) -> bool {
        matches!(self, Self::AmiPro | Self::Table)
    }
}

/// A data file read.
#[derive(Clone, Debug)]
pub struct MergeData {
    pub path: PathBuf,
    pub kind: DataKind,
    pub table: DataTable,
    /// An Ami Pro data document's delimiters, which the record editor
    /// writes back (ME.8).
    #[allow(dead_code)]
    pub delimiters: Option<Delimiters>,
}

impl MergeData {
    /// The records as a letter merges them: a table data file's values go
    /// through Ami Pro's own number rule (TD.9, `merge_number_text`); the
    /// other kinds merge as read (AMT3: a delimited file's text as typed).
    pub fn for_merge(&self) -> std::borrow::Cow<'_, DataTable> {
        if self.kind == DataKind::Table {
            std::borrow::Cow::Owned(self.table.with_merged_numbers())
        } else {
            std::borrow::Cow::Borrowed(&self.table)
        }
    }
}

/// The file a stored path names (MS2's rule): the path itself, else the
/// file of that name, without case, in the document's folder.
pub fn resolve(stored: &str, folder: Option<&Path>) -> Option<PathBuf> {
    let stored = stored.trim();
    if stored.is_empty() || stored.starts_with('*') {
        return None;
    }
    let direct = PathBuf::from(stored);
    if direct.is_absolute() && direct.is_file() {
        return Some(direct);
    }
    let name = stored.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
    let folder = folder?;
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_name()
                    .is_some_and(|file| file.to_string_lossy().to_ascii_lowercase() == name)
        })
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

/// Reads the data file at `path`, with its description file when it has
/// one (Ami Pro's `[recfile]` second string).
pub fn read_data(
    path: &Path,
    description: Option<&Path>,
    style_folders: &[PathBuf],
) -> Result<MergeData, String> {
    let message = |error: DataError| error.message();
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let made = |kind: DataKind, table: DataTable| MergeData {
        path: path.to_path_buf(),
        kind,
        table,
        delimiters: None,
    };
    if let Some(description) = description {
        let document = crate::master::read_document(description, style_folders)?;
        let description = read_description(&document).map_err(message)?;
        let names = Some(description.names.as_slice());
        return match description.kind {
            DescriptionKind::Dbase => read_dbase(&bytes)
                .map(|table| made(DataKind::Dbase, table))
                .map_err(message),
            DescriptionKind::Dif => {
                let text = text_of(&bytes).map_err(message)?;
                read_dif(&text, names)
                    .map(|table| made(DataKind::Described(DescriptionKind::Dif), table))
                    .map_err(message)
            }
            DescriptionKind::Fixed => {
                let text = text_of(&bytes).map_err(message)?;
                read_fixed(&text, &description)
                    .map(|table| made(DataKind::Described(DescriptionKind::Fixed), table))
                    .map_err(message)
            }
            DescriptionKind::Comma => {
                let text = text_of(&bytes).map_err(message)?;
                read_comma(&text, names)
                    .map(|table| made(DataKind::Described(DescriptionKind::Comma), table))
                    .map_err(message)
            }
        };
    }
    match extension(path).as_deref() {
        Some("sam" | "amipro") => {
            let document = crate::master::read_document(path, style_folders)?;
            match read_ami_data(&document) {
                Ok((table, delimiters)) => Ok(MergeData {
                    delimiters: Some(delimiters),
                    ..made(DataKind::AmiPro, table)
                }),
                Err(first) => read_table_data(&document)
                    .map(|table| made(DataKind::Table, table))
                    .map_err(|_| message(first)),
            }
        }
        Some("dbf") => read_dbase(&bytes)
            .map(|table| made(DataKind::Dbase, table))
            .map_err(message),
        Some("xlsx") => amipro_exchange::sheets::read_xlsx(&bytes, &crate::import::inflate)
            .map(|table| made(DataKind::Xlsx, table))
            .map_err(message),
        Some("ods") => amipro_exchange::sheets::read_ods(&bytes, &crate::import::inflate)
            .map(|table| made(DataKind::Ods, table))
            .map_err(message),
        Some("tsv" | "tab") => {
            let text = text_of(&bytes).map_err(message)?;
            read_separated(&text, '\t')
                .map(|table| made(DataKind::Tsv, table))
                .map_err(message)
        }
        Some("dif") => {
            let text = text_of(&bytes).map_err(message)?;
            read_dif(&text, None)
                .map(|table| made(DataKind::Described(DescriptionKind::Dif), table))
                .map_err(message)
        }
        _ => {
            // CSV, or any text: a tab in the first line makes it TSV.
            let text = text_of(&bytes).map_err(message)?;
            let first = text.lines().next().unwrap_or_default();
            let (separator, kind) = if first.contains('\t') && !first.contains(',') {
                ('\t', DataKind::Tsv)
            } else {
                (',', DataKind::Csv)
            };
            read_separated(&text, separator)
                .map(|table| made(kind, table))
                .map_err(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/merge-tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn data_files_of_each_kind_are_read_and_found_by_name() {
        let folder = scratch("kinds");
        std::fs::write(folder.join("Clients.CSV"), "Name,City\nAnn,Boston\n").unwrap();
        std::fs::write(folder.join("list.tsv"), "Name\tCity\nBob\tDallas\n").unwrap();
        std::fs::write(
            folder.join("DATA.SAM"),
            "[ver]\r\n\t4\r\n[edoc]\r\n~|\r\n\r\nNAME~CITY|\r\n\r\nCy~Troy|\r\n\r\n>\r\n\r\n",
        )
        .unwrap();
        std::fs::write(
            folder.join("desc.sam"),
            "[ver]\r\n\t4\r\n[edoc]\r\nCOMMA\r\n\r\nName\r\n\r\nCity\r\n\r\n>\r\n\r\n",
        )
        .unwrap();
        std::fs::write(folder.join("plain.txt"), "\"Di\", Reno\n").unwrap();
        let csv = read_data(
            &resolve("c:\\docs\\clients.csv", Some(&folder)).unwrap(),
            None,
            &[],
        )
        .unwrap();
        assert_eq!(csv.kind, DataKind::Csv);
        assert_eq!(csv.table.value(0, "city"), Some("Boston"));
        let tsv = read_data(&folder.join("list.tsv"), None, &[]).unwrap();
        assert_eq!(tsv.table.value(0, "NAME"), Some("Bob"));
        let ami = read_data(&folder.join("DATA.SAM"), None, &[]).unwrap();
        assert_eq!(ami.kind, DataKind::AmiPro);
        assert!(ami.kind.editable() && !csv.kind.editable());
        assert_eq!(ami.table.value(0, "CITY"), Some("Troy"));
        let described = read_data(
            &folder.join("plain.txt"),
            Some(&folder.join("desc.sam")),
            &[],
        )
        .unwrap();
        assert_eq!(described.table.value(0, "City"), Some("Reno"));
        assert!(resolve("*dBase", Some(&folder)).is_none());
        assert!(resolve("c:\\nothing.sam", Some(&folder)).is_none());
    }
}
