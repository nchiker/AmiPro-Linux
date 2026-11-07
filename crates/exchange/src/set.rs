//! A master document's files as one exchange document (MS10,
//! `UI_SPEC.md` §37): each file built on its own with its numbers in the
//! set, then joined in order, each file starting a section on a new page.
//!
//! Ami Pro has no such export: its master documents print file by file
//! (`docs/research/MASTER_NOTES.md` §5). The join keeps what each file
//! shows: its page numbers (the first section of each file states where
//! its numbers start, as the file prints them), its list numbers (each file
//! keeps its own list instances, which start where the set carried them),
//! and its footnotes (the formats number footnotes on through the
//! document, as the set carries them).
//!
//! Styles are shared by name: a style the files define alike is written
//! once. Where a later file defines a style of the same name otherwise, its
//! paragraphs take the first file's definition, as Word and LibreOffice do
//! when a document is inserted into another (MQ4); they are counted as
//! [`Feature::DifferingStyles`].

use crate::losses::{Feature, Features};
use crate::{
    Block, ExchangeDocument, Inline, NumberFormat, PageNumbering, Paragraph, ShapeContent,
};

/// One file of the set, built.
#[derive(Clone, Debug)]
pub struct SetFile {
    pub document: ExchangeDocument,
    pub features: Features,
}

/// The files joined in order into one document whose properties are
/// `properties`' (the master's), with the features of all.
pub fn join(files: Vec<SetFile>, properties: crate::Properties) -> (ExchangeDocument, Features) {
    let mut out = ExchangeDocument {
        properties,
        ..ExchangeDocument::default()
    };
    let mut features = Features::default();
    for (index, file) in files.into_iter().enumerate() {
        for (feature, count) in file.features.0 {
            *features.0.entry(feature).or_default() += count;
        }
        let differing = append(&mut out, file.document, index == 0);
        for _ in 0..differing {
            features.add(Feature::DifferingStyles);
        }
    }
    if out.sections.is_empty() {
        out.sections.push(crate::Section {
            page: crate::PageSetup {
                width: 12_240,
                height: 15_840,
                landscape: false,
                margin_top: 1_440,
                margin_bottom: 1_440,
                margin_left: 1_440,
                margin_right: 1_440,
                header_distance: 720,
                footer_distance: 720,
                columns: crate::Columns {
                    count: 1,
                    gap: 0,
                    widths: None,
                    line_between: false,
                },
                border: None,
            },
            new_page: true,
            headers: crate::HeaderFooters::default(),
            footers: crate::HeaderFooters::default(),
            page_numbers: None,
            blocks: vec![Block::Paragraph(Paragraph::default())],
        });
    }
    if out.styles.is_empty() {
        out.styles.push(crate::Style {
            name: "Body Text".into(),
            based_on: None,
            next: None,
            paragraph: crate::ParagraphProps::default(),
            character: crate::CharProps::default(),
            outline_level: None,
            list: None,
        });
    }
    (out, features)
}

/// The maps from a file's indices to the joined document's.
struct Maps {
    styles: Vec<usize>,
    instances: Vec<usize>,
    images: Vec<usize>,
    shape_ids: u32,
}

/// Appends `file` to `out`; returns how many of its styles differ from a
/// same-named style already there.
fn append(out: &mut ExchangeDocument, mut file: ExchangeDocument, first: bool) -> usize {
    if first {
        out.settings = file.settings.clone();
    } else {
        out.settings.even_and_odd_headers |= file.settings.even_and_odd_headers;
        out.settings.mirror_margins |= file.settings.mirror_margins;
        out.settings.hyphenation |= file.settings.hyphenation;
        out.settings.ami_pro_rows &= file.settings.ami_pro_rows;
    }
    for font in &file.fonts {
        if !out.fonts.iter().any(|known| known.name == font.name) {
            out.fonts.push(font.clone());
        }
    }
    // Lists and their instances are the file's own: its numbers count on
    // their own, from where the set carried them.
    let list_base = out.lists.len();
    out.lists.append(&mut file.lists);
    let instances: Vec<usize> = (0..file.list_instances.len())
        .map(|index| out.list_instances.len() + index)
        .collect();
    for mut instance in file.list_instances.drain(..) {
        instance.list += list_base;
        out.list_instances.push(instance);
    }
    // Styles by name: the first definition holds.
    let mut differing = 0;
    let mut styles = Vec::with_capacity(file.styles.len());
    let mut added = Vec::new();
    for style in &file.styles {
        match out
            .styles
            .iter()
            .position(|known| known.name.eq_ignore_ascii_case(&style.name))
        {
            Some(at) => styles.push(at),
            None => {
                styles.push(out.styles.len() + added.len());
                added.push(style.clone());
            }
        }
    }
    for (index, style) in file.styles.iter().enumerate() {
        let at = styles[index];
        if at < out.styles.len() && !same_style(&out.styles[at], style, &file, out) {
            differing += 1;
        }
    }
    for mut style in added {
        style.based_on = style.based_on.and_then(|at| styles.get(at).copied());
        style.next = style.next.and_then(|at| styles.get(at).copied());
        style.list = style.list.map(|(list, level)| (list + list_base, level));
        out.styles.push(style);
    }
    let images: Vec<usize> = (0..file.images.len())
        .map(|index| out.images.len() + index)
        .collect();
    out.images.append(&mut file.images);
    let maps = Maps {
        styles,
        instances,
        images,
        shape_ids: out
            .sections
            .iter()
            .flat_map(section_blocks)
            .map(|blocks| max_shape_id(blocks))
            .max()
            .map_or(0, |most| most + 1),
    };
    let previous_headers = out
        .sections
        .last()
        .map(|section| (section.headers.is_empty(), section.footers.is_empty()));
    for (at, mut section) in file.sections.into_iter().enumerate() {
        for blocks in section_blocks_mut(&mut section) {
            remap_blocks(blocks, &maps);
        }
        if at == 0 && !first {
            section.new_page = true;
            // Each file shows its own page numbers, as it prints them.
            match section.page_numbers.as_mut() {
                Some(numbers) => {
                    numbers.start.get_or_insert(1);
                }
                None => {
                    section.page_numbers = Some(PageNumbering {
                        format: NumberFormat::Decimal,
                        start: Some(1),
                    });
                }
            }
            // A section without headers takes the one before's in the
            // formats: a file without them gets empty ones.
            if let Some((headers_empty, footers_empty)) = previous_headers {
                let empty = || Some(vec![Block::Paragraph(Paragraph::default())]);
                if section.headers.is_empty() && !headers_empty {
                    section.headers.default = empty();
                }
                if section.footers.is_empty() && !footers_empty {
                    section.footers.default = empty();
                }
            }
        }
        out.sections.push(section);
    }
    differing
}

/// Whether two same-named styles show alike (their links compared by the
/// styles' names).
fn same_style(
    known: &crate::Style,
    style: &crate::Style,
    file: &ExchangeDocument,
    out: &ExchangeDocument,
) -> bool {
    let name = |styles: &[crate::Style], at: Option<usize>| {
        at.and_then(|at| styles.get(at))
            .map(|style| style.name.to_ascii_lowercase())
    };
    known.paragraph == style.paragraph
        && known.character == style.character
        && known.outline_level == style.outline_level
        && name(&out.styles, known.based_on) == name(&file.styles, style.based_on)
}

fn section_blocks(section: &crate::Section) -> impl Iterator<Item = &Vec<Block>> {
    std::iter::once(&section.blocks).chain(
        [&section.headers, &section.footers]
            .into_iter()
            .flat_map(|set| [&set.default, &set.even, &set.first])
            .flatten(),
    )
}

fn section_blocks_mut(section: &mut crate::Section) -> Vec<&mut Vec<Block>> {
    let mut out = vec![&mut section.blocks];
    for set in [&mut section.headers, &mut section.footers] {
        for blocks in [&mut set.default, &mut set.even, &mut set.first]
            .into_iter()
            .flatten()
        {
            out.push(blocks);
        }
    }
    out
}

fn max_shape_id(blocks: &[Block]) -> u32 {
    let mut most = 0;
    visit(blocks, &mut |inline| {
        if let Inline::Shape(shape) = inline {
            most = most.max(shape.id);
        }
    });
    most
}

/// Calls `each` on every inline of `blocks`, through tables, frames,
/// notes, and footnotes.
fn visit(blocks: &[Block], each: &mut dyn FnMut(&Inline)) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                for inline in &paragraph.inlines {
                    each(inline);
                    match inline {
                        Inline::Shape(shape) => {
                            if let ShapeContent::Text { blocks, .. } = &shape.content {
                                visit(blocks, each);
                            }
                        }
                        Inline::Note(note) => visit(&note.blocks, each),
                        Inline::Footnote(footnote) => visit(&footnote.blocks, each),
                        _ => {}
                    }
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        visit(&cell.blocks, each);
                    }
                }
            }
        }
    }
}

fn remap_blocks(blocks: &mut [Block], maps: &Maps) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => remap_paragraph(paragraph, maps),
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        remap_blocks(&mut cell.blocks, maps);
                    }
                }
            }
        }
    }
}

fn remap_paragraph(paragraph: &mut Paragraph, maps: &Maps) {
    paragraph.style = maps.styles.get(paragraph.style).copied().unwrap_or(0);
    if let Some((Some(instance), _)) = paragraph.list.as_mut() {
        *instance = maps.instances.get(*instance).copied().unwrap_or(*instance);
    }
    for inline in &mut paragraph.inlines {
        match inline {
            Inline::Shape(shape) => {
                shape.id += maps.shape_ids;
                match &mut shape.content {
                    ShapeContent::Text { blocks, .. } => remap_blocks(blocks, maps),
                    ShapeContent::Picture(picture) => {
                        picture.image = maps.images.get(picture.image).copied().unwrap_or(0);
                    }
                    ShapeContent::Empty => {}
                }
            }
            Inline::Note(note) => remap_blocks(&mut note.blocks, maps),
            Inline::Footnote(footnote) => remap_blocks(&mut footnote.blocks, maps),
            _ => {}
        }
    }
}
