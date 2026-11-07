//! Spreadsheets as merge data files (the merge run, decision MG2): the
//! first sheet of an XLSX or ODS file, its first row the field names and
//! each later row with any value a record. Read with the crate's own zip
//! and XML readers; packages are inflated by the caller.

use amipro_editing::merge::data::{DataError, DataTable};

use crate::xml_tree::{Content, Element, Tree};
use crate::zip::{Archive, Inflate};

/// The most columns and rows read (a sheet's repeated empty rows and
/// columns are never expanded past its last value).
pub const MAX_COLUMNS: usize = 1024;
pub const MAX_ROWS: usize = 1_048_576;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";

fn error(text: impl std::fmt::Display) -> DataError {
    DataError::Other(format!("The spreadsheet cannot be read: {text}."))
}

fn part(archive: &Archive, name: &str, inflate: Inflate) -> Result<Tree, DataError> {
    let bytes = archive
        .read(name, inflate)
        .map_err(|e| error(format!("{e:?}")))?;
    Tree::parse(&bytes).map_err(error)
}

/// The rows as records: the first the names, trailing empty values and
/// rows without values dropped.
fn table(mut rows: Vec<Vec<String>>) -> Result<DataTable, DataError> {
    for row in &mut rows {
        while row.last().is_some_and(|value| value.is_empty()) {
            row.pop();
        }
    }
    let first = rows
        .iter()
        .position(|row| !row.is_empty())
        .ok_or(DataError::NoFieldNames)?;
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .skip(first)
        .enumerate()
        .filter(|(index, row)| *index == 0 || !row.is_empty())
        .map(|(_, row)| row)
        .collect();
    let mut table = DataTable {
        names: rows[0].clone(),
        records: rows[1..].to_vec(),
    };
    if table.names.iter().any(|name| name.trim().is_empty()) {
        return Err(DataError::EmptyFieldName);
    }
    let width = table.names.len();
    for record in &mut table.records {
        record.resize(width.max(record.len()), String::new());
    }
    Ok(table)
}

/// A number as the sheet stores it, shown as its shortest text.
fn number_text(value: &str) -> String {
    match value.trim().parse::<f64>() {
        Ok(number) if number.is_finite() => format!("{number}"),
        _ => value.trim().to_string(),
    }
}

/// A day count from 1899-12-30 (Excel's 1900 system) as M/D/YYYY.
fn serial_date(serial: f64) -> Option<String> {
    if !(1.0..2_958_466.0).contains(&serial) {
        return None;
    }
    // Days since 1970-01-01, then the civil date.
    let days = serial.floor() as i64 - 25_569;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let of_era = z - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let m = (5 * of_year + 2) / 153;
    let day = of_year - (153 * m + 2) / 5 + 1;
    let month = if m < 10 { m + 3 } else { m - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    Some(format!("{month}/{day}/{year}"))
}

/// A cell reference's column (`AB12` is 27), from 0.
fn column_of(reference: &str) -> Option<usize> {
    let letters: String = reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if letters.is_empty() {
        return None;
    }
    letters
        .chars()
        .try_fold(0usize, |column, letter| {
            column
                .checked_mul(26)?
                .checked_add(usize::from(letter.to_ascii_uppercase() as u8 - b'A') + 1)
        })
        .map(|column| column - 1)
}

/// Whether a number format shows dates: the built-in 14–22 and 45–47, or
/// a code with day, month, or year letters outside quotes and brackets.
fn is_date_format(id: u32, code: Option<&str>) -> bool {
    if matches!(id, 14..=22 | 45..=47) {
        return true;
    }
    let Some(code) = code else {
        return false;
    };
    let mut quoted = false;
    let mut bracket = false;
    for character in code.chars() {
        match character {
            '"' => quoted = !quoted,
            '[' if !quoted => bracket = true,
            ']' if !quoted => bracket = false,
            'd' | 'D' | 'm' | 'M' | 'y' | 'Y' if !quoted && !bracket => return true,
            _ => {}
        }
    }
    false
}

/// The first sheet of an XLSX workbook.
pub fn read_xlsx(bytes: &[u8], inflate: Inflate) -> Result<DataTable, DataError> {
    let archive = Archive::open(bytes).map_err(|e| error(format!("{e:?}")))?;
    let workbook = part(&archive, "xl/workbook.xml", inflate)?;
    let first = workbook
        .root()
        .child(MAIN, "sheets")
        .and_then(|sheets| sheets.child(MAIN, "sheet"))
        .ok_or_else(|| error("no sheet"))?;
    let id = first
        .attribute(RELATIONSHIPS, "id")
        .ok_or_else(|| error("no sheet relationship"))?;
    let relationships = part(&archive, "xl/_rels/workbook.xml.rels", inflate)?;
    let target = relationships
        .root()
        .children_named(PACKAGE_RELATIONSHIPS, "Relationship")
        .find(|relationship| relationship.attribute("", "Id") == Some(id))
        .and_then(|relationship| relationship.attribute("", "Target"))
        .ok_or_else(|| error("the sheet is not in the package"))?;
    let sheet_name = match target.strip_prefix('/') {
        Some(absolute) => absolute.to_string(),
        None => format!("xl/{target}"),
    };
    let shared: Vec<String> = if archive.contains("xl/sharedStrings.xml") {
        let strings = part(&archive, "xl/sharedStrings.xml", inflate)?;
        strings
            .root()
            .children_named(MAIN, "si")
            .map(|item| string_item(&item))
            .collect()
    } else {
        Vec::new()
    };
    // Which cell styles show dates.
    let mut date_styles = Vec::new();
    if archive.contains("xl/styles.xml") {
        let styles = part(&archive, "xl/styles.xml", inflate)?;
        let codes: Vec<(u32, String)> = styles
            .root()
            .child(MAIN, "numFmts")
            .map(|formats| {
                formats
                    .children_named(MAIN, "numFmt")
                    .filter_map(|format| {
                        Some((
                            format.attribute("", "numFmtId")?.parse().ok()?,
                            format.attribute("", "formatCode")?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(cells) = styles.root().child(MAIN, "cellXfs") {
            for format in cells.children_named(MAIN, "xf") {
                let id: u32 = format
                    .attribute("", "numFmtId")
                    .and_then(|id| id.parse().ok())
                    .unwrap_or(0);
                let code = codes
                    .iter()
                    .find(|(own, _)| *own == id)
                    .map(|(_, code)| code.as_str());
                date_styles.push(is_date_format(id, code));
            }
        }
    }
    let sheet = part(&archive, &sheet_name, inflate)?;
    let data = sheet
        .root()
        .child(MAIN, "sheetData")
        .ok_or_else(|| error("no sheet data"))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in data.children_named(MAIN, "row") {
        let index = row
            .attribute("", "r")
            .and_then(|r| r.parse::<usize>().ok())
            .map_or(rows.len(), |r| r.saturating_sub(1));
        if index >= MAX_ROWS {
            break;
        }
        let mut values: Vec<String> = Vec::new();
        for cell in row.children_named(MAIN, "c") {
            let column = cell
                .attribute("", "r")
                .and_then(column_of)
                .unwrap_or(values.len());
            if column >= MAX_COLUMNS {
                continue;
            }
            let raw = cell.child(MAIN, "v").map(|v| v.text()).unwrap_or_default();
            let value = match cell.attribute("", "t").unwrap_or("n") {
                "s" => raw
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| shared.get(index).cloned())
                    .unwrap_or_default(),
                "inlineStr" => cell
                    .child(MAIN, "is")
                    .map(|item| string_item(&item))
                    .unwrap_or_default(),
                "b" => if raw.trim() == "1" { "TRUE" } else { "FALSE" }.to_string(),
                "str" | "e" => raw,
                _ if raw.is_empty() => String::new(),
                _ => {
                    let style = cell
                        .attribute("", "s")
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    if date_styles.get(style).copied().unwrap_or(false) {
                        raw.trim()
                            .parse::<f64>()
                            .ok()
                            .and_then(serial_date)
                            .unwrap_or_else(|| number_text(&raw))
                    } else {
                        number_text(&raw)
                    }
                }
            };
            if values.len() <= column {
                values.resize(column + 1, String::new());
            }
            values[column] = value;
        }
        if rows.len() <= index {
            rows.resize(index + 1, Vec::new());
        }
        rows[index] = values;
    }
    table(rows)
}

/// A shared or inline string: its `t` text, or its runs' (phonetic runs
/// left out).
fn string_item(item: &Element) -> String {
    let mut out = String::new();
    for child in item.children() {
        if child.is(MAIN, "t") {
            out.push_str(&child.text());
        } else if child.is(MAIN, "r") {
            if let Some(text) = child.child(MAIN, "t") {
                out.push_str(&text.text());
            }
        }
    }
    out
}

/// A paragraph's text with ODF's spaces, tabs, and line breaks.
fn odf_text(paragraph: &Element, out: &mut String) {
    for node in paragraph.nodes() {
        match node {
            Content::Text(text) => out.push_str(text),
            Content::Element(element) if element.is(TEXT, "s") => {
                let count = element
                    .attribute(TEXT, "c")
                    .and_then(|c| c.parse::<usize>().ok())
                    .unwrap_or(1)
                    .min(1000);
                out.extend(std::iter::repeat_n(' ', count));
            }
            Content::Element(element) if element.is(TEXT, "tab") => out.push('\t'),
            Content::Element(element) if element.is(TEXT, "line-break") => out.push('\n'),
            Content::Element(element) => odf_text(&element, out),
        }
    }
}

/// An ODF date value (`1994-03-15` or with a time) as M/D/YYYY.
fn odf_date(value: &str) -> Option<String> {
    let date = value.split('T').next()?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    Some(format!("{month}/{day}/{year}"))
}

/// The first sheet of an ODS spreadsheet.
pub fn read_ods(bytes: &[u8], inflate: Inflate) -> Result<DataTable, DataError> {
    let archive = Archive::open(bytes).map_err(|e| error(format!("{e:?}")))?;
    let content = part(&archive, "content.xml", inflate)?;
    let sheet = content
        .root()
        .child(OFFICE, "body")
        .and_then(|body| body.child(OFFICE, "spreadsheet"))
        .and_then(|spreadsheet| spreadsheet.child(TABLE, "table"))
        .ok_or_else(|| error("no sheet"))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row_elements = Vec::new();
    for child in sheet.children() {
        if child.is(TABLE, "table-row") {
            row_elements.push(child);
        } else if child.is(TABLE, "table-rows") || child.is(TABLE, "table-header-rows") {
            row_elements.extend(child.children_named(TABLE, "table-row"));
        }
    }
    for row in row_elements {
        let repeat = row
            .attribute(TABLE, "number-rows-repeated")
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let mut values: Vec<String> = Vec::new();
        for cell in row.children() {
            if !(cell.is(TABLE, "table-cell") || cell.is(TABLE, "covered-table-cell")) {
                continue;
            }
            let columns = cell
                .attribute(TABLE, "number-columns-repeated")
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(1)
                .max(1);
            let value = match cell.attribute(OFFICE, "value-type").unwrap_or("") {
                "float" | "percentage" | "currency" => cell
                    .attribute(OFFICE, "value")
                    .map(number_text)
                    .unwrap_or_default(),
                "date" => cell
                    .attribute(OFFICE, "date-value")
                    .and_then(odf_date)
                    .unwrap_or_default(),
                "boolean" => match cell.attribute(OFFICE, "boolean-value") {
                    Some("true") => "TRUE".into(),
                    Some(_) => "FALSE".into(),
                    None => String::new(),
                },
                _ => {
                    let mut text = String::new();
                    for (index, paragraph) in cell.children_named(TEXT, "p").enumerate() {
                        if index > 0 {
                            text.push('\n');
                        }
                        odf_text(&paragraph, &mut text);
                    }
                    text
                }
            };
            if value.is_empty() {
                // Empty columns count but are never expanded.
                let width = values.len().saturating_add(columns).min(MAX_COLUMNS);
                values.resize(width, String::new());
            } else {
                for _ in 0..columns {
                    if values.len() >= MAX_COLUMNS {
                        break;
                    }
                    values.push(value.clone());
                }
            }
        }
        while values.last().is_some_and(|value| value.is_empty()) {
            values.pop();
        }
        let copies = if values.is_empty() { 1 } else { repeat };
        for _ in 0..copies {
            if rows.len() >= MAX_ROWS {
                break;
            }
            rows.push(values.clone());
        }
        if rows.len() >= MAX_ROWS {
            break;
        }
    }
    table(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::{Entry, write};

    fn no_inflate(_: &[u8], _: usize) -> Option<Vec<u8>> {
        None
    }

    fn xlsx() -> Vec<u8> {
        let workbook = format!(
            r#"<workbook xmlns="{MAIN}" xmlns:r="{RELATIONSHIPS}"><sheets><sheet name="Clients" sheetId="1" r:id="rId3"/><sheet name="Other" sheetId="2" r:id="rId4"/></sheets></workbook>"#
        );
        let relationships = format!(
            r#"<Relationships xmlns="{PACKAGE_RELATIONSHIPS}"><Relationship Id="rId4" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Target="worksheets/sheet1.xml"/></Relationships>"#
        );
        let strings = format!(
            r#"<sst xmlns="{MAIN}"><si><t>LAST</t></si><si><t>CITY</t></si><si><r><t>Al</t></r><r><t>len</t></r></si><si><t xml:space="preserve">Boston </t></si></sst>"#
        );
        let styles = format!(
            r#"<styleSheet xmlns="{MAIN}"><numFmts><numFmt numFmtId="164" formatCode="d/m/yyyy"/></numFmts><cellXfs><xf numFmtId="0"/><xf numFmtId="164"/><xf numFmtId="14"/></cellXfs></styleSheet>"#
        );
        let sheet = format!(
            r#"<worksheet xmlns="{MAIN}"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="inlineStr"><is><t>AMOUNT</t></is></c><c r="D1" t="inlineStr"><is><t>DUE</t></is></c></row><row r="3"><c r="A3" t="s"><v>2</v></c><c r="B3" t="s"><v>3</v></c><c r="C3"><v>150</v></c><c r="D3" s="1"><v>34408</v></c></row><row r="4"><c r="A4" t="inlineStr"><is><t>Beck</t></is></c><c r="C4"><v>75.5</v></c><c r="D4" s="2"><v>34336</v></c></row></sheetData></worksheet>"#
        );
        write(
            &[
                Entry::stored("xl/workbook.xml", workbook),
                Entry::stored("xl/_rels/workbook.xml.rels", relationships),
                Entry::stored("xl/sharedStrings.xml", strings),
                Entry::stored("xl/styles.xml", styles),
                Entry::stored("xl/worksheets/sheet1.xml", sheet),
                Entry::stored(
                    "xl/worksheets/sheet2.xml",
                    format!(r#"<worksheet xmlns="{MAIN}"/>"#),
                ),
            ],
            None,
        )
    }

    #[test]
    fn an_xlsx_workbooks_first_sheet_is_a_data_file() {
        let table = read_xlsx(&xlsx(), &no_inflate).unwrap();
        assert_eq!(table.names, ["LAST", "CITY", "AMOUNT", "DUE"]);
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[0], ["Allen", "Boston ", "150", "3/15/1994"]);
        assert_eq!(table.records[1], ["Beck", "", "75.5", "1/2/1994"]);
    }

    #[test]
    fn an_ods_spreadsheets_first_sheet_is_a_data_file() {
        let content = format!(
            r#"<office:document-content xmlns:office="{OFFICE}" xmlns:table="{TABLE}" xmlns:text="{TEXT}"><office:body><office:spreadsheet><table:table table:name="Clients"><table:table-row><table:table-cell office:value-type="string"><text:p>LAST</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>CITY</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>DUE</text:p></table:table-cell><table:table-cell table:number-columns-repeated="16380"/></table:table-row><table:table-row><table:table-cell office:value-type="string"><text:p>Allen<text:s text:c="2"/>Jr</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>Boston</text:p><text:p>North</text:p></table:table-cell><table:table-cell office:value-type="date" office:date-value="1994-03-15"/></table:table-row><table:table-row table:number-rows-repeated="2"><table:table-cell office:value-type="float" office:value="7"/></table:table-row><table:table-row table:number-rows-repeated="1048570"><table:table-cell table:number-columns-repeated="1024"/></table:table-row></table:table></office:spreadsheet></office:body></office:document-content>"#
        );
        let bytes = write(
            &[
                Entry::stored("mimetype", "application/vnd.oasis.opendocument.spreadsheet"),
                Entry::stored("content.xml", content),
            ],
            None,
        );
        let table = read_ods(&bytes, &no_inflate).unwrap();
        assert_eq!(table.names, ["LAST", "CITY", "DUE"]);
        assert_eq!(table.records.len(), 3);
        assert_eq!(
            table.records[0],
            ["Allen  Jr", "Boston\nNorth", "3/15/1994"]
        );
        assert_eq!(table.records[1], ["7", "", ""]);
    }

    #[test]
    fn serial_dates_and_columns_read() {
        assert_eq!(serial_date(1.0).as_deref(), Some("12/31/1899"));
        assert_eq!(serial_date(45_000.0).as_deref(), Some("3/15/2023"));
        assert_eq!(column_of("AB12"), Some(27));
        assert_eq!(column_of("A1"), Some(0));
        assert!(is_date_format(164, Some("d/m/yyyy")));
        assert!(!is_date_format(164, Some("0.00\"m\"")));
        assert!(read_xlsx(b"nope", &no_inflate).is_err());
    }
}
