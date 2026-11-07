//! Ami Pro's own layout caches in a SAM file, read only to compare the
//! layout engine with the original (Phase 2.5d,
//! `docs/research/LAYOUT_COMPARISON.md`). The importer ignores them: they
//! are recomputed on read (`SAM_PAGE_FRAME_SPEC.md` §5.5, §7, §8).

use crate::structure::{RawFrame, read_structure};
use crate::text::split_body;
use crate::{LegacyKind, decode_windows_1252, parse};

/// What Ami Pro computed when it saved the file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOracle {
    /// `[pg]`: the stored page count.
    pub page_count: Option<u32>,
    /// `[pg]`: where each page ends.
    pub pages: Vec<PageEnd>,
    /// `<:#H,W>` at the start of main-text paragraphs.
    pub paragraphs: Vec<ParagraphCache>,
    /// Top-level frames in file order (back to front).
    pub frames: Vec<FrameCache>,
    /// Each style's font and `[spc]` spacing.
    pub styles: Vec<StyleSpacing>,
}

/// One `[pg]` record. The page ends on the line of main-text paragraph
/// `paragraph` (1-based) that starts at `line_start` and holds
/// `line_length` characters; offsets count Ami Pro's internal characters,
/// in which a control takes several.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageEnd {
    pub paragraph: usize,
    pub line_start: u32,
    pub line_length: u32,
    /// `1` on the last page; other bits not yet decoded.
    pub flags: u32,
    pub layout: String,
    /// The table whose anchor ends the page: the first row of its part on
    /// this page and how many rows the part holds (values 13 and 14 of the
    /// record's third group, `11c8:381e`; `TABLE_PAGES_NOTES.md`). `None`
    /// when no table ends the page.
    pub table_rows: Option<(u32, u32)>,
}

/// A paragraph's cached height and text width, in twips.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphCache {
    /// 0-based main-text paragraph index.
    pub paragraph: usize,
    pub height: u32,
    pub width: u32,
}

/// A top-level frame's stored page, flags, rectangle, and interior column
/// edges (absolute twips). For anchored frames the rectangle and page are
/// where Ami Pro last laid them out.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameCache {
    pub page: u32,
    pub flags: u32,
    /// Left, top, right, bottom.
    pub rect: [i32; 4],
    pub columns: Vec<(i32, i32)>,
    /// A table's rows as Ami Pro last measured them: the header's height,
    /// replaced by each `[h]` record's (a row split by Rows span pages
    /// keeps its last part's). Empty for other frames.
    pub table_rows: Vec<i32>,
    /// A table's Automatic bit (header value 7, bit 1).
    pub automatic: bool,
}

impl FrameCache {
    pub fn is_anchored(&self) -> bool {
        self.flags & 0x80000 != 0
    }
}

/// A style's font and `[spc]` values (VM fixtures S1–S6): `flags` holds the
/// line spacing (1 single, 2 one and a half, 4 double, 8 custom) and the Add
/// in choice (16 always, 32 when not at break); `line` is the line height of
/// the fixed kinds (stale for single spacing); `mode` is value 3, the custom
/// height's display unit (1 inches; README31 writes 4).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleSpacing {
    pub name: String,
    pub family: String,
    /// Twentieths of a point.
    pub size: u32,
    pub flags: u32,
    pub mode: u32,
    pub line: u32,
    pub above: u32,
    pub below: u32,
    /// `[algn]` value 1 (1 left, 2 right, 4 centre, 8 justified).
    pub alignment: u32,
    /// `[algn]` values 3–5 in twips, read as the indents of all lines, the
    /// first line, and the other lines (INFERRED; the comparison tests it).
    pub indents: [u32; 3],
    /// The last of the values that follow `[nfmt]` when the style has them
    /// (the right indent, AP31NAV's ruler).
    pub right_indent: u32,
}

fn number<T: std::str::FromStr>(value: Option<&String>) -> Option<T> {
    value.and_then(|value| value.trim().parse().ok())
}

/// Reads the caches. Missing or malformed parts are left out.
pub fn read_layout_oracle(sam: &[u8]) -> LayoutOracle {
    let mut oracle = LayoutOracle::default();
    if let Ok((legacy, _)) = parse(sam, LegacyKind::Document) {
        let mut current: Option<StyleSpacing> = None;
        for section in &legacy.sections {
            match section.name.to_ascii_lowercase().as_str() {
                "pg" => read_page_table(&section.values, &mut oracle),
                "tag" => {
                    oracle.styles.extend(current.take());
                    current = section.values.first().map(|name| StyleSpacing {
                        name: name.clone(),
                        ..StyleSpacing::default()
                    });
                }
                "fnt" => {
                    if let Some(style) = current.as_mut().filter(|style| style.family.is_empty()) {
                        style.family = section.values.first().cloned().unwrap_or_default();
                        style.size = number(section.values.get(1)).unwrap_or(0);
                    }
                }
                "spc" => {
                    if let Some(style) = current.as_mut() {
                        let value = |index| number(section.values.get(index)).unwrap_or(0);
                        style.flags = value(0);
                        style.line = value(1);
                        style.mode = value(2);
                        style.above = value(3);
                        style.below = value(4);
                    }
                }
                "algn" => {
                    if let Some(style) = current.as_mut() {
                        let value = |index| number(section.values.get(index)).unwrap_or(0);
                        style.alignment = value(0);
                        style.indents = [value(2), value(3), value(4)];
                    }
                }
                "nfmt" => {
                    // Six values of its own, then the style's next style
                    // and two numbers.
                    if let Some(style) = current.as_mut().filter(|_| section.values.len() >= 9) {
                        style.right_indent = number(section.values.last()).unwrap_or(0);
                    }
                }
                "brk" | "line" | "spec" => {}
                _ => oracle.styles.extend(current.take()),
            }
        }
        oracle.styles.extend(current);
    }

    let end = sam.iter().position(|byte| *byte == 0).unwrap_or(sam.len());
    let decoded = decode_windows_1252(&sam[..end]);
    let lines: Vec<&str> = decoded
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let Some(body) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[edoc]"))
    else {
        return oracle;
    };
    oracle.frames = read_structure(&lines[..body])
        .frames
        .iter()
        .map(frame_cache)
        .collect();
    let (paragraphs, _) = split_body(lines[body + 1..].iter().copied());
    for (index, paragraph) in paragraphs.iter().enumerate() {
        if let Some((height, width)) = paragraph_cache(paragraph) {
            oracle.paragraphs.push(ParagraphCache {
                paragraph: index,
                height,
                width,
            });
        }
    }
    oracle
}

fn read_page_table(values: &[String], oracle: &mut LayoutOracle) {
    oracle.page_count = number(values.first());
    for record in values.iter().skip(1) {
        let first_group = record.split('\t').next().unwrap_or_default();
        let fields: Vec<&str> = first_group.split_whitespace().collect();
        let field = |index: usize| fields.get(index).and_then(|value| value.parse().ok());
        let (Some(paragraph), Some(line_start), Some(line_length), Some(flags)) =
            (field(0), field(1), field(2), field(3))
        else {
            continue;
        };
        let third: Vec<u32> = record
            .split('\t')
            .nth(2)
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();
        let table_rows = match (third.get(12), third.get(13)) {
            (Some(first), Some(count)) if *count > 0 => Some((*first, *count)),
            _ => None,
        };
        oracle.pages.push(PageEnd {
            paragraph: paragraph as usize,
            line_start,
            line_length,
            flags,
            layout: fields
                .get(9..)
                .map(|name| name.join(" "))
                .unwrap_or_default(),
            table_rows,
        });
    }
}

/// `<:#H,W>` among the controls at the start of a paragraph, after an
/// optional `@Style@` tag.
fn paragraph_cache(paragraph: &str) -> Option<(u32, u32)> {
    let mut rest = paragraph;
    if let Some(tagged) = rest.strip_prefix('@')
        && !tagged.starts_with('@')
    {
        rest = &tagged[tagged.find('@')? + 1..];
    }
    while let Some(control) = rest.strip_prefix('<') {
        let close = control.find('>')?;
        if let Some(cache) = control[..close].strip_prefix(":#") {
            let (height, width) = cache.split_once(',')?;
            return Some((height.trim().parse().ok()?, width.trim().parse().ok()?));
        }
        rest = &control[close + 1..];
    }
    None
}

fn frame_cache(frame: &RawFrame) -> FrameCache {
    let value = |index: usize| number::<i64>(frame.values.get(index)).unwrap_or(0);
    // `[frmlay]` (a LAYOUT): the column count is value 15, then each column
    // is LEFT RIGHT and a tab count followed by its tabs.
    let mut columns = Vec::new();
    let interior = &frame.interior;
    let count: usize = number(interior.get(14)).unwrap_or(0);
    let mut index = 15;
    for _ in 0..count.min(8) {
        let (Some(left), Some(right), Some(tabs)) = (
            number::<i32>(interior.get(index)),
            number::<i32>(interior.get(index + 1)),
            number::<usize>(interior.get(index + 2)),
        ) else {
            break;
        };
        columns.push((left, right));
        index += 3 + tabs;
    }
    let (mut table_rows, mut automatic) = (Vec::new(), false);
    if let crate::structure::RawContent::Table(table) = &frame.content {
        let header: Vec<i64> = table
            .header
            .first()
            .map(|line| {
                line.split_whitespace()
                    .filter_map(|value| value.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        let rows = header.first().copied().unwrap_or(0).clamp(0, 10_000) as usize;
        let height = header.get(2).copied().unwrap_or(0) as i32;
        automatic = header.get(6).is_none_or(|flags| flags & 1 != 0);
        table_rows = vec![height; rows];
        for record in &table.rows {
            let values: Vec<i64> = record
                .split_whitespace()
                .filter_map(|value| value.parse().ok())
                .collect();
            if let (Some(index), Some(size)) = (values.first(), values.get(1))
                && let Some(slot) = usize::try_from(*index)
                    .ok()
                    .and_then(|index| table_rows.get_mut(index))
            {
                *slot = *size as i32;
            }
        }
    }
    FrameCache {
        page: value(0) as u32,
        flags: value(1) as u32,
        rect: [value(2), value(3), value(4), value(5)].map(|value| value as i32),
        columns,
        table_rows,
        automatic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAM: &str = "[ver]\r\n\t4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tTimes New Roman\r\n\t\t240\r\n\t\t0\r\n\t\t49152\r\n\t[algn]\r\n\t\t8\r\n\t\t1\r\n\t\t2880\r\n\t\t0\r\n\t\t144\r\n\t[spc]\r\n\t\t33\r\n\t\t273\r\n\t\t1\r\n\t\t0\r\n\t\t72\r\n\t\t1\r\n\t\t100\r\n\t[nfmt]\r\n\t\t280\r\n\t\t1\r\n\t\t3\r\n\t\t.\r\n\t\t,\r\n\t\t$\r\n\tBody Text\r\n\t0\r\n\t2088\r\n[frm]\r\n\t1\r\n\t721472\r\n\t2160\r\n\t2292\r\n\t5040\r\n\t4452\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 100 100 18 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t720\r\n\t2880 2160\r\n\t[frmlay]\r\n\t\t4452\r\n\t\t2880\r\n\t\t1\r\n\t\t144\r\n\t\t144\r\n\t\t1\r\n\t\t2436\r\n\t\t144\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t160\r\n\t\t5\r\n\t\t14472\r\n\t\t1\r\n\t\t2361\r\n\t\t4839\r\n\t\t0\r\n\t[txt]\r\nFrame text.\r\n\r\n>\r\n[pg]\r\n\t2\r\n\t2 0 27 576 0 0 0 65535 65535 Standard\t65535 0 0\r\n\t3 0 14 1025 0 0 0 65535 65535 My Layout\t65535 0 0\r\n[edoc]\r\n<:#568,9360>First paragraph long enough to wrap \r\nonto two lines.\r\n\r\n@Body Text@<+!><:#284,7920>Second.<:A0>\r\n\r\nThird.\r\n\r\n>\r\n";

    /// A page that ends in a table names the rows of its part (TP1S: page
    /// 1 holds rows 0–10 of the table anchored in paragraph 31).
    #[test]
    fn reads_the_table_rows_of_a_page_end() {
        let mut values = vec!["2".to_owned()];
        values.push("31 0 5 0 0 0 0 65535 65535 Standard\t65535 0 0\t0 0 0 0 0 0 0 65535 0 0 65535 0 0 11 0 0".into());
        values.push("32 0 16 1025 0 0 0 65535 65535 Standard\t65535 0 0\t0 0 0 0 0 0 0 65535 0 0 65535 0 0 0 0 0".into());
        let mut oracle = LayoutOracle::default();
        read_page_table(&values, &mut oracle);
        assert_eq!(oracle.pages[0].table_rows, Some((0, 11)));
        assert_eq!(oracle.pages[1].table_rows, None);
    }

    #[test]
    fn reads_page_ends_paragraph_caches_frames_and_spacing() {
        let oracle = read_layout_oracle(SAM.as_bytes());
        assert_eq!(oracle.page_count, Some(2));
        assert_eq!(
            oracle.pages[0],
            PageEnd {
                paragraph: 2,
                line_start: 0,
                line_length: 27,
                flags: 576,
                layout: "Standard".into(),
                table_rows: None,
            }
        );
        assert_eq!(oracle.pages[1].layout, "My Layout");
        assert_eq!(
            oracle.paragraphs,
            [
                ParagraphCache {
                    paragraph: 0,
                    height: 568,
                    width: 9360,
                },
                ParagraphCache {
                    paragraph: 1,
                    height: 284,
                    width: 7920,
                },
            ]
        );
        let frame = &oracle.frames[0];
        assert!(frame.is_anchored());
        assert_eq!(frame.rect, [2160, 2292, 5040, 4452]);
        assert_eq!(frame.columns, [(2361, 4839)]);
        assert_eq!(
            oracle.styles,
            [StyleSpacing {
                name: "Body Text".into(),
                family: "Times New Roman".into(),
                size: 240,
                flags: 33,
                mode: 1,
                line: 273,
                above: 0,
                below: 72,
                alignment: 8,
                indents: [2880, 0, 144],
                right_indent: 2088,
            }]
        );
    }
}
