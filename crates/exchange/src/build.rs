//! The model resolved into an [`ExchangeDocument`]: Ami Pro's meaning
//! worked out once for every format (`docs/research/EXPORT_NOTES.md`, "The
//! mapping").
//!
//! - Styles keep their names; Body Text comes first, as the default.
//! - Runs carry only what differs from their style; lower case and initial
//!   caps are written as shown (no format has them).
//! - Page layouts become sections: the start layout, then each inserted
//!   layout from its break; a floating header or footer starts a section
//!   at the first paragraph of the page after its mark.
//! - Frames are placed where the layout put them: Where placed frames in
//!   the first paragraph of their page, with page positions; anchored
//!   frames at their anchor; repeating frames in the headers.
//! - A table alone in its line (Ami Pro's page table) is a table between
//!   paragraphs; other tables are in their frames.

use std::collections::{BTreeMap, HashMap};

use amipro_document::{
    Alignment as AmiAlignment, BreakKind, Caps, CellSpan, CharacterAttributes, CharacterFormat,
    Document, FieldContext, FlowId, Frame, FrameContent, FrameId, HeaderFooterKind, InlineObject,
    LayoutId, LineSpacing as AmiLineSpacing, OBJECT_REPLACEMENT_CHARACTER, PageGeometry,
    PageLayout, PageSides, Paragraph as AmiParagraph, ParagraphStyle, Picture, PictureFormat,
    Placement, PowerMark, RepeatPages, RuleLength, Size, StyleId, TabKind as AmiTabKind,
    TabLeader as AmiTabLeader, TabStop, Twips, Wrap, line_style_width, power_kind, rule_strokes,
};
use amipro_layout::{Container, DocumentLayout, display_text};

use crate::losses::{Feature, Features};
use crate::numbering::{Lists, Numbered, bullet_character};
use crate::{
    Alignment, Anchor, Block, Cell, CharProps, Columns, ExchangeDocument, Field, FieldKind, Font,
    FontFamily, HeaderFooters, HorizontalFrom, Image, Inline, LineKind, LineSpacing, NumberFormat,
    PageBorder, PageNumbering, PageSetup, Paragraph, ParagraphProps, PlacedPicture, Properties,
    Row, RowHeight, RuleLine, Script, Section, Settings, Shape, ShapeContent, ShapeLine,
    ShapeShadow, ShapeWrap, Style, Tab, TabKind, TabLeader, Table, Underline, VerticalFrom,
    VerticalMerge,
};

/// Converts pictures for the formats: the app decodes and encodes them.
pub trait PictureSink {
    /// `picture` in a form the formats take, and its natural size (Ami
    /// Pro's, which Graphics Scaling scales); `None` when it cannot be
    /// read (its frame is written empty and listed). A turned picture comes
    /// turned, to be stretched over its rectangle.
    fn picture(&mut self, document: &Document, picture: &Picture) -> Option<SunkPicture>;
}

/// A converted picture.
#[derive(Clone, Debug)]
pub struct SunkPicture {
    pub image: Image,
    /// Its natural size, which Graphics Scaling scales; `None` for a
    /// picture that covers its picture area (Ami Pro's screen image of a
    /// drawing or chart).
    pub natural: Option<Size>,
}

/// A sink that converts nothing: every picture is listed as missing.
pub struct NoPictures;

impl PictureSink for NoPictures {
    fn picture(&mut self, _: &Document, _: &Picture) -> Option<SunkPicture> {
        None
    }
}

/// What the builder needs besides the document.
#[derive(Clone, Debug, Default)]
pub struct BuildOptions<'a> {
    /// The document's pages as the app lays them out, which place Where
    /// placed frames, anchored frames, and floating headers; without it
    /// frames go to their anchor's paragraph and the document's start.
    pub layout: Option<&'a DocumentLayout>,
    /// What dates and Doc Info entries show.
    pub fields: FieldContext,
    /// The document's title: its file's name without the extension.
    pub title: String,
}

/// The exchange document and what it uses that some format lacks.
#[derive(Clone, Debug)]
pub struct Built {
    pub document: ExchangeDocument,
    pub features: Features,
}

/// Resolves `document` into an exchange document.
pub fn build(document: &Document, options: &BuildOptions, sink: &mut dyn PictureSink) -> Built {
    let mut builder = Builder::new(document, options, sink);
    builder.run();
    Built {
        document: builder.out,
        features: builder.features,
    }
}

/// Where a flow's paragraphs are, for their tabs and indents.
#[derive(Clone, Debug)]
struct Context {
    /// The tab stops of the box the flow is laid out in.
    box_tabs: Vec<TabStop>,
    /// Added to every paragraph's left and right indents (a header band
    /// whose margins are not the page's).
    indent_delta: (Twips, Twips),
    main: bool,
}

/// What a main-text paragraph turns into.
enum Piece {
    Block(Box<Block>),
    /// A new section from here, with this layout.
    Layout(LayoutId),
}

struct Builder<'a> {
    document: &'a Document,
    options: &'a BuildOptions<'a>,
    sink: &'a mut dyn PictureSink,
    out: ExchangeDocument,
    features: Features,
    style_index: BTreeMap<StyleId, usize>,
    /// The styles as exchange character and paragraph formatting, by index.
    font_names: HashMap<String, usize>,
    lists: Lists,
    /// Converted pictures by their source object (or link), with their
    /// natural sizes.
    images: HashMap<String, (usize, Option<Size>)>,
    next_shape: u32,
    /// Where placed frames to anchor in main paragraph `index`.
    page_frames: BTreeMap<usize, Vec<FrameId>>,
    /// Pages past the text that hold Where placed frames.
    trailing_pages: BTreeMap<u32, Vec<FrameId>>,
    /// Main paragraphs that start a section for a floating header or footer.
    floating_starts: BTreeMap<usize, Vec<(HeaderFooterKind, PageSides, FlowId)>>,
    /// The page each main paragraph's first line is on.
    paragraph_pages: HashMap<usize, u32>,
}

impl<'a> Builder<'a> {
    fn new(
        document: &'a Document,
        options: &'a BuildOptions<'a>,
        sink: &'a mut dyn PictureSink,
    ) -> Self {
        Self {
            document,
            options,
            sink,
            out: ExchangeDocument::default(),
            features: Features::default(),
            style_index: BTreeMap::new(),
            font_names: HashMap::new(),
            lists: Lists::plan(document.styles()),
            images: HashMap::new(),
            next_shape: 1,
            page_frames: BTreeMap::new(),
            trailing_pages: BTreeMap::new(),
            floating_starts: BTreeMap::new(),
            paragraph_pages: HashMap::new(),
        }
    }

    fn run(&mut self) {
        self.properties();
        self.styles();
        self.place_from_layout();
        self.sections();
        for section in &mut self.out.sections {
            separate_tables(&mut section.blocks);
            for set in [&mut section.headers, &mut section.footers] {
                for blocks in [&mut set.default, &mut set.even, &mut set.first]
                    .into_iter()
                    .flatten()
                {
                    separate_tables(blocks);
                }
            }
        }
        self.out.lists = self.lists.lists.clone();
        self.out.list_instances = self.lists.instances.clone();
        self.settings();
        self.sequence();
    }

    /// A member of a master document shows its page numbers after the
    /// members before (`[sequence]`'s page base, added everywhere; VM MD2)
    /// and numbers its footnotes after theirs (`1058:0000`; VM MD3).
    fn sequence(&mut self) {
        let Some(sequence) = self.document.contents().sequence.as_ref() else {
            return;
        };
        self.out.settings.footnote_start = sequence.footnotes + 1;
        let base = sequence.page_base;
        if base == 0 {
            return;
        }
        for (at, section) in self.out.sections.iter_mut().enumerate() {
            match section.page_numbers.as_mut() {
                Some(numbers) => {
                    if let Some(start) = numbers.start.as_mut() {
                        *start += base;
                    } else if at == 0 {
                        numbers.start = Some(1 + base);
                    }
                }
                None if at == 0 => {
                    section.page_numbers = Some(PageNumbering {
                        format: NumberFormat::Decimal,
                        start: Some(1 + base),
                    });
                }
                None => {}
            }
        }
    }

    // ---- Properties, fonts, and settings ----

    fn properties(&mut self) {
        let info = self.document.info();
        let clock = amipro_document::WallClock::from_ami_pro;
        self.out.properties = Properties {
            title: self.options.title.clone(),
            description: info.description.clone(),
            keywords: info.keywords.clone(),
            custom: (0..8)
                .filter(|index| !info.fields[*index].trim().is_empty())
                .map(|index| {
                    (
                        crate::fields::custom_name(&info.field_names, index),
                        info.fields[index].clone(),
                    )
                })
                .collect(),
            template: self
                .document
                .style_sheet()
                .map(|link| link.name.clone())
                .unwrap_or_default(),
            created: info.created.map(clock),
            modified: self.options.fields.saved.or(info.saved).map(clock),
            revisions: info.revisions,
            editing_minutes: info
                .editing_minutes
                .saturating_add(self.options.fields.session_minutes),
            pages: self
                .options
                .layout
                .map_or(info.statistics.pages, |layout| layout.pages.len() as u32),
            words: info.statistics.words,
            characters: info.statistics.characters,
        };
    }

    /// The font `name` as it is written (`modern_font`), added to the font
    /// table on first use.
    fn font(&mut self, name: &str) -> String {
        let original = name.trim();
        let name = modern_font(original).to_owned();
        if !self.font_names.contains_key(&name) {
            let kind = self
                .document
                .face_kinds()
                .iter()
                .find(|(face, _)| face.eq_ignore_ascii_case(original))
                .map_or(0, |(_, kind)| *kind);
            let family = match kind & 0xF0 {
                0x10 => FontFamily::Roman,
                0x20 => FontFamily::Swiss,
                0x30 => FontFamily::Modern,
                0x40 => FontFamily::Script,
                0x50 => FontFamily::Decorative,
                _ => FontFamily::Unknown,
            };
            self.font_names.insert(name.clone(), self.out.fonts.len());
            self.out.fonts.push(Font {
                name: name.clone(),
                family,
                fixed: kind & 3 == 1,
            });
        }
        name
    }

    fn settings(&mut self) {
        let layouts = self.document.layouts();
        let styles = self.document.styles();
        self.out.settings =
            Settings {
                even_and_odd_headers: self.out.sections.iter().any(|section| {
                    section.headers.even.is_some() || section.footers.even.is_some()
                }),
                mirror_margins: layouts.iter().any(|layout| layout.mirror),
                hyphenation: styles.values().any(|style| style.hyphenation),
                hyphenation_zone: Twips::from(self.document.hyphenation_zone()) * 1440 / 10 / 10,
                ami_pro_rows: true,
                footnote_start: u32::from(self.document.footnote_options().start),
                footnotes_each_page: self.document.footnote_options().reset_each_page(),
                endnotes: self.document.footnote_options().endnotes(),
            };
    }

    // ---- Styles ----

    fn styles(&mut self) {
        let styles = self.document.styles();
        let body = styles
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Body Text"))
            .or_else(|| styles.values().next())
            .map(|style| style.id);
        let order: Vec<StyleId> = body
            .into_iter()
            .chain(styles.keys().copied().filter(|id| Some(*id) != body))
            .collect();
        for (index, id) in order.iter().enumerate() {
            self.style_index.insert(*id, index);
        }
        let tabs = self
            .document
            .document_layout()
            .right
            .layout
            .tab_stops()
            .to_vec();
        for id in &order {
            let style = &styles[id];
            let character = self.style_character(style);
            let mut paragraph = paragraph_props(style, self.document.line_heights());
            paragraph.tabs = Some(exchange_tabs(style.tabs_in_effect(&tabs), 0));
            self.features_of_style(style);
            self.out.styles.push(Style {
                name: style.name.clone(),
                based_on: style
                    .parent
                    .and_then(|parent| self.style_index.get(&parent).copied()),
                next: style
                    .next
                    .and_then(|next| self.style_index.get(&next).copied()),
                paragraph,
                character,
                outline_level: (1..=9)
                    .contains(&style.bullets.level)
                    .then_some(style.bullets.level),
                list: self.lists.styles.get(id).copied(),
            });
        }
    }

    fn features_of_style(&mut self, style: &ParagraphStyle) {
        if style.spacing.tightness != 100 {
            self.features.add(Feature::Tightness);
        }
        if (style.rules.above.on || style.rules.below.on)
            && style.rules.length != RuleLength::Margins
        {
            self.features.add(Feature::ShortRules);
        }
        if style.first_line_bold {
            self.features.add(Feature::FirstLineBold);
        }
    }

    /// A style's character formatting, every field set.
    fn style_character(&mut self, style: &ParagraphStyle) -> CharProps {
        let font = self.font(&style.font_family);
        let mut props = attribute_props(&style.attributes);
        props.font = Some(font);
        props.size = Some(points(style.font_size_points));
        props.bold = Some(style.bold);
        props.italic = Some(style.italic);
        props
    }

    fn style_of(&self, id: StyleId) -> Option<&'a ParagraphStyle> {
        self.document.styles().get(&id)
    }

    fn style_number(&self, id: StyleId) -> usize {
        self.style_index.get(&id).copied().unwrap_or(0)
    }

    // ---- Where the layout put things ----

    fn place_from_layout(&mut self) {
        let main = self.document.main_flow_id();
        let last_paragraph = self.document.main_flow().len().saturating_sub(1);
        let mut first_on_page: BTreeMap<u32, usize> = BTreeMap::new();
        let mut last_page = 1;
        if let Some(layout) = self.options.layout {
            for page in &layout.pages {
                for line in &page.lines {
                    if line.container == Container::Main && line.flow == main {
                        last_page = last_page.max(page.number);
                        if line.bytes.start == 0 {
                            first_on_page
                                .entry(page.number)
                                .or_insert(line.paragraph_index);
                            self.paragraph_pages
                                .entry(line.paragraph_index)
                                .or_insert(page.number);
                        }
                    }
                }
            }
        }
        // Where placed frames: in the first paragraph starting on their
        // page; past the text, on pages of their own.
        for id in self.document.frame_stack() {
            let Some(frame) = self.document.frame(*id) else {
                continue;
            };
            let Placement::Fixed { page, .. } = frame.placement else {
                continue;
            };
            if self.options.layout.is_some() && page > last_page {
                self.trailing_pages.entry(page).or_default().push(*id);
                continue;
            }
            let paragraph = first_on_page
                .range(page..)
                .next()
                .or_else(|| first_on_page.range(..page).next_back())
                .map_or(0, |(_, index)| *index);
            self.page_frames
                .entry(paragraph.min(last_paragraph))
                .or_default()
                .push(*id);
        }
        // Floating headers and footers: from the page after their mark.
        for (index, paragraph) in self.document.main_flow().paragraphs.iter().enumerate() {
            for object in &paragraph.objects {
                if let InlineObject::FloatingHeaderFooter { kind, pages, flow } = object {
                    let page = self.paragraph_pages.get(&index).copied();
                    let start = page
                        .and_then(|page| first_on_page.range(page + 1..).next())
                        .map_or(index + 1, |(_, start)| *start);
                    self.floating_starts
                        .entry(start)
                        .or_default()
                        .push((*kind, *pages, *flow));
                }
            }
        }
    }

    // ---- Sections ----

    fn sections(&mut self) {
        let document = self.document;
        let start = document.start_layout();
        let mut section = self.section(start, false);
        let mut headers = (section.headers.clone(), section.footers.clone());
        let context = Context {
            box_tabs: start.right.layout.tab_stops().to_vec(),
            indent_delta: (0, 0),
            main: true,
        };
        let mut context = context;
        let main = document.main_flow();
        let numbers = self.lists.number_flow(&main.paragraphs, document.styles());
        let mut carry = Carry::default();
        for (index, paragraph) in main.paragraphs.iter().enumerate() {
            if let Some(switches) = self.floating_starts.get(&index).cloned() {
                let mut next = self.section_like(&section, false);
                for (kind, pages, flow) in switches {
                    let blocks = self.header_blocks(flow, &context);
                    let target = match kind {
                        HeaderFooterKind::Header => &mut headers.0,
                        HeaderFooterKind::Footer => &mut headers.1,
                    };
                    match pages {
                        PageSides::All => {
                            target.default = Some(blocks.clone());
                            if target.even.is_some() {
                                target.even = Some(blocks);
                            }
                        }
                        PageSides::Right => target.default = Some(blocks),
                        PageSides::Left => target.even = Some(blocks),
                    }
                    target.first = None;
                }
                next.headers = headers.0.clone();
                next.footers = headers.1.clone();
                let done = std::mem::replace(&mut section, next);
                self.out.sections.push(done);
            }
            let frames = self.page_frames.remove(&index).unwrap_or_default();
            let pieces = self.main_paragraph(
                index,
                paragraph,
                &numbers[index],
                &frames,
                &context,
                &mut carry,
            );
            for piece in pieces {
                match piece {
                    Piece::Block(block) => section.blocks.push(*block),
                    Piece::Layout(layout) => {
                        let Some(layout) = document.layout(layout) else {
                            continue;
                        };
                        let next = self.section(layout, true);
                        headers = (next.headers.clone(), next.footers.clone());
                        context.box_tabs = layout.right.layout.tab_stops().to_vec();
                        let done = std::mem::replace(&mut section, next);
                        self.out.sections.push(done);
                    }
                }
            }
        }
        // Pages past the text that hold Where placed frames.
        let trailing = std::mem::take(&mut self.trailing_pages);
        for frames in trailing.values() {
            let mut paragraph = Paragraph {
                props: ParagraphProps {
                    page_break_before: Some(true),
                    ..ParagraphProps::default()
                },
                ..Paragraph::default()
            };
            for id in frames {
                if let Some(shape) = self.page_frame_shape(*id) {
                    paragraph.inlines.push(Inline::Shape(Box::new(shape)));
                }
            }
            section.blocks.push(Block::Paragraph(paragraph));
        }
        if section.blocks.is_empty() {
            section.blocks.push(Block::Paragraph(Paragraph::default()));
        }
        self.out.sections.push(section);
    }

    /// A section with `layout`'s pages, headers, and footers.
    fn section(&mut self, layout: &PageLayout, new_page: bool) -> Section {
        let geometry = &layout.right;
        if layout
            .left
            .as_ref()
            .is_some_and(|left| !mirrors(geometry, left))
        {
            self.features.add(Feature::LeftPageLayouts);
        }
        if geometry.layout.balance {
            self.features.add(Feature::ColumnBalance);
        }
        let (header_distance, header) =
            self.band(layout, HeaderFooterKind::Header, layout.header.right);
        let (footer_distance, footer) =
            self.band(layout, HeaderFooterKind::Footer, layout.footer.right);
        let left_pages = layout.left.is_some();
        let even_header = left_pages.then(|| {
            self.band(
                layout,
                HeaderFooterKind::Header,
                layout.header.left.or(layout.header.right),
            )
            .1
        });
        let even_footer = left_pages.then(|| {
            self.band(
                layout,
                HeaderFooterKind::Footer,
                layout.footer.left.or(layout.footer.right),
            )
            .1
        });
        let mut headers = HeaderFooters {
            default: header,
            even: even_header.flatten(),
            first: None,
        };
        let mut footers = HeaderFooters {
            default: footer,
            even: even_footer.flatten(),
            first: None,
        };
        if layout.header_from_second_page {
            headers.first = Some(Vec::new());
        }
        if layout.footer_from_second_page {
            footers.first = Some(Vec::new());
        }
        // Repeating frames: in the headers, placed on the page.
        let mut right_shapes = Vec::new();
        let mut left_shapes = Vec::new();
        for id in &layout.repeating {
            let Some(frame) = self.document.frame(*id) else {
                continue;
            };
            let Placement::Repeating { pages, rect } = frame.placement else {
                continue;
            };
            if let Some(shape) =
                self.frame_shape(frame, rect.left, rect.top, rect.width(), rect.height())
            {
                right_shapes.push(shape.clone());
                let mut left = shape;
                if pages == RepeatPages::RightLeft {
                    left.anchor = page_anchor(frame, geometry.width - rect.right, rect.top);
                }
                left.id = self.shape_id();
                left_shapes.push(left);
            }
        }
        if !right_shapes.is_empty() {
            let add = |blocks: &mut Option<Vec<Block>>, shapes: &[Shape]| {
                let blocks = blocks.get_or_insert_with(Vec::new);
                let paragraph = match blocks.first_mut() {
                    Some(Block::Paragraph(paragraph)) => paragraph,
                    _ => {
                        blocks.insert(0, Block::Paragraph(Paragraph::default()));
                        match blocks.first_mut() {
                            Some(Block::Paragraph(paragraph)) => paragraph,
                            _ => unreachable!("just inserted"),
                        }
                    }
                };
                for (at, shape) in shapes.iter().enumerate() {
                    paragraph
                        .inlines
                        .insert(at, Inline::Shape(Box::new(shape.clone())));
                }
            };
            add(&mut headers.default, &right_shapes);
            if left_pages {
                let mut even = headers.even.take().or_else(|| headers.default.clone());
                if let Some(blocks) = &mut even {
                    strip_shapes(blocks);
                }
                headers.even = even;
                add(&mut headers.even, &left_shapes);
            }
            if headers.first.is_some() {
                add(&mut headers.first, &self.renumber(&right_shapes));
            }
        }
        let margins = geometry.layout.margins;
        let text_width = geometry.text_width();
        let columns = &geometry.layout;
        let widths = columns.column_edges.as_ref().map(|edges| {
            edges
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    let gap = edges
                        .get(index + 1)
                        .map_or(0, |next| next.left - column.right);
                    (column.width(), gap)
                })
                .collect()
        });
        let _ = text_width;
        Section {
            page: PageSetup {
                width: geometry.width,
                height: geometry.height,
                landscape: layout.landscape,
                margin_top: margins.top,
                margin_bottom: margins.bottom,
                margin_left: margins.left,
                margin_right: margins.right,
                header_distance,
                footer_distance,
                columns: Columns {
                    count: columns.columns.max(1),
                    gap: columns.gutter,
                    widths,
                    line_between: columns.line_between.is_some(),
                },
                border: page_border(geometry),
            },
            new_page,
            headers,
            footers,
            page_numbers: self.page_numbering(layout),
            blocks: Vec::new(),
        }
    }

    /// A section continuing `section`'s pages (a floating header's).
    fn section_like(&self, section: &Section, new_page: bool) -> Section {
        Section {
            page: section.page.clone(),
            new_page,
            headers: section.headers.clone(),
            footers: section.footers.clone(),
            page_numbers: None,
            blocks: Vec::new(),
        }
    }

    /// Copies of `shapes` with ids of their own.
    fn renumber(&mut self, shapes: &[Shape]) -> Vec<Shape> {
        shapes
            .iter()
            .map(|shape| {
                let mut shape = shape.clone();
                shape.id = self.shape_id();
                shape
            })
            .collect()
    }

    /// A header or footer band: the distance from the paper's edge to its
    /// text, and its text, from the layout's frame (`1110:…`, the band is
    /// the page's width by its top or bottom margin, its text inside the
    /// frame's interior margins).
    fn band(
        &mut self,
        layout: &PageLayout,
        kind: HeaderFooterKind,
        frame: Option<FrameId>,
    ) -> (Twips, Option<Vec<Block>>) {
        let geometry = &layout.right;
        let margins = geometry.layout.margins;
        let Some(frame) = frame.and_then(|id| self.document.frame(id)) else {
            let distance = match kind {
                HeaderFooterKind::Header => margins.top / 2,
                HeaderFooterKind::Footer => margins.bottom / 2,
            };
            return (distance, None);
        };
        let interior = frame.style.interior.margins;
        let distance = match kind {
            HeaderFooterKind::Header => interior.top,
            HeaderFooterKind::Footer => interior.bottom,
        };
        let FrameContent::Text(flow) = frame.content else {
            return (distance, None);
        };
        let context = Context {
            box_tabs: frame.style.interior.tab_stops().to_vec(),
            indent_delta: (interior.left - margins.left, interior.right - margins.right),
            main: false,
        };
        let blocks = self.flow_blocks(flow, &context);
        let empty = blocks.iter().all(|block| match block {
            Block::Paragraph(paragraph) => paragraph.inlines.is_empty(),
            Block::Table(_) => false,
        });
        (distance, (!empty).then_some(blocks))
    }

    /// A floating header's or footer's text.
    fn header_blocks(&mut self, flow: FlowId, context: &Context) -> Vec<Block> {
        let context = Context {
            box_tabs: context.box_tabs.clone(),
            indent_delta: (0, 0),
            main: false,
        };
        self.flow_blocks(flow, &context)
    }

    /// The first page-number field in a layout's header and footer text:
    /// its numbering, as Ami Pro's Start with number gives it.
    fn page_numbering(&mut self, layout: &PageLayout) -> Option<PageNumbering> {
        for id in [layout.header.right, layout.footer.right]
            .into_iter()
            .flatten()
        {
            let Some(FrameContent::Text(flow)) =
                self.document.frame(id).map(|frame| &frame.content)
            else {
                continue;
            };
            let Some(flow) = self.document.flow(*flow) else {
                continue;
            };
            for paragraph in &flow.paragraphs {
                for object in &paragraph.objects {
                    if let InlineObject::PageNumber(field) = object {
                        if field.start_page > 1 && field.start_page != 9999 {
                            self.features.add(Feature::LatePageNumbers);
                        }
                        let start = (field.start_number > 0).then(|| {
                            if field.start_page > 1 {
                                field
                                    .start_number
                                    .saturating_sub(field.start_page - 1)
                                    .max(1)
                            } else {
                                field.start_number
                            }
                        });
                        return Some(PageNumbering {
                            format: crate::fields::page_format(field.style),
                            start,
                        });
                    }
                }
            }
        }
        None
    }

    // ---- Paragraphs ----

    /// A flow's paragraphs (a frame's, a header's, a cell's), numbered on
    /// their own.
    fn flow_blocks(&mut self, flow: FlowId, context: &Context) -> Vec<Block> {
        let Some(flow) = self.document.flow(flow) else {
            return vec![Block::Paragraph(Paragraph::default())];
        };
        let paragraphs = flow.paragraphs.clone();
        let numbers = self.lists.number_flow(&paragraphs, self.document.styles());
        let mut carry = Carry::default();
        let mut blocks = Vec::new();
        for (paragraph, numbered) in paragraphs.iter().zip(&numbers) {
            for piece in self.paragraph_pieces(paragraph, numbered, &[], context, &mut carry) {
                if let Piece::Block(block) = piece {
                    blocks.push(*block);
                }
            }
        }
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::default()));
        }
        blocks
    }

    fn main_paragraph(
        &mut self,
        _index: usize,
        paragraph: &AmiParagraph,
        numbered: &Numbered,
        frames: &[FrameId],
        context: &Context,
        carry: &mut Carry,
    ) -> Vec<Piece> {
        let mut shapes = Vec::new();
        for id in frames {
            if let Some(shape) = self.page_frame_shape(*id) {
                shapes.push(shape);
            }
        }
        self.paragraph_pieces(paragraph, numbered, &shapes, context, carry)
    }

    /// A paragraph as blocks: itself, split where a table stands alone in
    /// its line or an inserted layout begins.
    fn paragraph_pieces(
        &mut self,
        paragraph: &AmiParagraph,
        numbered: &Numbered,
        page_shapes: &[Shape],
        context: &Context,
        carry: &mut Carry,
    ) -> Vec<Piece> {
        let document = self.document;
        let Some(style) = self.style_of(paragraph.style) else {
            return vec![Piece::Block(Box::new(Block::Paragraph(
                Paragraph::default(),
            )))];
        };
        let mut resolved = style.clone();
        paragraph.format.apply(&mut resolved);
        // A table alone in its line: Ami Pro's page table.
        if context.main
            && let Some(frame) = lone_table(document, paragraph)
            && let FrameContent::Table(table) = &frame.content
        {
            let mut pieces = Vec::new();
            if !page_shapes.is_empty() {
                let mut holder = Paragraph {
                    style: self.style_number(paragraph.style),
                    ..Paragraph::default()
                };
                holder.inlines.extend(
                    page_shapes
                        .iter()
                        .cloned()
                        .map(|shape| Inline::Shape(Box::new(shape))),
                );
                pieces.push(Piece::Block(Box::new(Block::Paragraph(holder))));
            }
            // A centred line centres the table alone in it.
            let align = match resolved.alignment {
                AmiAlignment::Centre => Alignment::Centre,
                AmiAlignment::Right => Alignment::Right,
                _ => Alignment::Left,
            };
            let mut table = self.table(table, 0, align);
            if carry.page_break {
                if let Some(row) = table.rows.first_mut() {
                    row.page_break_before = true;
                }
                carry.page_break = false;
            }
            pieces.push(Piece::Block(Box::new(Block::Table(table))));
            return pieces;
        }
        let mut current = self.new_paragraph(paragraph, &resolved, numbered, context, carry);
        current.inlines.extend(
            page_shapes
                .iter()
                .cloned()
                .map(|shape| Inline::Shape(Box::new(shape))),
        );
        let mut pieces = Vec::new();
        let shown = display_text(&paragraph.text, style, &paragraph.runs);
        let fields = mapped_fields(paragraph, &mut self.features);
        self.count_marks(paragraph);
        let mut text = String::new();
        let mut text_props = CharProps::default();
        let flush = |inlines: &mut Vec<Inline>, text: &mut String, props: &CharProps| {
            if !text.is_empty() {
                inlines.push(Inline::Text {
                    text: std::mem::take(text),
                    props: props.clone(),
                });
            }
        };
        let mut objects = paragraph.objects.iter();
        let mut field_at: Option<(usize, FieldKind, String, CharProps)> = None;
        let length = paragraph.text.len();
        // Text bookmarks' marks where they stand (the notes run).
        let bookmark_marks: Vec<(usize, amipro_document::bookmarks::BookmarkMark)> = paragraph
            .marks
            .iter()
            .filter_map(|mark| {
                let parsed = amipro_document::bookmarks::BookmarkMark::parse(&mark.raw)?;
                parsed.cells.is_none().then_some((mark.byte, parsed))
            })
            .collect();
        let mut next_bookmark = 0;
        for (byte, character) in paragraph.text.char_indices() {
            while let Some((at, mark)) = bookmark_marks.get(next_bookmark)
                && *at <= byte
            {
                flush(&mut current.inlines, &mut text, &text_props);
                current.inlines.push(Inline::Bookmark {
                    name: mark.name.clone(),
                    end: mark.end,
                });
                next_bookmark += 1;
            }
            // A power field with a counterpart: its result as the field's.
            if let Some((end, ..)) = &field_at
                && byte >= *end
            {
                let (_, kind, result, props) = field_at.take().expect("checked");
                flush(&mut current.inlines, &mut text, &text_props);
                current.inlines.push(Inline::Field(Field {
                    kind,
                    result,
                    props,
                }));
            }
            // Index and contents entries where they stand: fields that
            // show nothing.
            for (_, _, kind) in fields
                .iter()
                .filter(|(start, end, _)| *start == byte && *end == byte)
            {
                flush(&mut current.inlines, &mut text, &text_props);
                current.inlines.push(Inline::Field(Field {
                    kind: kind.clone(),
                    result: String::new(),
                    props: CharProps::default(),
                }));
            }
            if field_at.is_none()
                && let Some((_, end, kind)) = fields
                    .iter()
                    .find(|(start, end, _)| *start == byte && end > start)
            {
                let props = self.run_props(style, paragraph.format_at(byte));
                field_at = Some((*end, kind.clone(), String::new(), props));
            }
            let format = paragraph.format_at(byte);
            let props = self.run_props(style, format);
            self.char_features(style, format);
            if character == OBJECT_REPLACEMENT_CHARACTER {
                let object = objects.next();
                flush(&mut current.inlines, &mut text, &text_props);
                match object {
                    Some(InlineObject::Break {
                        kind,
                        centre,
                        layout,
                        ..
                    }) => {
                        if *centre {
                            self.features.add(Feature::CentredPages);
                        }
                        let at_end = paragraph.text[byte + character.len_utf8()..]
                            .trim()
                            .is_empty();
                        match (kind, layout) {
                            (BreakKind::Page, Some(layout)) if context.main => {
                                pieces.push(Piece::Block(Box::new(Block::Paragraph(current))));
                                pieces.push(Piece::Layout(*layout));
                                current = self.continuation(paragraph, &resolved, context);
                                if at_end {
                                    return pieces;
                                }
                            }
                            (BreakKind::Page, _) if at_end => carry.page_break = true,
                            (BreakKind::Column, _) if at_end => carry.column_break = true,
                            (BreakKind::Page, _) => current.inlines.push(Inline::PageBreak),
                            (BreakKind::Column, _) => current.inlines.push(Inline::ColumnBreak),
                        }
                    }
                    Some(InlineObject::FrameAnchor(id)) => {
                        if let Some(shape) = self.anchored_shape(*id, paragraph) {
                            current.inlines.push(Inline::Shape(Box::new(shape)));
                        }
                    }
                    Some(InlineObject::PageNumber(field)) => {
                        if !field.leading_text.is_empty() {
                            current.inlines.push(Inline::Text {
                                text: field.leading_text.clone(),
                                props: props.clone(),
                            });
                        }
                        current.inlines.push(Inline::Field(Field {
                            kind: FieldKind::Page {
                                format: crate::fields::page_format(field.style),
                            },
                            result: field.number_on(1).map_or_else(String::new, |number| {
                                crate::numbering::format_number(
                                    crate::fields::page_format(field.style),
                                    number,
                                )
                            }),
                            props: props.clone(),
                        }));
                    }
                    Some(InlineObject::Bullet(code)) => {
                        current.inlines.push(Inline::Symbol {
                            character: bullet_character(*code)
                                .and_then(|text| text.chars().next())
                                .unwrap_or('\u{2022}'),
                            font: None,
                            props: props.clone(),
                        });
                    }
                    Some(InlineObject::Date(field)) => {
                        current.inlines.push(Inline::Field(Field {
                            kind: crate::fields::date_kind(field),
                            result: self.options.fields.date_text(field, document.info()),
                            props: props.clone(),
                        }));
                    }
                    Some(InlineObject::DocInfo(field)) => {
                        let result = self.options.fields.doc_info_text(field, document);
                        match crate::fields::doc_info_kind(field, &document.info().field_names) {
                            Some(kind) => current.inlines.push(Inline::Field(Field {
                                kind,
                                result,
                                props: props.clone(),
                            })),
                            None => current.inlines.push(Inline::Text {
                                text: result,
                                props: props.clone(),
                            }),
                        }
                    }
                    Some(InlineObject::Seq(seq)) => {
                        let name = seq
                            .code
                            .split(|character: char| character.is_whitespace() || character == '%')
                            .find(|part| !part.is_empty())
                            .unwrap_or("Seq");
                        current.inlines.push(Inline::Field(Field {
                            kind: FieldKind::Seq {
                                name: name.to_owned(),
                            },
                            result: seq.text(),
                            props: props.clone(),
                        }));
                    }
                    // Notes as comments, footnotes as the formats' own (the
                    // notes run, NR3).
                    Some(InlineObject::Note { note, flow }) => {
                        self.features.add(Feature::Notes);
                        let note_context = Context {
                            box_tabs: Vec::new(),
                            indent_delta: (0, 0),
                            main: false,
                        };
                        let blocks = self.flow_blocks(*flow, &note_context);
                        let initials = note.initials.clone().unwrap_or_default();
                        current.inlines.push(Inline::Note(Box::new(crate::Note {
                            author: initials.clone(),
                            initials,
                            date: Some(amipro_document::WallClock::from_ami_pro(note.time)),
                            blocks,
                        })));
                    }
                    Some(InlineObject::Footnote { flow }) => {
                        self.features.add(Feature::Footnotes);
                        let note_context = Context {
                            box_tabs: Vec::new(),
                            indent_delta: (0, 0),
                            main: false,
                        };
                        let mut blocks = self.flow_blocks(*flow, &note_context);
                        // The Footnote style's number is the note's own
                        // mark: no list numbers its paragraphs.
                        for block in &mut blocks {
                            if let Block::Paragraph(paragraph) = block {
                                paragraph.list = Some((None, 0));
                                paragraph.number_text = None;
                            }
                        }
                        current
                            .inlines
                            .push(Inline::Footnote(Box::new(crate::Footnote {
                                endnote: document.footnote_options().endnotes(),
                                blocks,
                                props: props.clone(),
                            })));
                    }
                    // A table footnote's text is its table's last rows: its
                    // reference is the number, superscript.
                    Some(InlineObject::TableFootnote { number, .. }) => {
                        let mut props = props.clone();
                        props.script = Some(crate::Script::Super);
                        current.inlines.push(Inline::Text {
                            text: number.to_string(),
                            props,
                        });
                    }
                    Some(InlineObject::FloatingHeaderFooter { .. }) | None => {}
                }
                continue;
            }
            let shown_character = match format.attributes(style).caps {
                Some(Caps::Lower | Caps::Initial) => {
                    shown[byte..].chars().next().unwrap_or(character)
                }
                _ => character,
            };
            if let Some((_, _, result, _)) = &mut field_at {
                result.push(shown_character);
                continue;
            }
            match character {
                '\t' => {
                    flush(&mut current.inlines, &mut text, &text_props);
                    current.inlines.push(Inline::Tab { props });
                }
                '\u{2028}' => {
                    flush(&mut current.inlines, &mut text, &text_props);
                    current.inlines.push(Inline::LineBreak);
                }
                _ => {
                    if props != text_props {
                        flush(&mut current.inlines, &mut text, &text_props);
                        text_props = props;
                    }
                    text.push(shown_character);
                }
            }
        }
        flush(&mut current.inlines, &mut text, &text_props);
        for (_, mark) in &bookmark_marks[next_bookmark..] {
            current.inlines.push(Inline::Bookmark {
                name: mark.name.clone(),
                end: mark.end,
            });
        }
        if let Some((_, kind, result, props)) = field_at.take() {
            current.inlines.push(Inline::Field(Field {
                kind,
                result,
                props,
            }));
        }
        for (_, _, kind) in fields
            .iter()
            .filter(|(start, end, _)| *start == length && *end == length)
        {
            current.inlines.push(Inline::Field(Field {
                kind: kind.clone(),
                result: String::new(),
                props: CharProps::default(),
            }));
        }
        // A paragraph holding only anchors takes no height (Ami Pro's rule):
        // it is left out where it holds tables that follow it.
        let only_anchors = paragraph.text.chars().all(|character| {
            character == OBJECT_REPLACEMENT_CHARACTER || character.is_whitespace()
        }) && paragraph.objects.iter().any(|object| {
            matches!(object, InlineObject::FrameAnchor(id)
            if document.frame(*id).is_some_and(|frame| {
                matches!(frame.placement, Placement::BelowParagraph { .. })
                    && matches!(frame.content, FrameContent::Table(_))
            }))
        }) && current
            .inlines
            .iter()
            .all(|inline| matches!(inline, Inline::Shape(_)))
            && current.inlines.is_empty()
            && current.number_text.is_none();
        // A break before the left-out paragraph goes to its first table.
        let mut break_before = only_anchors && current.props.page_break_before == Some(true);
        if !only_anchors {
            pieces.push(Piece::Block(Box::new(Block::Paragraph(current))));
        }
        // Tables With para above: after the paragraph, at their offset.
        for object in &paragraph.objects {
            let InlineObject::FrameAnchor(id) = object else {
                continue;
            };
            if let Some(frame) = document.frame(*id)
                && let Placement::BelowParagraph { x_offset, .. } = frame.placement
                && let FrameContent::Table(table) = &frame.content
            {
                let mut table = self.table(table, x_offset, Alignment::Left);
                if std::mem::take(&mut break_before)
                    && let Some(row) = table.rows.first_mut()
                {
                    row.page_break_before = true;
                }
                pieces.push(Piece::Block(Box::new(Block::Table(table))));
            }
        }
        pieces
    }

    /// A paragraph's start: its style, formatting over the style, number,
    /// and what the paragraph before it carried over (a break at its end).
    fn new_paragraph(
        &mut self,
        paragraph: &AmiParagraph,
        resolved: &ParagraphStyle,
        numbered: &Numbered,
        context: &Context,
        carry: &mut Carry,
    ) -> Paragraph {
        let style_index = self.style_number(paragraph.style);
        let style = &self.out.styles[style_index];
        let mut props = ParagraphProps::default();
        let full = paragraph_props(resolved, self.document.line_heights());
        let base = &style.paragraph;
        // What the paragraph sets over its style, and the context's shift.
        let (left_delta, right_delta) = context.indent_delta;
        if full.alignment != base.alignment {
            props.alignment = full.alignment;
        }
        let left = full.indent_left.unwrap_or(0) + left_delta;
        if Some(left) != base.indent_left {
            props.indent_left = Some(left);
        }
        let right = full.indent_right.unwrap_or(0) + right_delta;
        if Some(right) != base.indent_right {
            props.indent_right = Some(right);
        }
        if full.first_line != base.first_line {
            props.first_line = full.first_line;
        }
        if full.line != base.line {
            props.line = full.line;
        }
        let tabs = exchange_tabs(resolved.tabs_in_effect(&context.box_tabs), left_delta);
        if Some(&tabs) != base.tabs.as_ref() {
            props.tabs = Some(tabs);
        }
        if std::mem::take(&mut carry.page_break) || carry.page_after {
            props.page_break_before = Some(true);
        }
        carry.page_after = resolved.breaks.page_after();
        let mut inlines = Vec::new();
        if std::mem::take(&mut carry.column_break) || carry.column_after {
            inlines.push(Inline::ColumnBreak);
        }
        carry.column_after = resolved.breaks.column_after();
        if resolved.breaks.column_before() {
            inlines.push(Inline::ColumnBreak);
        }
        let (list, number_text) = match numbered {
            Numbered::None => (None, None),
            Numbered::List(instance, level, shown) => {
                (Some((Some(*instance), *level)), Some(shown.clone()))
            }
            Numbered::Text(text) => {
                let level = style.list.map_or(0, |(_, level)| level);
                (style.list.map(|_| (None, level)), Some(text.clone()))
            }
        };
        let mark = self.run_props(
            self.style_of(paragraph.style).unwrap_or(resolved),
            &paragraph.end_format,
        );
        if paragraph
            .runs
            .iter()
            .any(|run| run.format.no_hyphenation == Some(true))
        {
            if paragraph.runs.iter().any(|run| {
                run.format.no_hyphenation == Some(true)
                    && run.start == 0
                    && run.end == paragraph.text.len()
            }) {
                props.no_hyphenation = Some(true);
            } else if resolved.hyphenation {
                // Only where the style would split words.
                self.features.add(Feature::PartlyUnhyphenated);
            }
        }
        Paragraph {
            style: style_index,
            props,
            list,
            number_text,
            inlines,
            mark,
        }
    }

    /// The paragraph that goes on after an inserted layout's break in the
    /// middle of a paragraph.
    fn continuation(
        &mut self,
        paragraph: &AmiParagraph,
        resolved: &ParagraphStyle,
        context: &Context,
    ) -> Paragraph {
        let mut carry = Carry::default();
        let mut next =
            self.new_paragraph(paragraph, resolved, &Numbered::None, context, &mut carry);
        next.props.page_break_before = None;
        next
    }

    /// What `format` sets over `style`'s character formatting.
    fn run_props(&mut self, style: &ParagraphStyle, format: &CharacterFormat) -> CharProps {
        if format.is_empty() {
            return CharProps::default();
        }
        let mut props = CharProps::default();
        if let Some(family) = &format.font_family
            && !family.eq_ignore_ascii_case(&style.font_family)
        {
            props.font = Some(self.font(family));
        }
        if let Some(size) = format.font_size_points
            && points(size) != points(style.font_size_points)
        {
            props.size = Some(points(size));
        }
        if let Some(bold) = format.bold
            && bold != style.bold
        {
            props.bold = Some(bold);
        }
        if let Some(italic) = format.italic
            && italic != style.italic
        {
            props.italic = Some(italic);
        }
        let mine = attribute_props(&format.attributes(style));
        let theirs = attribute_props(&style.attributes);
        if mine.underline != theirs.underline {
            props.underline = mine.underline;
        }
        if mine.strike != theirs.strike {
            props.strike = mine.strike;
        }
        if mine.script != theirs.script {
            props.script = mine.script;
        }
        if mine.caps != theirs.caps {
            props.caps = mine.caps;
        }
        if mine.small_caps != theirs.small_caps {
            props.small_caps = mine.small_caps;
        }
        if mine.colour != theirs.colour {
            props.colour = mine.colour;
        }
        props
    }

    /// Counts the character formatting no format holds.
    fn char_features(&mut self, style: &ParagraphStyle, format: &CharacterFormat) {
        if format.protected == Some(true) && !style.attributes.protected {
            self.features.add(Feature::ProtectedText);
        }
        if format.overstrike.is_some() {
            self.features.add(Feature::Overstrike);
        }
    }

    /// Counts the kept marks the formats have no place for yet: notes,
    /// footnotes, and bookmarks (their runs come later). Other kept marks
    /// (`<:s>`, a paragraph flag) show nothing.
    fn count_marks(&mut self, paragraph: &AmiParagraph) {
        for mark in &paragraph.marks {
            let raw = mark.raw.as_str();
            if raw.starts_with("<:N") {
                self.features.add(Feature::Notes);
            } else if raw.starts_with("<:F") {
                self.features.add(Feature::Footnotes);
            } else if raw.starts_with("<:Z") && !raw.starts_with("<:Z~") {
                self.features.add(Feature::Bookmarks);
            }
        }
    }

    // ---- Frames ----

    fn shape_id(&mut self) -> u32 {
        let id = self.next_shape;
        self.next_shape += 1;
        id
    }

    /// The layout's box for frame `id` (its first, for a table that goes
    /// on over pages) and its page.
    fn frame_box(&self, id: FrameId) -> Option<(u32, &'a amipro_layout::FrameBox)> {
        let layout = self.options.layout?;
        layout.pages.iter().find_map(|page| {
            page.frames
                .iter()
                .find(|frame_box| frame_box.frame == id)
                .map(|frame_box| (page.number, frame_box))
        })
    }

    /// A Where placed frame as a shape on its page.
    fn page_frame_shape(&mut self, id: FrameId) -> Option<Shape> {
        let frame = self.document.frame(id)?;
        let Placement::Fixed { rect, .. } = frame.placement else {
            return None;
        };
        self.frame_shape(frame, rect.left, rect.top, rect.width(), rect.height())
    }

    /// An anchored frame at its anchor: inline in the line, or below its
    /// paragraph where the layout put it.
    fn anchored_shape(&mut self, id: FrameId, paragraph: &AmiParagraph) -> Option<Shape> {
        let frame = self.document.frame(id)?;
        match frame.placement {
            Placement::Inline { y_offset, size } => {
                let mut shape = self.frame_shape(frame, 0, 0, size.width, size.height)?;
                shape.anchor = Anchor::Inline { drop: y_offset };
                Some(shape)
            }
            // A table below its paragraph goes on in the text, as the
            // layout continues it across pages (`place_table`).
            Placement::BelowParagraph { .. } if matches!(frame.content, FrameContent::Table(_)) => {
                None
            }
            Placement::BelowParagraph { x_offset, size } => {
                let placed = self.frame_box(id).map(|(_, frame_box)| frame_box.bounds);
                let (x, y, width, height) = match placed {
                    Some(bounds) => (
                        bounds.x.round() as Twips,
                        bounds.y.round() as Twips,
                        bounds.width.round() as Twips,
                        bounds.height.round() as Twips,
                    ),
                    None => (x_offset, 0, size.width, size.height),
                };
                let mut shape = self.frame_shape(frame, x, y, width, height)?;
                // Below its paragraph: from the paragraph's top where the
                // layout knows it, else just after it.
                let top = self.paragraph_top(paragraph);
                let wrap = shape_wrap(frame.style.wrap);
                let behind = frame.style.wrap == Wrap::NoWrapAround && !frame.style.opaque;
                shape.anchor = match (placed, top) {
                    (Some(_), Some(top)) => Anchor::Floating {
                        horizontal: (HorizontalFrom::Page, x),
                        vertical: (VerticalFrom::Paragraph, (y - top).max(0)),
                        wrap,
                        behind,
                    },
                    _ => Anchor::Floating {
                        horizontal: (HorizontalFrom::Margin, x_offset),
                        vertical: (VerticalFrom::Paragraph, 0),
                        wrap,
                        behind,
                    },
                };
                Some(shape)
            }
            _ => None,
        }
    }

    /// The top of a main paragraph's first line on its page.
    fn paragraph_top(&self, paragraph: &AmiParagraph) -> Option<Twips> {
        let layout = self.options.layout?;
        layout.pages.iter().find_map(|page| {
            page.lines
                .iter()
                .find(|line| {
                    line.container == Container::Main
                        && line.paragraph == paragraph.id
                        && line.bytes.start == 0
                })
                .map(|line| line.top.round() as Twips)
        })
    }

    /// A frame as a shape `width` × `height` at `left`, `top` on the page.
    fn frame_shape(
        &mut self,
        frame: &Frame,
        left: Twips,
        top: Twips,
        width: Twips,
        height: Twips,
    ) -> Option<Shape> {
        if frame.group.is_some() {
            self.features.add(Feature::Groups);
        }
        let style = &frame.style;
        let border = style.border;
        let line_width = line_style_width(border.style);
        let line = (!border.sides.is_none() && line_width > 0).then(|| {
            let sides = border.sides;
            if !(sides.left && sides.right && sides.top && sides.bottom) {
                self.features.add(Feature::PartialFrameLines);
            }
            ShapeLine {
                kind: line_kind(border.style),
                width: line_width,
                colour: border.colour,
                sides: [sides.top, sides.right, sides.bottom, sides.left],
            }
        });
        let shadow = style.shadow;
        let shadow =
            (shadow.left | shadow.top | shadow.right | shadow.bottom != 0).then_some(ShapeShadow {
                dx: shadow.right - shadow.left,
                dy: shadow.bottom - shadow.top,
                colour: shadow.colour,
            });
        let corner_radius = Twips::from(style.corners.min(100)) * width.min(height) / 200;
        if corner_radius > 0 {
            self.features.add(Feature::RoundedCorners);
        }
        let content = match &frame.content {
            FrameContent::Text(flow) => {
                // Plain text holds a frame's text only after its anchor.
                self.features.add(Feature::Frames);
                if style.interior.columns > 1 {
                    self.features.add(Feature::FrameColumns);
                }
                let context = Context {
                    box_tabs: style.interior.tab_stops().to_vec(),
                    indent_delta: (0, 0),
                    main: false,
                };
                let insets = style.text_insets(width, height);
                ShapeContent::Text {
                    blocks: self.flow_blocks(*flow, &context),
                    insets: [insets.top, insets.right, insets.bottom, insets.left],
                    columns: style.interior.columns.max(1),
                    column_gap: style.interior.gutter,
                }
            }
            FrameContent::Table(table) => {
                let insets = style.text_insets(width, height);
                ShapeContent::Text {
                    blocks: vec![
                        Block::Table(self.table(table, 0, Alignment::Left)),
                        Block::Paragraph(Paragraph::default()),
                    ],
                    insets: [insets.top, insets.right, insets.bottom, insets.left],
                    columns: 1,
                    column_gap: 0,
                }
            }
            FrameContent::Picture(picture) => {
                self.features.add(Feature::Pictures);
                let insets = style.text_insets(width, height);
                let area = Size {
                    width: (width - insets.left - insets.right).max(1),
                    height: (height - insets.top - insets.bottom).max(1),
                };
                self.picture_content(picture, insets.left, insets.top, area)
            }
            FrameContent::Legacy(_) => {
                self.features.add(Feature::UnknownFrames);
                ShapeContent::Empty
            }
        };
        let z = page_frame_z(self.document, frame.id);
        Some(Shape {
            id: self.shape_id(),
            name: style
                .name
                .clone()
                .unwrap_or_else(|| format!("Frame {}", frame.id.raw())),
            anchor: page_anchor(frame, left, top),
            width,
            height,
            z,
            line,
            fill: style.opaque.then_some(style.background),
            shadow,
            corner_radius,
            content,
        })
    }

    /// A picture placed in its frame's picture area (`left`, `top`, `area`
    /// from the frame's corner), scaled as Graphics Scaling says and moved
    /// by its pan (`1168:2dd2`; the page surface's `draw_picture`).
    fn picture_content(
        &mut self,
        picture: &Picture,
        left: Twips,
        top: Twips,
        area: Size,
    ) -> ShapeContent {
        if matches!(
            picture.format(),
            PictureFormat::Drawing
                | PictureFormat::Chart
                | PictureFormat::Ole
                | PictureFormat::Equation
        ) {
            self.features.add(Feature::DrawingsAsPictures);
        }
        let key = match &picture.source {
            amipro_document::PictureSource::Embedded(object) => format!("object {}", object.raw()),
            amipro_document::PictureSource::Linked(path) => format!("link {path}"),
        };
        let sunk = match self.images.get(&key) {
            Some(found) => Some(*found),
            None => match self.sink.picture(self.document, picture) {
                Some(sunk) => {
                    let index = self.out.images.len();
                    self.out.images.push(sunk.image);
                    self.images.insert(key, (index, sunk.natural));
                    Some((index, sunk.natural))
                }
                None => None,
            },
        };
        let Some((image, natural)) = sunk else {
            self.features.add(Feature::MissingPictures);
            return ShapeContent::Empty;
        };
        let size = natural.map_or(area, |natural| picture.scaling.drawn_size(natural, area));
        ShapeContent::Picture(PlacedPicture {
            image,
            left: left + picture.placed_pan().x,
            top: top + picture.placed_pan().y,
            width: size.width.max(1),
            height: size.height.max(1),
            description: picture.description.clone(),
        })
    }

    // ---- Tables ----

    /// A table on Ami Pro's grid: each grid column the column's text width
    /// and its gutter, the text `spacing.left` in from the cell's left and
    /// `spacing.top` down from its top, as Ami Pro places it
    /// (`docs/research/CELL_FRAME_NOTES.md`).
    fn table(&mut self, table: &amipro_document::Table, indent: Twips, align: Alignment) -> Table {
        self.features.add(Feature::Tables);
        let count = table.columns.len().max(1);
        let columns: Vec<Twips> = (0..table.columns.len())
            .map(|column| table.columns[column] + table.column_gutter(column))
            .collect();
        let left = table.spacing.left;
        let top = table.spacing.top;
        let options = &table.row_options;
        let mut rows = Vec::with_capacity(table.rows.len());
        for (row_index, height) in table.rows.iter().enumerate() {
            let full = height + table.row_gutter(row_index);
            let mut cells = Vec::new();
            let mut column = 0;
            while column < count {
                let index = row_index * count + column;
                let Some(cell) = table.cells.get(index) else {
                    break;
                };
                if !cell.span.is_covered() {
                    let span = cell.span;
                    cells.push(Cell {
                        span: span.columns.max(1),
                        merge: if span.rows > 1 {
                            VerticalMerge::Start
                        } else {
                            VerticalMerge::None
                        },
                        blocks: self.cell_blocks(table, index),
                        borders: cell_borders(cell.lines(), cell.format.line_colour),
                        shading: cell.fill(),
                    });
                    column += usize::from(span.columns.max(1));
                    continue;
                }
                // Covered: by a block from a row above (continue it), or
                // by one in this row (already spanned).
                match owner(table, row_index, column) {
                    Some((owner_row, owner_column, span)) if owner_row < row_index => {
                        if owner_column == column {
                            // The block's lines and fill, on its part here.
                            let first = &table.cells[owner_row * count + owner_column];
                            cells.push(Cell {
                                span: span.columns.max(1),
                                merge: VerticalMerge::Continue,
                                blocks: vec![Block::Paragraph(Paragraph::default())],
                                borders: cell_borders(first.lines(), first.format.line_colour),
                                shading: first.fill(),
                            });
                            column += usize::from(span.columns.max(1));
                        } else {
                            column += 1;
                        }
                    }
                    _ => column += 1,
                }
            }
            rows.push(Row {
                height: if options.automatic {
                    RowHeight::AtLeast(full)
                } else {
                    RowHeight::Exact(full)
                },
                heading: options.headings.contains(&row_index),
                keep_together: !options.span_pages,
                page_break_before: options.page_breaks.contains(&row_index),
                cells,
            });
        }
        Table {
            columns,
            rows,
            indent,
            align,
            cell_margins: [
                top,
                (table.cell_margins.horizontal - left).max(0),
                (table.cell_margins.vertical - top).max(0),
                left,
            ],
        }
    }

    /// A cell's text, with the picture it holds inline at its start.
    fn cell_blocks(&mut self, table: &amipro_document::Table, index: usize) -> Vec<Block> {
        let cell = &table.cells[index];
        let amipro_document::CellContent::Text(flow) = cell.content;
        let context = Context {
            box_tabs: Vec::new(),
            indent_delta: (0, 0),
            main: false,
        };
        let mut blocks = self.flow_blocks(flow, &context);
        if let Some(frame) = cell.frame.and_then(|id| self.document.frame(id))
            && let Some(area) = table.cell_text_area(index)
            && let Some(mut shape) = self.frame_shape(frame, 0, 0, area.width(), area.height())
        {
            shape.anchor = Anchor::Inline { drop: 0 };
            if let Some(Block::Paragraph(paragraph)) = blocks.first_mut() {
                paragraph.inlines.insert(0, Inline::Shape(Box::new(shape)));
            }
        }
        blocks
    }
}

/// What one paragraph hands the next.
#[derive(Default)]
struct Carry {
    /// A page or column break at the end of the paragraph before.
    page_break: bool,
    column_break: bool,
    /// The paragraph before's style breaks after it.
    page_after: bool,
    column_after: bool,
}

/// A cell's lines (Lines & Color) as borders: top, right, bottom, left;
/// black, as Ami Pro draws them.
fn cell_borders(
    lines: amipro_document::CellLines,
    colour: amipro_document::Colour,
) -> [Option<RuleLine>; 4] {
    let side = |style: u8| {
        (style != 0).then(|| RuleLine {
            kind: line_kind(u16::from(style)),
            width: rule_strokes(style).iter().sum::<Twips>().max(1),
            // Black, Ami Pro's; another is the app's own (TD8).
            colour,
            space: 0,
        })
    };
    [
        side(lines.top),
        side(lines.right),
        side(lines.bottom),
        side(lines.left),
    ]
}

/// A hairline paragraph between two tables that follow each other, which
/// Word and RTF readers would otherwise join into one table; also inside
/// cells and text boxes.
fn separate_tables(blocks: &mut Vec<Block>) {
    let mut index = 0;
    while index < blocks.len() {
        match &mut blocks[index] {
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        separate_tables(&mut cell.blocks);
                    }
                }
            }
            Block::Paragraph(paragraph) => {
                for inline in &mut paragraph.inlines {
                    if let Inline::Shape(shape) = inline
                        && let ShapeContent::Text { blocks, .. } = &mut shape.content
                    {
                        separate_tables(blocks);
                    }
                }
            }
        }
        if index > 0
            && matches!(blocks[index], Block::Table(_))
            && matches!(blocks[index - 1], Block::Table(_))
        {
            blocks.insert(index, Block::Paragraph(hairline()));
            index += 1;
        }
        index += 1;
    }
}

/// A paragraph a twentieth of a point high, with nothing in it.
fn hairline() -> Paragraph {
    Paragraph {
        props: ParagraphProps {
            space_before: Some(0),
            space_after: Some(0),
            line: Some(LineSpacing::Exact(20)),
            ..ParagraphProps::default()
        },
        mark: CharProps {
            size: Some(20),
            ..CharProps::default()
        },
        ..Paragraph::default()
    }
}

/// The frame whose table stands alone in `paragraph`'s text (only blanks
/// besides): Ami Pro's page table.
fn lone_table<'d>(document: &'d Document, paragraph: &AmiParagraph) -> Option<&'d Frame> {
    let mut found = None;
    let mut objects = paragraph.objects.iter();
    for character in paragraph.text.chars() {
        if character == OBJECT_REPLACEMENT_CHARACTER {
            match objects.next() {
                Some(InlineObject::FrameAnchor(id)) if found.is_none() => {
                    let frame = document.frame(*id)?;
                    if !matches!(frame.placement, Placement::Inline { .. })
                        || !matches!(frame.content, FrameContent::Table(_))
                    {
                        return None;
                    }
                    found = Some(frame);
                }
                _ => return None,
            }
        } else if !character.is_whitespace() {
            return None;
        }
    }
    found
}

/// The cell whose joined block covers row `row`, column `column`: its row,
/// column, and span.
fn owner(
    table: &amipro_document::Table,
    row: usize,
    column: usize,
) -> Option<(usize, usize, CellSpan)> {
    let count = table.columns.len().max(1);
    for owner_row in (0..=row).rev() {
        for owner_column in (0..=column).rev() {
            let cell = table.cells.get(owner_row * count + owner_column)?;
            if cell.span.is_covered() {
                continue;
            }
            let span = cell.span;
            if owner_row + usize::from(span.rows) > row
                && owner_column + usize::from(span.columns) > column
            {
                return Some((owner_row, owner_column, span));
            }
        }
    }
    None
}

/// The power fields of `paragraph` whose function the formats have: their
/// result's bytes and kind (an entry's are empty). The others' results stay
/// text and are counted.
fn mapped_fields(
    paragraph: &AmiParagraph,
    features: &mut Features,
) -> Vec<(usize, usize, FieldKind)> {
    let mut found = Vec::new();
    let mut open: Vec<(usize, PowerMark)> = Vec::new();
    for mark in &paragraph.marks {
        let Some(power) = PowerMark::parse(&mark.raw) else {
            continue;
        };
        if !power.end {
            if let Some(kind) = crate::fields::entry_kind(&power) {
                found.push((mark.byte, mark.byte, kind));
                continue;
            }
            if !amipro_document::has_result(power.kind) && power.kind != power_kind::SEQ {
                // Set, PrintEscape, merge marks: nothing to show.
                features.add(Feature::PowerFields);
                continue;
            }
            open.push((mark.byte, power));
            continue;
        }
        if let Some(position) = open
            .iter()
            .rposition(|(_, start)| start.kind == power.kind && start.code == power.code)
        {
            let (start, mark) = open.remove(position);
            match crate::fields::power_kind(&mark) {
                Some(kind) => found.push((start, mark_end(paragraph, start, &power), kind)),
                None => features.add(Feature::PowerFields),
            }
        }
    }
    for _ in open {
        features.add(Feature::PowerFields);
    }
    found
}

/// The byte where a field's end mark stands.
fn mark_end(paragraph: &AmiParagraph, start: usize, end: &PowerMark) -> usize {
    paragraph
        .marks
        .iter()
        .filter(|mark| mark.byte >= start)
        .find(|mark| PowerMark::parse(&mark.raw).is_some_and(|parsed| parsed == *end))
        .map_or(start, |mark| mark.byte)
}

/// Where a frame sits on its page, with its wrap.
fn page_anchor(frame: &Frame, left: Twips, top: Twips) -> Anchor {
    Anchor::Floating {
        horizontal: (HorizontalFrom::Page, left),
        vertical: (VerticalFrom::Page, top),
        wrap: shape_wrap(frame.style.wrap),
        behind: frame.style.wrap == Wrap::NoWrapAround && !frame.style.opaque,
    }
}

fn shape_wrap(wrap: Wrap) -> ShapeWrap {
    match wrap {
        Wrap::Around => ShapeWrap::Square,
        Wrap::NoWrapBeside => ShapeWrap::TopAndBottom,
        Wrap::NoWrapAround => ShapeWrap::None,
    }
}

/// The frame's place back to front, as the layout stacks them: repeating
/// frames under anchored ones, Where placed ones over both
/// (`page_frame_z`; FC7G, FC8).
fn page_frame_z(document: &Document, frame: FrameId) -> u32 {
    let stack = document.frame_stack();
    let Some(index) = stack.iter().position(|id| *id == frame) else {
        return 1;
    };
    let anchored = document
        .frame(frame)
        .is_some_and(|frame| frame.placement.is_anchored());
    let z = if anchored { index } else { stack.len() + index };
    z as u32 + 2
}

/// The name today's systems know for a font Ami Pro's era named
/// otherwise (EC4): Windows 3's screen fonts and PostScript printer names
/// that neither Windows nor fontconfig resolve any more, as the face they
/// were; Book Antiqua as Palatino Linotype, the same face, which fontconfig
/// maps to its metric clone. Other names are kept.
pub fn modern_font(name: &str) -> &str {
    match name.to_ascii_lowercase().as_str() {
        "tms rmn" | "tmsrmn" | "times new roman ps" | "timesnewromanps" | "timesnewromanpsmt"
        | "times roman" | "dutch" => "Times New Roman",
        "helv" | "arial mt" | "arialmt" | "swiss" => "Arial",
        "helvetica-narrow" | "arial narrow mt" | "arialnarrow" => "Arial Narrow",
        "courier" | "courier10" => "Courier New",
        "book antiqua" | "bookantiqua" => "Palatino Linotype",
        "newcenturyschlbk" => "New Century Schoolbook",
        _ => name,
    }
}

/// Points as twentieths of a point, rounded.
fn points(points: f32) -> Twips {
    (points * 20.0).round() as Twips
}

/// Ami Pro's line style as a line's kind.
fn line_kind(style: u16) -> LineKind {
    match style {
        8 | 9 => LineKind::Double,
        10 => LineKind::Triple,
        11 => LineKind::ThickThin,
        12 => LineKind::ThinThick,
        _ => LineKind::Single,
    }
}

/// A style's character attributes as exchange formatting.
fn attribute_props(attributes: &CharacterAttributes) -> CharProps {
    CharProps {
        underline: Some(if attributes.double_underline {
            Underline::Double
        } else if attributes.word_underline {
            Underline::Words
        } else if attributes.underline {
            Underline::Single
        } else {
            Underline::None
        }),
        strike: Some(attributes.strikethrough),
        script: Some(if attributes.superscript {
            Script::Super
        } else if attributes.subscript {
            Script::Sub
        } else {
            Script::Normal
        }),
        caps: Some(attributes.caps == Some(Caps::Upper)),
        small_caps: Some(attributes.caps == Some(Caps::Small)),
        colour: Some(attributes.colour),
        ..CharProps::default()
    }
}

/// A style's paragraph formatting, every field set (tabs apart). Where
/// the document has Ami Pro's printer line height for the style's font,
/// single, 1½, and double spacing are at least that line, as Ami Pro set
/// them (EC5; `Document::line_heights`, `10d0:1ecb`).
fn paragraph_props(
    style: &ParagraphStyle,
    heights: &amipro_document::LineHeights,
) -> ParagraphProps {
    let printer = heights.get(&amipro_document::FaceKey::new(
        &style.font_family,
        style.font_size_points,
        style.bold,
        style.italic,
    ));
    let indents = style.indents;
    let rule = |side: amipro_document::RuleSide| {
        side.on.then(|| {
            let strokes = rule_strokes(side.line);
            let width: Twips = strokes.iter().sum();
            RuleLine {
                kind: line_kind(u16::from(side.line)),
                width: width.max(1),
                colour: style.rules.colour,
                space: (side.space - width).max(0),
            }
        })
    };
    ParagraphProps {
        alignment: Some(match style.alignment {
            AmiAlignment::Left => Alignment::Left,
            AmiAlignment::Right => Alignment::Right,
            AmiAlignment::Centre => Alignment::Centre,
            AmiAlignment::Justify => Alignment::Justify,
        }),
        indent_left: Some(indents.all + indents.rest),
        indent_right: Some(indents.right),
        first_line: Some(indents.first - indents.rest),
        space_before: Some(style.spacing.above),
        space_after: Some(style.spacing.below),
        line: Some(match (style.spacing.line, printer) {
            (AmiLineSpacing::Single, Some(single)) => LineSpacing::AtLeast(single),
            (AmiLineSpacing::OneAndAHalf(_), Some(single)) => {
                LineSpacing::AtLeast(single + single / 2)
            }
            (AmiLineSpacing::Double(_), Some(single)) => LineSpacing::AtLeast(single * 2),
            (AmiLineSpacing::Single, None) => LineSpacing::Single,
            (AmiLineSpacing::OneAndAHalf(_), None) => LineSpacing::Multiple(1.5),
            (AmiLineSpacing::Double(_), None) => LineSpacing::Multiple(2.0),
            (AmiLineSpacing::Custom(height), _) => LineSpacing::Exact(height),
        }),
        keep_with_next: Some(style.breaks.keep_with_next()),
        keep_together: Some(!style.breaks.within()),
        page_break_before: Some(style.breaks.page_before()),
        column_break_before: Some(style.breaks.column_before()),
        tabs: None,
        border_top: Some(rule(style.rules.above)),
        border_bottom: Some(rule(style.rules.below)),
        no_hyphenation: Some(!style.hyphenation),
    }
}

/// Ami Pro's tab stops as the formats': positions moved by `delta`.
fn exchange_tabs(stops: &[TabStop], delta: Twips) -> Vec<Tab> {
    stops
        .iter()
        .map(|stop| Tab {
            position: stop.position + delta,
            kind: match stop.kind {
                AmiTabKind::Left => TabKind::Left,
                AmiTabKind::Centre => TabKind::Centre,
                AmiTabKind::Right => TabKind::Right,
                AmiTabKind::Decimal => TabKind::Decimal,
            },
            leader: match stop.leader {
                AmiTabLeader::None => TabLeader::None,
                AmiTabLeader::Dots => TabLeader::Dots,
                AmiTabLeader::Hyphens => TabLeader::Hyphens,
                AmiTabLeader::Line => TabLeader::Line,
            },
        })
        .collect()
}

/// Lines around the page (`page_strokes`): their distance from the paper's
/// edge as the line's position puts it in the margin.
fn page_border(geometry: &PageGeometry) -> Option<PageBorder> {
    let border = geometry.border?;
    if border.sides.is_none() {
        return None;
    }
    let strokes = rule_strokes(u8::try_from(border.style).unwrap_or(0));
    let width: Twips = strokes.iter().sum();
    let margin = geometry.layout.margins.top;
    let distance = match border.position {
        amipro_document::LinePosition::Outside => 0,
        amipro_document::LinePosition::Inside => margin - width,
        amipro_document::LinePosition::CloseToOutside => margin / 4 - width / 2,
        amipro_document::LinePosition::Middle => margin / 2 - width / 2,
        amipro_document::LinePosition::CloseToInside => margin * 3 / 4 - width / 2,
    }
    .max(0);
    Some(PageBorder {
        top: border.sides.top,
        bottom: border.sides.bottom,
        left: border.sides.left,
        right: border.sides.right,
        line: RuleLine {
            kind: line_kind(border.style),
            width: width.max(1),
            colour: border.colour,
            space: 0,
        },
        distance,
    })
}

/// Whether left pages are right pages mirrored: the same paper, the same
/// columns, and the left and right margins swapped (or equal).
fn mirrors(right: &PageGeometry, left: &PageGeometry) -> bool {
    let (r, l) = (right.layout.margins, left.layout.margins);
    right.width == left.width
        && right.height == left.height
        && r.top == l.top
        && r.bottom == l.bottom
        && ((r.left == l.right && r.right == l.left) || (r.left == l.left && r.right == l.right))
        && right.layout.columns == left.layout.columns
}

/// Removes the shapes from a header's first paragraph (a copy for left
/// pages gets its own).
fn strip_shapes(blocks: &mut [Block]) {
    if let Some(Block::Paragraph(paragraph)) = blocks.first_mut() {
        paragraph
            .inlines
            .retain(|inline| !matches!(inline, Inline::Shape(_)));
    }
}
