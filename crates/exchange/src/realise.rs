//! An exchange document turned into the model: the export's [`crate::build`]
//! run backwards (`docs/research/IMPORT_NOTES.md`, `UI_SPEC.md` §31), so that
//! every reader's result becomes an Ami Pro document the same way.
//!
//! - Styles come in by Ami Pro's Import Options ([`StyleOption`]): Convert
//!   makes the file's styles the document's, Apply gives every paragraph
//!   Body Text with its style's look as formatting, Keep keeps the names
//!   with the document's definitions, Ignore keeps only the file's own
//!   formatting. The file's default style is Body Text (IM4).
//! - What Ami Pro holds only in styles (a paragraph's own spacing above and
//!   below, breaks kept together, rules) comes in through a variant of the
//!   paragraph's style, as an Ami Pro user would make one.
//! - Sections become page layouts: the first the document's own, each later
//!   one an inserted layout from a page break; a section that only changes
//!   its headers is a floating header or footer.
//! - Shapes become frames placed as the file places them: on a page (Where
//!   placed, on the page its anchor's paragraph is laid out on), below a
//!   paragraph (With para above), in the line (Flow with text), or on
//!   every page (repeating, from the headers). Tables are Ami Pro's page
//!   tables: a frame alone in its line.
//! - Comments are Ami Pro notes; footnotes and endnotes numbered paragraphs
//!   at the document's end (IM6). Markers become places for the report.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use amipro_document::{
    Alignment as AmiAlignment, Border, BreakKind, Breaks, Caps, CellContent, CellFormat, CellLines,
    CellMargins, CellSpan, CharacterAttributes, CharacterFormat, CharacterRun, DateField,
    DateSource as AmiDateSource, DocInfoEntry, DocInfoField, Document, FaceKey, Flow, FlowId,
    Frame, FrameContent, FrameId, FrameStyle, HeaderFooterKind, Indents, InlineObject, LayoutBox,
    LayoutId, LegacyData, LegacyMark, LinePosition, LineSpacing as AmiLineSpacing, Margins,
    OBJECT_REPLACEMENT_CHARACTER, PageGeometry, PageLayout, PageNumberField, PageSides,
    Paragraph as AmiParagraph, ParagraphFormat, ParagraphId, ParagraphRules, ParagraphStyle,
    Picture, PictureScaling, PictureSizing, PictureSource, Placement, PowerMark, Rect, RepeatPages,
    RuleLength, RuleSide, SeqField, Shadow, Sides, Size, Spacing, StyleId, StyleKept, StyleOrigin,
    TabKind as AmiTabKind, TabLeader as AmiTabLeader, TabStop, Table as AmiTable, TableCell,
    TableLayout, TableRows, TableSpacing, Twips, Wrap, power_kind,
};
use amipro_layout::{Container, FrameBoxKind, ShapeCache, Shaper, layout_document};

use crate::import::{FindingKind, Findings, StyleOption};
use crate::{
    Alignment, Anchor, Block, CharProps, DateSource, ExchangeDocument, Field, FieldKind,
    FontFamily, HorizontalFrom, Image, ImageKind, Inline, LineKind, LineSpacing, Note,
    NumberFormat, PageSetup, Paragraph, ParagraphProps, RuleLine, Script, Section, Shape,
    ShapeContent, ShapeWrap, Style, Tab, TabKind, TabLeader, Table, Underline, VerticalFrom,
    VerticalMerge,
};

/// What [`realise`] needs besides the exchange document.
#[derive(Clone, Debug)]
pub struct RealiseOptions<'a> {
    pub styles: StyleOption,
    /// The styles a new document starts from (a new document's), or the
    /// document a file is inserted into.
    pub base: &'a Document,
    /// Inserting into `base` (Insert › File…): the styles `base` has keep
    /// its definitions; only new ones are added (IM3).
    pub inserting: bool,
    /// Inserting into a frame's or a table cell's text: tables and frames
    /// become their text, as in a header (Ami Pro's frames and cells hold
    /// text or one table, and its tables are in the main text).
    pub into_text: bool,
    /// The document's title.
    pub title: String,
    /// Ami Pro's time now, for notes without a date.
    pub now: u32,
}

/// A place in the realised document: a marker's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Place {
    pub flow: FlowId,
    pub paragraph: ParagraphId,
    pub byte: usize,
}

/// The realised document and where its markers ended up.
#[derive(Clone, Debug)]
pub struct Realised {
    pub document: Document,
    pub markers: BTreeMap<u32, Place>,
}

/// The names other programs give their default paragraph style, which
/// becomes Body Text (IM4).
pub fn is_default_style_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "normal" | "default paragraph style" | "standard" | "default" | "default style"
    )
}

/// Turns `exchange` into a document, laying it out with `shaper` to place
/// frames that sit on a page.
pub fn realise(
    exchange: &ExchangeDocument,
    options: &RealiseOptions,
    shaper: &mut dyn Shaper,
    findings: &mut Findings,
) -> Realised {
    let mut realiser = Realiser::new(exchange, options, findings);
    realiser.run();
    realiser.settle_page_frames(shaper);
    realiser.apply_lists();
    realiser.measure_rows(shaper);
    Realised {
        document: realiser.document,
        markers: realiser.markers,
    }
}

/// How much taller than its text an Automatic row's minimum may be and
/// still be kept (`measure_rows`): a tenth of an inch, more than the
/// difference between Ami Pro's measure and this app's.
const ROW_SHRINK: Twips = 144;

/// Where a flow's paragraphs are.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Where {
    Main,
    /// A header or footer band, a frame's text, or a note's.
    Other,
    Cell,
}

/// A paragraph that asked for a list: its flow and place, and the list.
struct ListRequest {
    flow: FlowId,
    paragraph: ParagraphId,
    kind: amipro_editing::lists::ListKind,
    depth: u8,
}

struct Realiser<'a, 'f> {
    exchange: &'a ExchangeDocument,
    options: &'a RealiseOptions<'a>,
    findings: &'f mut Findings,
    document: Document,
    /// Each exchange style's model style.
    style_ids: Vec<StyleId>,
    body: StyleId,
    /// Variants of styles for paragraphs' own spacing, by the style and the
    /// values.
    variants: BTreeMap<(StyleId, String), StyleId>,
    markers: BTreeMap<u32, Place>,
    /// Each exchange image's embedded object.
    images: Vec<Option<amipro_document::ObjectId>>,
    footnotes: Vec<(bool, Vec<Block>)>,
    /// Footnotes and endnotes made Ami Pro's footnotes (in the main text),
    /// and notes made, counted (the notes run).
    typed_footnotes: (usize, usize),
    notes_made: u16,
    /// Where placed frames, the paragraph whose page they go on, and for a
    /// frame placed from its paragraph, how far below the paragraph's top.
    page_frames: Vec<(FrameId, ParagraphId, Option<Twips>)>,
    /// Automatic tables, whose rows Ami Pro measures as it opens a file
    /// (`measure_rows`).
    automatic_tables: Vec<FrameId>,
    /// The frames in the stack with their shapes' z.
    stacked: Vec<(FrameId, u32)>,
    lists: Vec<ListRequest>,
    /// The new document's styles the file does not name (Convert on Open).
    base_only: Vec<StyleId>,
    /// The current section's page geometry, for shape positions.
    page: PageSetup,
    used_keys: Vec<u8>,
}

impl<'a, 'f> Realiser<'a, 'f> {
    fn new(
        exchange: &'a ExchangeDocument,
        options: &'a RealiseOptions<'a>,
        findings: &'f mut Findings,
    ) -> Self {
        let base_styles: BTreeMap<StyleId, ParagraphStyle> = options.base.styles().clone();
        let document = Document::empty(options.title.clone(), base_styles);
        let body = document.body_text_style();
        let used_keys = document
            .styles()
            .values()
            .map(|style| style.kept.function_key)
            .filter(|key| *key != 0)
            .collect();
        let page = exchange
            .sections
            .first()
            .map(|section| section.page.clone())
            .unwrap_or_else(letter_setup);
        Self {
            exchange,
            options,
            findings,
            document,
            style_ids: Vec::new(),
            body,
            variants: BTreeMap::new(),
            markers: BTreeMap::new(),
            images: Vec::new(),
            footnotes: Vec::new(),
            typed_footnotes: (0, 0),
            notes_made: 0,
            page_frames: Vec::new(),
            automatic_tables: Vec::new(),
            stacked: Vec::new(),
            lists: Vec::new(),
            base_only: Vec::new(),
            page,
            used_keys,
        }
    }

    fn run(&mut self) {
        self.fonts();
        self.styles();
        self.images();
        self.sections();
        self.endnotes();
        self.properties();
        self.drop_unused_base_styles();
        // Parents and next styles the document has.
        let known: std::collections::BTreeSet<StyleId> =
            self.document.styles().keys().copied().collect();
        for style in self.document.styles_mut().values_mut() {
            if style
                .parent
                .is_some_and(|parent| parent == style.id || !known.contains(&parent))
            {
                style.parent = None;
            }
            if style.next.is_some_and(|next| !known.contains(&next)) {
                style.next = None;
            }
        }
        let mut stacked = std::mem::take(&mut self.stacked);
        stacked.sort_by_key(|(_, z)| *z);
        self.document
            .frame_stack_mut()
            .extend(stacked.into_iter().map(|(id, _)| id));
        if self.exchange.settings.hyphenation_zone > 0 {
            let zone = self.exchange.settings.hyphenation_zone * 100 / 1440;
            self.document
                .set_hyphenation_zone(u16::try_from(zone.clamp(1, 99)).unwrap_or(5));
        }
    }

    // ---- Fonts and styles ----

    /// The fonts' kinds (pitch and family), as Ami Pro's face kinds.
    fn fonts(&mut self) {
        let mut kinds = self.document.face_kinds().clone();
        for font in &self.exchange.fonts {
            let family = match font.family {
                FontFamily::Roman => 0x10,
                FontFamily::Swiss => 0x20,
                FontFamily::Modern => 0x30,
                FontFamily::Script => 0x40,
                FontFamily::Decorative => 0x50,
                FontFamily::Unknown => 0,
            };
            let pitch = if font.fixed { 1 } else { 2 };
            kinds.insert(font.name.trim().to_lowercase(), family | pitch);
        }
        self.document.set_face_kinds(kinds);
    }

    fn styles(&mut self) {
        let exchange = self.exchange;
        // The single-spacing heights first, so that styles read them.
        for style in &exchange.styles {
            self.record_line_height(&style.character, style.paragraph.line);
        }
        if self.options.styles == StyleOption::Convert && !self.options.inserting {
            self.renumber_base_styles();
        }
        // Names first, so that based-on and next resolve.
        let mut names: Vec<String> = Vec::with_capacity(exchange.styles.len());
        for (index, style) in exchange.styles.iter().enumerate() {
            let name = if index == 0 {
                "Body Text".to_owned()
            } else if style.name.eq_ignore_ascii_case("Body Text") {
                // The file's own Body Text beside its default (Word's).
                "Body Text (file)".to_owned()
            } else {
                style.name.trim().to_owned()
            };
            names.push(name);
        }
        let mut ids = Vec::with_capacity(names.len());
        for (index, style) in exchange.styles.iter().enumerate() {
            let name = &names[index];
            let existing = self
                .document
                .styles()
                .values()
                .find(|known| known.name.eq_ignore_ascii_case(name))
                .map(|known| known.id);
            let id = match self.options.styles {
                StyleOption::Apply | StyleOption::Ignore => self.body,
                StyleOption::Keep => existing.unwrap_or_else(|| self.missing_style(name)),
                StyleOption::Convert => match existing {
                    // Inserting keeps the document's own definition (IM3).
                    Some(id) if self.options.inserting => id,
                    Some(id) => {
                        let mut defined = self.style_from(style, name, id);
                        defined.kept.function_key = self.document.styles()[&id].kept.function_key;
                        self.document.styles_mut().insert(id, defined);
                        id
                    }
                    None => {
                        // On Open the file's styles keep their order.
                        let id = if self.options.inserting {
                            self.next_style_id()
                        } else {
                            StyleId(index as u64 + 1)
                        };
                        let mut defined = self.style_from(style, name, id);
                        defined.kept.function_key = self.next_function_key();
                        self.document.styles_mut().insert(id, defined);
                        id
                    }
                },
            };
            ids.push(id);
        }
        // Next styles, once every style has its id.
        if self.options.styles == StyleOption::Convert {
            for (index, style) in exchange.styles.iter().enumerate() {
                let id = ids[index];
                let next = style.next.and_then(|next| ids.get(next).copied());
                let parent = style.based_on.and_then(|parent| ids.get(parent).copied());
                if let Some(defined) = self.document.styles_mut().get_mut(&id)
                    && defined.origin == StyleOrigin::Document
                    && !(self.options.inserting && index == 0)
                {
                    defined.parent = parent.filter(|parent| *parent != id);
                    defined.next = next.filter(|next| *next != id);
                    if defined.next.is_some() {
                        defined.breaks = Breaks(defined.breaks.0 | Breaks::NEXT_STYLE);
                    }
                }
            }
        }
        self.style_ids = ids;
    }

    /// Opening with Convert: the file's styles take the first ids in the
    /// file's order (as a document read from it would), the new
    /// document's other styles after them, so that the styles keep the
    /// file's order.
    fn renumber_base_styles(&mut self) {
        let exchange = self.exchange;
        let base = self.document.styles().clone();
        let count = exchange.styles.len() as u64;
        let named = |name: &str| {
            exchange.styles.iter().enumerate().any(|(index, style)| {
                let file = if index == 0 {
                    "Body Text"
                } else {
                    style.name.trim()
                };
                file.eq_ignore_ascii_case(name)
            })
        };
        let mut moved: BTreeMap<StyleId, StyleId> = BTreeMap::new();
        let mut next = count + 1;
        for style in base.values() {
            if style.name.eq_ignore_ascii_case("Body Text") {
                moved.insert(style.id, StyleId(1));
            } else if let Some(index) = exchange
                .styles
                .iter()
                .position(|file| file.name.trim().eq_ignore_ascii_case(&style.name))
                .filter(|_| named(&style.name))
            {
                moved.insert(style.id, StyleId(index as u64 + 1));
            } else {
                moved.insert(style.id, StyleId(next));
                next += 1;
            }
        }
        let mut styles = BTreeMap::new();
        for style in base.values() {
            let mut style = style.clone();
            style.id = moved[&style.id];
            style.next = style.next.and_then(|id| moved.get(&id).copied());
            style.parent = style.parent.and_then(|id| moved.get(&id).copied());
            styles.insert(style.id, style);
        }
        // Flows made already (the bands') follow.
        let flows: Vec<FlowId> = self.document.flows().map(|(id, _)| id).collect();
        for flow in flows {
            if let Some(flow) = self.document.flow_mut(flow) {
                for paragraph in &mut flow.paragraphs {
                    let paragraph = Arc::make_mut(paragraph);
                    if let Some(id) = moved.get(&paragraph.style) {
                        paragraph.style = *id;
                    }
                }
            }
        }
        *self.document.styles_mut() = styles;
        self.body = StyleId(1);
        self.base_only = (count + 1..next).map(StyleId).collect();
    }

    /// After opening with Convert: the new document's own styles the file
    /// did not use are left out (Body Text stays).
    fn drop_unused_base_styles(&mut self) {
        if self.base_only.is_empty() {
            return;
        }
        let mut used: std::collections::BTreeSet<StyleId> = Default::default();
        for (_, flow) in self.document.flows() {
            for paragraph in &flow.paragraphs {
                used.insert(paragraph.style);
            }
        }
        for style in self.document.styles().values() {
            used.extend(style.next);
            used.extend(style.parent);
        }
        let unused: Vec<StyleId> = self
            .base_only
            .iter()
            .copied()
            .filter(|id| !used.contains(id))
            .collect();
        for id in unused {
            self.document.styles_mut().remove(&id);
        }
    }

    fn next_style_id(&self) -> StyleId {
        StyleId(
            self.document
                .styles()
                .keys()
                .next_back()
                .map_or(1, |id| id.0 + 1),
        )
    }

    /// The next free function key, F2 to F12 without F10 (Ami Pro's Convert
    /// gives each new style one, `10a8:00e2`).
    fn next_function_key(&mut self) -> u8 {
        for key in (2..=9).chain(11..=12) {
            if !self.used_keys.contains(&key) {
                self.used_keys.push(key);
                return key;
            }
        }
        0
    }

    /// A name the document has no style for (Keep style names): a missing
    /// style, shown as Body Text until a sheet with the name is used.
    fn missing_style(&mut self, name: &str) -> StyleId {
        if let Some(found) = self
            .document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case(name))
        {
            return found.id;
        }
        let id = self.next_style_id();
        let body = self.document.styles()[&self.body].clone();
        let style = ParagraphStyle {
            id,
            name: name.to_owned(),
            origin: StyleOrigin::Missing,
            next: None,
            kept: StyleKept {
                function_key: 0,
                ..body.kept.clone()
            },
            ..body
        };
        self.document.styles_mut().insert(id, style);
        id
    }

    /// An exchange style as an Ami Pro style named `name` with `id`.
    fn style_from(&self, style: &Style, name: &str, id: StyleId) -> ParagraphStyle {
        let character = &style.character;
        let paragraph = &style.paragraph;
        let font = character
            .font
            .clone()
            .filter(|font| !font.trim().is_empty())
            .unwrap_or_else(|| "Times New Roman".into());
        let size = character.size.filter(|size| *size > 0).unwrap_or(240) as f32 / 20.0;
        let bold = character.bold.unwrap_or(false);
        let italic = character.italic.unwrap_or(false);
        let mut breaks = 0;
        if paragraph.keep_with_next == Some(true) {
            breaks |= Breaks::KEEP_WITH_NEXT;
        }
        if paragraph.keep_together != Some(true) {
            breaks |= Breaks::WITHIN;
        }
        if paragraph.page_break_before == Some(true) {
            breaks |= Breaks::PAGE_BEFORE;
        }
        if paragraph.column_break_before == Some(true) {
            breaks |= Breaks::COLUMN_BEFORE;
        }
        let tabs = paragraph.tabs.as_deref().map(ami_tabs).unwrap_or_default();
        if paragraph.tabs.is_some() && tabs != ami_tabs(&default_tabs()) {
            breaks |= Breaks::OWN_TABS;
        }
        let line = self.line_spacing(paragraph.line, &font, size, bold, italic);
        let mut bullets = amipro_document::Bullets::default();
        if let Some(level) = style.outline_level {
            bullets.level = level.min(9);
        }
        if let Some((list, level)) = style.list
            && let Some(level) = self
                .exchange
                .lists
                .get(list)
                .and_then(|list| list.levels.get(usize::from(level)))
        {
            bullets = bullets_of(level, bullets.level);
        }
        ParagraphStyle {
            id,
            name: name.to_owned(),
            parent: None,
            font_family: font,
            font_size_points: size,
            bold,
            italic,
            spacing: Spacing {
                line,
                above: paragraph.space_before.unwrap_or(0).max(0),
                below: paragraph.space_after.unwrap_or(0).max(0),
                ..Spacing::default()
            },
            indents: ami_indents(paragraph),
            breaks: Breaks(breaks),
            alignment: paragraph
                .alignment
                .map_or(AmiAlignment::Left, ami_alignment),
            attributes: CharacterAttributes {
                colour: attributes_of(character).colour & amipro_document::WHITE,
                ..attributes_of(character)
            },
            rules: rules_of(
                paragraph.border_top.flatten(),
                paragraph.border_bottom.flatten(),
            ),
            next: None,
            tabs,
            hanging: false,
            hyphenation: paragraph.no_hyphenation == Some(false),
            bullets,
            origin: StyleOrigin::Document,
            both_sides: false,
            first_line_bold: false,
            kept: StyleKept {
                face_bits: face_bits(self.font_family(character.font.as_deref())),
                ..StyleKept::default()
            },
        }
    }

    fn font_family(&self, name: Option<&str>) -> FontFamily {
        name.and_then(|name| {
            self.exchange
                .fonts
                .iter()
                .find(|font| font.name.eq_ignore_ascii_case(name))
        })
        .map_or(FontFamily::Unknown, |font| font.family)
    }

    /// A line spacing in Ami Pro's terms: at least a height becomes single
    /// spacing with that printer line height for the font (the export's
    /// EC5 backwards), multiples of 1½ and 2 its 1½ and double, other
    /// multiples a custom height.
    fn line_spacing(
        &self,
        line: Option<LineSpacing>,
        font: &str,
        size: f32,
        bold: bool,
        italic: bool,
    ) -> AmiLineSpacing {
        match line {
            None | Some(LineSpacing::Single) => AmiLineSpacing::Single,
            Some(LineSpacing::Multiple(multiple)) if (multiple - 1.0).abs() < 0.05 => {
                AmiLineSpacing::Single
            }
            Some(LineSpacing::Multiple(multiple)) if (multiple - 1.5).abs() < 0.05 => {
                AmiLineSpacing::OneAndAHalf(0)
            }
            Some(LineSpacing::Multiple(multiple)) if (multiple - 2.0).abs() < 0.05 => {
                AmiLineSpacing::Double(0)
            }
            Some(LineSpacing::Multiple(multiple)) => {
                // A font's single line is about 1.15 of its size.
                AmiLineSpacing::Custom(
                    ((multiple * size * 20.0 * 1.15).round() as Twips).clamp(1, 31_680),
                )
            }
            Some(LineSpacing::Exact(height)) => AmiLineSpacing::Custom(height.clamp(1, 31_680)),
            Some(LineSpacing::AtLeast(height)) => {
                let key = FaceKey::new(font, size, bold, italic);
                match self.document.line_heights().get(&key) {
                    Some(single) if (height - single * 3 / 2).abs() <= 2 => {
                        AmiLineSpacing::OneAndAHalf(0)
                    }
                    Some(single) if (height - single * 2).abs() <= 2 => AmiLineSpacing::Double(0),
                    _ => AmiLineSpacing::Single,
                }
            }
        }
    }

    /// Records the printer line heights the styles' at-least spacing says
    /// (single spacing at least a height: the export's EC5).
    fn record_line_height(&mut self, character: &CharProps, line: Option<LineSpacing>) {
        let Some(LineSpacing::AtLeast(height)) = line else {
            return;
        };
        if height <= 0 || character.size.is_some_and(|size| size <= 0) {
            return;
        }
        let font = character.font.as_deref().unwrap_or("Times New Roman");
        let size = character.size.unwrap_or(240) as f32 / 20.0;
        let key = FaceKey::new(
            font,
            size,
            character.bold.unwrap_or(false),
            character.italic.unwrap_or(false),
        );
        let mut heights = self.document.line_heights().clone();
        if heights.get(&key).is_none() {
            heights.insert(key, height);
            self.document.set_line_heights(heights);
        }
    }

    // ---- Pictures ----

    fn images(&mut self) {
        for image in &self.exchange.images {
            let object = self.document.add_object(image.bytes.clone());
            self.images.push(Some(object));
        }
    }

    /// Where the main text's paragraphs are: the main text, or the text of
    /// the frame or cell the file goes into.
    fn main_place(&self) -> Where {
        if self.options.into_text {
            Where::Other
        } else {
            Where::Main
        }
    }

    // ---- Sections ----

    fn sections(&mut self) {
        let exchange = self.exchange;
        let main = self.document.main_flow_id();
        let mut paragraphs: Vec<AmiParagraph> = Vec::new();
        let mut previous_headers: Option<(&Section, LayoutId)> = None;
        for (index, section) in exchange.sections.iter().enumerate() {
            self.page = section.page.clone();
            if index == 0 {
                let id = self.document.document_layout().id;
                self.set_layout(id, section);
                previous_headers = Some((section, id));
            } else if section.new_page
                || page_differs(&section.page, &exchange.sections[index - 1].page)
            {
                if !section.new_page {
                    self.findings.count(FindingKind::SectionBreak);
                }
                let layout = self.inserted_layout(section);
                if let Some(last) = paragraphs.last_mut() {
                    push_object(
                        last,
                        InlineObject::Break {
                            kind: BreakKind::Page,
                            centre: false,
                            layout: Some(layout),
                            restart: false,
                        },
                    );
                }
                previous_headers = Some((section, layout));
            } else if let Some((before, _)) = previous_headers {
                // The same pages with other headers or footers: floating ones
                // from the page after the mark.
                if section.page.columns != before.page.columns {
                    self.findings.count(FindingKind::Columns);
                }
                for (kind, set, old) in [
                    (HeaderFooterKind::Header, &section.headers, &before.headers),
                    (HeaderFooterKind::Footer, &section.footers, &before.footers),
                ] {
                    for (pages, new, was) in [
                        (PageSides::All, &set.default, &old.default),
                        (PageSides::Left, &set.even, &old.even),
                    ] {
                        if new != was
                            && let Some(blocks) = new
                        {
                            let flow = self.flow_from_blocks(blocks, Where::Other);
                            if let Some(last) = paragraphs.last_mut() {
                                push_object(
                                    last,
                                    InlineObject::FloatingHeaderFooter { kind, pages, flow },
                                );
                            }
                        }
                    }
                }
            }
            let place = self.main_place();
            let mut blocks = self.paragraphs_of(&section.blocks, main, place);
            paragraphs.append(&mut blocks);
        }
        if paragraphs.is_empty() {
            paragraphs.push(self.document.new_paragraph("", self.body));
        }
        let paragraphs = paragraphs.into_iter().map(Arc::new).collect();
        let _ = self.document.replace_flow_paragraphs(main, paragraphs);
    }

    /// A new inserted layout for `section`.
    fn inserted_layout(&mut self, section: &Section) -> LayoutId {
        let count = self.document.layouts().len();
        let layout = self.document.new_standard_layout(count.to_string());
        let id = layout.id;
        self.document.layouts_mut().push(Arc::new(layout));
        self.set_layout(id, section);
        id
    }

    /// Gives layout `id` the section's page set-up, headers, and footers.
    fn set_layout(&mut self, id: LayoutId, section: &Section) {
        let Some(mut layout) = self.document.layout(id).cloned() else {
            return;
        };
        let setup = &section.page;
        let settings = &self.exchange.settings;
        layout.landscape = setup.landscape;
        layout.right = geometry_of(setup, &layout.right);
        layout.mirror = settings.mirror_margins;
        let even = section.headers.even.is_some() || section.footers.even.is_some();
        if even || settings.mirror_margins {
            let mut left = layout.right.clone();
            if settings.mirror_margins {
                let margins = &mut left.layout.margins;
                std::mem::swap(&mut margins.left, &mut margins.right);
            }
            layout.left = Some(left);
        }
        layout.header_from_second_page = section
            .headers
            .first
            .as_ref()
            .is_some_and(|first| blocks_empty(first));
        layout.footer_from_second_page = section
            .footers
            .first
            .as_ref()
            .is_some_and(|first| blocks_empty(first));
        if section
            .headers
            .first
            .iter()
            .chain(section.footers.first.iter())
            .any(|first| !blocks_empty(first))
        {
            // Ami Pro has no first page's own header text.
            self.findings.count(FindingKind::FirstPageHeaders);
        }
        // The bands: their text and distances.
        let geometry = layout.right.clone();
        for (kind, set, distance) in [
            (
                HeaderFooterKind::Header,
                &section.headers,
                setup.header_distance,
            ),
            (
                HeaderFooterKind::Footer,
                &section.footers,
                setup.footer_distance,
            ),
        ] {
            let slots = match kind {
                HeaderFooterKind::Header => layout.header,
                HeaderFooterKind::Footer => layout.footer,
            };
            if let Some(band) = slots.right.and_then(|frame| self.document.frame_mut(frame)) {
                band.style = band_style(kind, &geometry, distance);
            }
            let empty = vec![Block::Paragraph(Paragraph::default())];
            let left_pages = layout.left.is_some();
            for (side, blocks) in [(slots.right, &set.default), (slots.left, &set.even)] {
                // Left pages have bands of their own, empty where the file
                // has no even header.
                let blocks = match blocks {
                    Some(blocks) => blocks,
                    None if left_pages && side == slots.left => &empty,
                    None => continue,
                };
                let (blocks, shapes) = take_page_shapes(blocks);
                let (blocks, shift) = band_shift(&blocks);
                let frame = match side {
                    Some(frame) => frame,
                    None => {
                        let style = band_style(kind, &geometry, distance);
                        let placement = match kind {
                            HeaderFooterKind::Header => Placement::Header {
                                side: amipro_document::PageSide::Left,
                            },
                            HeaderFooterKind::Footer => Placement::Footer {
                                side: amipro_document::PageSide::Left,
                            },
                        };
                        let frame = self.document.create_text_frame(placement, style);
                        match kind {
                            HeaderFooterKind::Header => layout.header.left = Some(frame),
                            HeaderFooterKind::Footer => layout.footer.left = Some(frame),
                        }
                        frame
                    }
                };
                match self.document.frame(frame).map(|band| band.content.clone()) {
                    Some(FrameContent::Text(flow)) => self.fill_flow(flow, &blocks, Where::Other),
                    _ => {
                        let flow = self.flow_from_blocks(&blocks, Where::Other);
                        if let Some(band) = self.document.frame_mut(frame) {
                            band.content = FrameContent::Text(flow);
                        }
                    }
                }
                if let Some(band) = self.document.frame_mut(frame) {
                    band.style = band_style(kind, &geometry, distance);
                    let margins = &mut band.style.interior.margins;
                    margins.left = (margins.left + shift.0).max(0);
                    margins.right = (margins.right + shift.1).max(0);
                }
                if side == slots.right {
                    for shape in shapes {
                        if let Some(frame) = self.repeating_frame(&shape) {
                            layout.repeating.push(frame);
                        }
                    }
                }
            }
        }
        if let Some(numbering) = section.page_numbers {
            self.number_pages(&layout, numbering);
        }
        if let Some(slot) = self.document.layout_mut(id) {
            *slot = layout;
        }
    }

    /// Gives the page-number fields of a layout's bands their start.
    fn number_pages(&mut self, layout: &PageLayout, numbering: crate::PageNumbering) {
        let Some(start) = numbering.start else { return };
        for frame in [
            layout.header.right,
            layout.footer.right,
            layout.header.left,
            layout.footer.left,
        ]
        .into_iter()
        .flatten()
        {
            let Some(FrameContent::Text(flow)) = self
                .document
                .frame(frame)
                .map(|frame| frame.content.clone())
            else {
                continue;
            };
            let Some(flow) = self.document.flow_mut(flow) else {
                continue;
            };
            for paragraph in &mut flow.paragraphs {
                let paragraph = Arc::make_mut(paragraph);
                for object in &mut paragraph.objects {
                    if let InlineObject::PageNumber(field) = object {
                        field.start_number = start;
                    }
                }
            }
        }
    }

    // ---- Flows and paragraphs ----

    /// A new flow holding `blocks`.
    fn flow_from_blocks(&mut self, blocks: &[Block], place: Where) -> FlowId {
        let id = self.document.allocate_flow_id();
        self.document.insert_flow(id, Flow::default());
        self.fill_flow(id, blocks, place);
        id
    }

    /// Flow `id`'s paragraphs made `blocks`.
    fn fill_flow(&mut self, id: FlowId, blocks: &[Block], place: Where) {
        let mut paragraphs = self.paragraphs_of(blocks, id, place);
        if paragraphs.is_empty() {
            let style = if place == Where::Cell {
                self.document.table_text_style()
            } else {
                self.body
            };
            paragraphs.push(self.document.new_paragraph("", style));
        }
        let _ = self
            .document
            .replace_flow_paragraphs(id, paragraphs.into_iter().map(Arc::new).collect());
    }

    /// `blocks` as paragraphs of flow `flow`: tables become paragraphs
    /// holding their frame (in a cell, their text).
    fn paragraphs_of(&mut self, blocks: &[Block], flow: FlowId, place: Where) -> Vec<AmiParagraph> {
        let mut out: Vec<AmiParagraph> = Vec::new();
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    // A break before: Ami Pro's break at the end of the one
                    // before.
                    let mut paragraph = std::borrow::Cow::Borrowed(paragraph);
                    for (wanted, kind) in [
                        (paragraph.props.page_break_before, BreakKind::Page),
                        (paragraph.props.column_break_before, BreakKind::Column),
                    ] {
                        if wanted != Some(true) || place != Where::Main {
                            continue;
                        }
                        // A column break before the first paragraph: at its
                        // start, as Ami Pro keeps it.
                        if out.is_empty() && kind == BreakKind::Column {
                            paragraph.to_mut().inlines.insert(0, Inline::ColumnBreak);
                        }
                        if let Some(previous) = out.last_mut() {
                            push_object(
                                previous,
                                InlineObject::Break {
                                    kind,
                                    centre: false,
                                    layout: None,
                                    restart: false,
                                },
                            );
                        }
                    }
                    // A column break before the text: the style's own (it
                    // breaks before), or the one before's at its end.
                    if matches!(paragraph.inlines.first(), Some(Inline::ColumnBreak)) {
                        let style_breaks = self
                            .exchange
                            .styles
                            .get(paragraph.style)
                            .is_some_and(|style| style.paragraph.column_break_before == Some(true));
                        let first = out.is_empty();
                        if style_breaks || !first {
                            paragraph.to_mut().inlines.remove(0);
                        }
                        if !style_breaks
                            && !first
                            && place == Where::Main
                            && let Some(previous) = out.last_mut()
                        {
                            push_object(
                                previous,
                                InlineObject::Break {
                                    kind: BreakKind::Column,
                                    centre: false,
                                    layout: None,
                                    restart: false,
                                },
                            );
                        }
                    }
                    let made = self.paragraph(&paragraph, flow, place);
                    out.push(made);
                }
                Block::Table(table) if place != Where::Main => {
                    // A table in a table, a frame, or a header: its text, a
                    // row a paragraph (Ami Pro's frames hold text or a
                    // table, and its tables are in the main text).
                    self.findings.count(FindingKind::NestedTable);
                    for row in &table.rows {
                        let mut text = String::new();
                        for (index, cell) in row.cells.iter().enumerate() {
                            if index > 0 {
                                text.push('\t');
                            }
                            text.push_str(&blocks_text(&cell.blocks));
                        }
                        let style = if place == Where::Cell {
                            self.document.table_text_style()
                        } else {
                            self.body
                        };
                        out.push(self.document.new_paragraph(text, style));
                    }
                }
                Block::Table(table) if table.indent != 0 && !out.is_empty() => {
                    // A table at an offset: With para above, below the
                    // paragraph before it, as Ami Pro's tables placed so.
                    let frame = self.table_frame(table, place);
                    if let Some(made) = self.document.frame_mut(frame)
                        && let Placement::Inline { size, .. } = made.placement
                    {
                        made.placement = Placement::BelowParagraph {
                            x_offset: table.indent,
                            size,
                        };
                        made.style.wrap = Wrap::NoWrapAround;
                    }
                    if table.rows.first().is_some_and(|row| row.page_break_before)
                        && let Some(previous) = out.last_mut()
                    {
                        push_object(
                            previous,
                            InlineObject::Break {
                                kind: BreakKind::Page,
                                centre: false,
                                layout: None,
                                restart: false,
                            },
                        );
                    }
                    if let Some(previous) = out.last_mut() {
                        push_object(previous, InlineObject::FrameAnchor(frame));
                    }
                }
                Block::Table(table) => {
                    let frame = self.table_frame(table, place);
                    let mut holder = self.document.new_paragraph("", self.body);
                    if table.align != Alignment::Left {
                        holder.format.alignment = Some(ami_alignment(table.align));
                    }
                    if table.rows.first().is_some_and(|row| row.page_break_before)
                        && let Some(previous) = out.last_mut()
                    {
                        push_object(
                            previous,
                            InlineObject::Break {
                                kind: BreakKind::Page,
                                centre: false,
                                layout: None,
                                restart: false,
                            },
                        );
                    }
                    push_object(&mut holder, InlineObject::FrameAnchor(frame));
                    out.push(holder);
                }
            }
        }
        out
    }

    /// The model style an exchange paragraph style index stands for.
    fn style_for(&self, index: usize) -> StyleId {
        self.style_ids.get(index).copied().unwrap_or(self.body)
    }

    /// The exchange style's full formatting where its look is applied as
    /// formatting (Apply styles); none otherwise.
    fn applied(&self, index: usize) -> Option<&'a Style> {
        (self.options.styles == StyleOption::Apply)
            .then(|| self.exchange.styles.get(index))
            .flatten()
    }

    fn paragraph(&mut self, paragraph: &Paragraph, flow: FlowId, place: Where) -> AmiParagraph {
        let style_id = self.style_for(paragraph.style);
        let file_style = self.exchange.styles.get(paragraph.style);
        // The paragraph's formatting over its style, as the file has it.
        let mut props = file_style.map_or_else(ParagraphProps::default, |style| {
            if self.options.styles == StyleOption::Ignore {
                ParagraphProps::default()
            } else {
                style.paragraph.clone()
            }
        });
        overlay_paragraph(&mut props, &paragraph.props);
        // Paragraph formatting Ami Pro keeps on a paragraph.
        let style = self.document.styles()[&style_id].clone();
        let mut format = ParagraphFormat::default();
        let indents = ami_indents_over(&props, &style.indents);
        if indents != style.indents {
            format.indents = Some(indents);
        }
        if let Some(alignment) = props.alignment.map(ami_alignment)
            && alignment != style.alignment
        {
            format.alignment = Some(alignment);
        }
        let char_style = file_style.map(|style| &style.character);
        if let Some(line) = props.line {
            let (font, size, bold, italic) = char_style.map_or(
                (
                    style.font_family.clone(),
                    style.font_size_points,
                    style.bold,
                    style.italic,
                ),
                |character| {
                    (
                        character
                            .font
                            .clone()
                            .unwrap_or_else(|| style.font_family.clone()),
                        character
                            .size
                            .map_or(style.font_size_points, |size| size as f32 / 20.0),
                        character.bold.unwrap_or(style.bold),
                        character.italic.unwrap_or(style.italic),
                    )
                },
            );
            let spacing = self.line_spacing(Some(line), &font, size, bold, italic);
            if spacing != style.spacing.line {
                format.line_spacing = Some(spacing);
            }
        }
        if let Some(tabs) = &props.tabs {
            let tabs = ami_tabs(tabs);
            let in_effect = if style.breaks.0 & Breaks::OWN_TABS != 0 {
                style.tabs.clone()
            } else {
                ami_tabs(&default_tabs())
            };
            if tabs != in_effect && !(tabs.is_empty() && in_effect == ami_tabs(&default_tabs())) {
                format.tabs = Some(tabs);
            }
        }
        // What Ami Pro holds only in styles: a variant of the style.
        let style_id = if self.options.styles == StyleOption::Apply
            || self.options.styles == StyleOption::Ignore
        {
            if (props.space_before.unwrap_or(0) != style.spacing.above
                || props.space_after.unwrap_or(0) != style.spacing.below)
                && self.options.styles == StyleOption::Apply
            {
                self.findings.count(FindingKind::OwnSpacing);
            }
            style_id
        } else {
            self.variant(style_id, &props)
        };
        let mut made = self.document.new_paragraph("", style_id);
        made.format = format;
        // The text, its runs, and its objects.
        let run_base = self
            .applied(paragraph.style)
            .map(|style| style.character.clone());
        let mut runs = Runs::default();
        self.inlines(
            &paragraph.inlines,
            &mut made,
            &mut runs,
            run_base.as_ref(),
            flow,
            place,
        );
        made.runs = runs.finish(made.text.len());
        let mut mark = run_base.clone().unwrap_or_default();
        mark.overlay(&paragraph.mark);
        made.end_format = character_format(&mark, &self.document.styles()[&style_id]);
        if let Some((_, level)) = paragraph.list
            && file_style.is_none_or(|style| style.list.is_none())
        {
            // A list set on the paragraph: Ami Pro's list styles (IM5).
            let numbered = paragraph
                .list
                .and_then(|(instance, level)| self.list_format(instance, level))
                .is_none_or(|format| format != NumberFormat::Bullet);
            let kind = if numbered {
                amipro_editing::lists::ListKind::Numbering
            } else {
                amipro_editing::lists::ListKind::Bullets
            };
            self.lists.push(ListRequest {
                flow,
                paragraph: made.id,
                kind,
                depth: level + 1,
            });
        }
        made
    }

    /// The number format of an instance's level.
    fn list_format(&self, instance: Option<usize>, level: u8) -> Option<NumberFormat> {
        let instance = self.exchange.list_instances.get(instance?)?;
        let list = self.exchange.lists.get(instance.list)?;
        Some(list.levels.get(usize::from(level))?.format)
    }

    /// `style_id`, or a variant of it with the paragraph's own spacing,
    /// breaks, and rules where they differ from it.
    fn variant(&mut self, style_id: StyleId, props: &ParagraphProps) -> StyleId {
        let style = self.document.styles()[&style_id].clone();
        let above = props.space_before.unwrap_or(style.spacing.above).max(0);
        let below = props.space_after.unwrap_or(style.spacing.below).max(0);
        let keep_next = props
            .keep_with_next
            .unwrap_or(style.breaks.keep_with_next());
        let keep_together = props.keep_together.unwrap_or(!style.breaks.within());
        let rules = rules_of(
            props.border_top.unwrap_or(None),
            props.border_bottom.unwrap_or(None),
        );
        let rules_differ = (props.border_top.is_some() || props.border_bottom.is_some())
            && (rules.above.on != style.rules.above.on || rules.below.on != style.rules.below.on);
        if above == style.spacing.above
            && below == style.spacing.below
            && keep_next == style.breaks.keep_with_next()
            && keep_together == !style.breaks.within()
            && !rules_differ
        {
            return style_id;
        }
        let key = format!("{above} {below} {keep_next} {keep_together} {rules:?}");
        if let Some(id) = self.variants.get(&(style_id, key.clone())) {
            return *id;
        }
        let mut number = 2;
        let name = loop {
            let candidate = format!("{} {number}", style.name);
            if !self
                .document
                .styles()
                .values()
                .any(|known| known.name.eq_ignore_ascii_case(&candidate))
            {
                break candidate;
            }
            number += 1;
        };
        let id = self.next_style_id();
        let mut variant = style.clone();
        variant.id = id;
        variant.name = name.clone();
        variant.spacing.above = above;
        variant.spacing.below = below;
        let mut breaks = style.breaks.0 & !(Breaks::KEEP_WITH_NEXT | Breaks::WITHIN);
        if keep_next {
            breaks |= Breaks::KEEP_WITH_NEXT;
        }
        if !keep_together {
            breaks |= Breaks::WITHIN;
        }
        variant.breaks = Breaks(breaks);
        if rules_differ {
            variant.rules = rules;
        }
        variant.origin = StyleOrigin::Document;
        variant.kept.function_key = 0;
        variant.next = style.next.or(Some(style_id));
        self.document.styles_mut().insert(id, variant);
        self.variants.insert((style_id, key), id);
        self.findings.count(FindingKind::VariantStyle(name));
        id
    }

    /// Appends `inlines` to `made`.
    fn inlines(
        &mut self,
        inlines: &[Inline],
        made: &mut AmiParagraph,
        runs: &mut Runs,
        base: Option<&CharProps>,
        flow: FlowId,
        place: Where,
    ) {
        let style = self.document.styles()[&made.style].clone();
        let format_of = |props: &CharProps| {
            let mut full = base.cloned().unwrap_or_default();
            full.overlay(props);
            character_format(&full, &style)
        };
        for inline in inlines {
            match inline {
                Inline::Text { text, props } => {
                    let start = made.text.len();
                    made.text.push_str(&clean_text(text));
                    runs.push(start, made.text.len(), format_of(props));
                }
                Inline::Tab { props } => {
                    let start = made.text.len();
                    made.text.push('\t');
                    runs.push(start, made.text.len(), format_of(props));
                }
                Inline::LineBreak => made.text.push('\u{2028}'),
                Inline::PageBreak | Inline::ColumnBreak if place != Where::Main => {}
                Inline::PageBreak | Inline::ColumnBreak => {
                    let kind = if matches!(inline, Inline::PageBreak) {
                        BreakKind::Page
                    } else {
                        BreakKind::Column
                    };
                    push_object(
                        made,
                        InlineObject::Break {
                            kind,
                            centre: false,
                            layout: None,
                            restart: false,
                        },
                    );
                }
                Inline::Field(field) => self.field(field, made, runs, &format_of(&field.props)),
                Inline::Symbol {
                    character,
                    font,
                    props,
                } => match bullet_code(*character) {
                    Some(code) => {
                        let start = made.text.len();
                        push_object(made, InlineObject::Bullet(code));
                        runs.push(start, made.text.len(), format_of(props));
                    }
                    None => {
                        let mut props = props.clone();
                        if font.is_some() {
                            props.font.clone_from(font);
                        }
                        let start = made.text.len();
                        made.text.push(*character);
                        runs.push(start, made.text.len(), format_of(&props));
                    }
                },
                Inline::Shape(shape) if place != Where::Main => {
                    // Ami Pro anchors frames in the main text only.
                    let kind = match place {
                        Where::Cell => "in a table cell",
                        _ => "in a frame or a header",
                    };
                    self.findings.count(FindingKind::Shape(kind.into()));
                    if let ShapeContent::Text { blocks, .. } = &shape.content {
                        let text = blocks_text(blocks);
                        if !text.trim().is_empty() {
                            let start = made.text.len();
                            made.text.push_str(&text);
                            runs.push(start, made.text.len(), CharacterFormat::default());
                        }
                    }
                }
                Inline::Shape(shape) => {
                    if let Some(frame) = self.shape_frame(shape, made.id, flow) {
                        push_object(made, InlineObject::FrameAnchor(frame));
                    }
                }
                // A comment is Ami Pro's note, its text a flow of its own
                // (the notes run).
                Inline::Note(note) => {
                    let flow = self.flow_from_blocks(&note.blocks, Where::Other);
                    let number = if place == Where::Main {
                        self.notes_made += 1;
                        self.notes_made
                    } else {
                        0
                    };
                    push_object(
                        made,
                        InlineObject::Note {
                            note: amipro_document::NoteInfo {
                                time: note.date.map_or(self.options.now, |date| date.to_ami_pro()),
                                initials: Some(note_initials(note)),
                                colour: Some(amipro_document::NoteInfo::YELLOW),
                                number: Some(number),
                            },
                            flow,
                        },
                    );
                }
                // In the main text a footnote is Ami Pro's footnote, its text
                // in the Footnote style; elsewhere it goes to the end (IM6).
                Inline::Footnote(footnote) if place == Where::Main => {
                    let style = self.footnote_style();
                    let flow = self.flow_from_blocks(&footnote.blocks, Where::Other);
                    if let Some(paragraphs) = self.document.flow(flow).map(|flow| {
                        flow.paragraphs
                            .iter()
                            .map(|paragraph| {
                                Arc::new(AmiParagraph {
                                    style,
                                    ..(**paragraph).clone()
                                })
                            })
                            .collect::<Vec<_>>()
                    }) {
                        let _ = self.document.replace_flow_paragraphs(flow, paragraphs);
                    }
                    if footnote.endnote {
                        self.typed_footnotes.1 += 1;
                    } else {
                        self.typed_footnotes.0 += 1;
                    }
                    push_object(made, InlineObject::Footnote { flow });
                }
                Inline::Bookmark { name, end } => {
                    let mark = amipro_document::bookmarks::BookmarkMark {
                        name: name
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .take(19)
                            .collect(),
                        end: *end,
                        cells: None,
                    };
                    if !mark.name.is_empty() {
                        made.marks.push(mark.mark(made.text.len()));
                    }
                }
                Inline::Footnote(footnote) => {
                    self.footnotes
                        .push((footnote.endnote, footnote.blocks.clone()));
                    let number = self
                        .footnotes
                        .iter()
                        .filter(|(endnote, _)| *endnote == footnote.endnote)
                        .count();
                    let mark = if footnote.endnote {
                        crate::numbering::format_number(NumberFormat::LowerRoman, number as u32)
                    } else {
                        number.to_string()
                    };
                    let mut props = footnote.props.clone();
                    props.script = Some(Script::Super);
                    let start = made.text.len();
                    made.text.push_str(&mark);
                    runs.push(start, made.text.len(), format_of(&props));
                }
                Inline::Marker(id) => {
                    self.markers.insert(
                        *id,
                        Place {
                            flow,
                            paragraph: made.id,
                            byte: made.text.len(),
                        },
                    );
                }
            }
        }
    }

    // ---- Fields ----

    fn field(
        &mut self,
        field: &Field,
        made: &mut AmiParagraph,
        runs: &mut Runs,
        format: &CharacterFormat,
    ) {
        let doc_info = |entry: DocInfoEntry, format: u8| {
            InlineObject::DocInfo(DocInfoField {
                code: entry.code(),
                format,
            })
        };
        let object = match &field.kind {
            FieldKind::Page { format } => Some(InlineObject::PageNumber(PageNumberField {
                style: match format {
                    NumberFormat::UpperRoman => 'I',
                    NumberFormat::LowerRoman => 'i',
                    NumberFormat::UpperLetter => 'A',
                    NumberFormat::LowerLetter => 'a',
                    _ => '1',
                },
                // Empty on the first page: Ami Pro's numbering starts on a
                // later one (the export's result is page 1's).
                start_page: if field.result.is_empty() { 2 } else { 1 },
                start_number: 0,
                leading_text: String::new(),
            })),
            FieldKind::Date {
                source,
                picture,
                upper,
            } => {
                let letter = date_letter(picture, *upper);
                Some(InlineObject::Date(DateField {
                    source: match source {
                        DateSource::Now => AmiDateSource::Today,
                        DateSource::Created => AmiDateSource::Created,
                        DateSource::Saved => AmiDateSource::LastSaved,
                    },
                    format: letter,
                }))
            }
            FieldKind::Time { source } => Some(match source {
                DateSource::Now => InlineObject::Date(DateField {
                    source: AmiDateSource::Time,
                    format: b'a',
                }),
                DateSource::Created => doc_info(DocInfoEntry::TimeCreated, b'a'),
                DateSource::Saved => doc_info(DocInfoEntry::TimeLastRevised, b'a'),
            }),
            FieldKind::Description => Some(doc_info(DocInfoEntry::Description, b'a')),
            FieldKind::Custom(name) => self
                .custom_field(name)
                .map(|index| doc_info(DocInfoEntry::Field(index), b'a')),
            FieldKind::FileName { .. } => Some(doc_info(DocInfoEntry::FileName, b'a')),
            FieldKind::Template => Some(doc_info(DocInfoEntry::StyleSheet, b'a')),
            FieldKind::Revisions => Some(doc_info(DocInfoEntry::TotalRevisions, b'a')),
            FieldKind::EditingTime => Some(doc_info(DocInfoEntry::EditingTime, b'a')),
            FieldKind::NumPages => Some(doc_info(DocInfoEntry::Pages, b'a')),
            FieldKind::Words => Some(doc_info(DocInfoEntry::Words, b'a')),
            FieldKind::Characters => Some(doc_info(DocInfoEntry::Characters, b'a')),
            FieldKind::Seq { name } => Some(InlineObject::Seq(SeqField {
                code: name.clone(),
                value: field.result.trim().parse::<u16>().unwrap_or(1).min(0x3FFF),
            })),
            FieldKind::Keywords => {
                self.power_field("Keywords", &field.result, made, runs, format);
                None
            }
            FieldKind::IndexEntry { primary, secondary } => {
                let quote = |text: &str| text.replace('"', "'");
                let code = if secondary.is_empty() {
                    format!("Index \"{}\"", quote(primary))
                } else {
                    format!("Index \"{}\" # \"{}\"", quote(primary), quote(secondary))
                };
                made.marks.push(LegacyMark {
                    byte: made.text.len(),
                    raw: PowerMark {
                        end: false,
                        kind: power_kind::INDEX,
                        value: amipro_document::power_flag::MAIN_TEXT,
                        code,
                    }
                    .raw(),
                });
                None
            }
            FieldKind::ContentsEntry { text, level } => {
                made.marks.push(LegacyMark {
                    byte: made.text.len(),
                    raw: PowerMark {
                        end: false,
                        kind: power_kind::TOC,
                        value: amipro_document::power_flag::MAIN_TEXT,
                        code: format!("TOC {} \"{}\"", level.clamp(&1, &9), text.replace('"', "'")),
                    }
                    .raw(),
                });
                None
            }
            FieldKind::Title => {
                let start = made.text.len();
                made.text.push_str(&clean_text(&field.result));
                runs.push(start, made.text.len(), format.clone());
                None
            }
        };
        if let Some(object) = object {
            let start = made.text.len();
            push_object(made, object);
            runs.push(start, made.text.len(), format.clone());
        }
    }

    /// A power field with its result between its marks.
    fn power_field(
        &mut self,
        code: &str,
        result: &str,
        made: &mut AmiParagraph,
        runs: &mut Runs,
        format: &CharacterFormat,
    ) {
        let value = amipro_document::power_flag::MAIN_TEXT;
        let mark = |end| PowerMark {
            end,
            kind: power_kind::GENERAL,
            value,
            code: code.to_owned(),
        };
        made.marks.push(LegacyMark {
            byte: made.text.len(),
            raw: mark(false).raw(),
        });
        let start = made.text.len();
        made.text.push_str(&clean_text(result));
        runs.push(start, made.text.len(), format.clone());
        made.marks.push(LegacyMark {
            byte: made.text.len(),
            raw: mark(true).raw(),
        });
    }

    /// The Doc Info field index named `name`, naming a free one if needed.
    fn custom_field(&mut self, name: &str) -> Option<u8> {
        let info = self.document.info_mut();
        let found = (0..8).find(|index| info.field_name(*index).eq_ignore_ascii_case(name));
        if let Some(index) = found {
            return Some(index as u8);
        }
        let free = (0..8).find(|index| {
            info.field_names[*index].trim().is_empty() && info.fields[*index].is_empty()
        })?;
        info.field_names[free] = name.to_owned();
        Some(free as u8)
    }

    // ---- Tables ----

    /// A table as an Ami Pro page table: a frame in the line.
    fn table_frame(&mut self, table: &Table, place: Where) -> FrameId {
        let _ = place;
        let [top, right, bottom, left] = table.cell_margins;
        let horizontal = (right + left).max(0);
        let vertical = (bottom + top).max(0);
        let columns: Vec<Twips> = table
            .columns
            .iter()
            .map(|width| (width - horizontal).max(20))
            .collect();
        let count = columns.len().max(1);
        let rows_count = table.rows.len().max(1);
        let mut heights = vec![0; rows_count];
        let mut row_options = TableRows {
            automatic: true,
            ..TableRows::default()
        };
        for (index, row) in table.rows.iter().enumerate() {
            let (height, exact) = match row.height {
                crate::RowHeight::AtLeast(height) => (height, false),
                crate::RowHeight::Exact(height) => (height, true),
            };
            heights[index] = (height - vertical).max(0);
            if exact {
                row_options.automatic = false;
            }
            if row.heading {
                row_options.headings.insert(index);
            }
            if row.page_break_before && index > 0 {
                row_options.page_breaks.insert(index);
            }
            if !row.keep_together {
                row_options.span_pages = true;
            }
        }
        if !row_options.automatic {
            row_options.span_pages = false;
        }
        let row_automatic = row_options.automatic;
        // The grid: each row's cells at their columns, joined blocks over
        // the cells they cover.
        let mut grid: Vec<Option<(CellSpan, Option<&crate::Cell>)>> =
            vec![None; rows_count * count];
        for (row_index, row) in table.rows.iter().enumerate() {
            let mut column = 0;
            for cell in &row.cells {
                while column < count && grid[row_index * count + column].is_some() {
                    column += 1;
                }
                if column >= count {
                    break;
                }
                let span = usize::from(cell.span.max(1)).min(count - column);
                match cell.merge {
                    VerticalMerge::Continue => {
                        for covered in column..column + span {
                            grid[row_index * count + covered] = Some((CellSpan::COVERED, None));
                        }
                        // The block above grows by a row.
                        let mut above = row_index;
                        while above > 0 {
                            above -= 1;
                            if let Some((owner, Some(_))) = &mut grid[above * count + column] {
                                owner.rows += 1;
                                break;
                            }
                        }
                    }
                    _ => {
                        grid[row_index * count + column] = Some((
                            CellSpan {
                                rows: 1,
                                columns: span as u16,
                            },
                            Some(cell),
                        ));
                        for covered in column + 1..column + span {
                            grid[row_index * count + covered] = Some((CellSpan::COVERED, None));
                        }
                    }
                }
                column += span;
            }
        }
        let table_text = self.document.table_text_style();
        let mut cells = Vec::with_capacity(grid.len());
        let mut cell_frames = Vec::new();
        for (index, slot) in grid.into_iter().enumerate() {
            let (span, cell) = slot.unwrap_or((CellSpan::SINGLE, None));
            let legacy = LegacyData::default();
            let mut format = CellFormat::default();
            let mut frame = None;
            let flow = match cell {
                Some(cell) => {
                    // A frame at a cell's start is the cell's frame.
                    let (blocks, picture) = take_cell_picture(&cell.blocks);
                    if let Some(shape) = picture {
                        frame = self.cell_frame(&shape);
                        if let Some(frame) = frame {
                            cell_frames.push(frame);
                            // The picture fills its cell (T2): its row is at
                            // least the picture's height.
                            let row = index / count;
                            if span.rows <= 1 && row < heights.len() {
                                heights[row] = heights[row].max(shape.height.max(0));
                            }
                        }
                    }
                    format.lines = CellLines::from_word(cell_line_word(&cell.borders));
                    format.fill = cell.shading;
                    // A coloured line keeps its colour (TD8: the app's own).
                    if let Some(line) = cell.borders.iter().flatten().next() {
                        format.line_colour = line.colour;
                    }
                    self.flow_from_blocks(&blocks, Where::Cell)
                }
                None => self.document.create_flow(table_text),
            };
            let _ = index;
            cells.push(TableCell {
                content: CellContent::Text(flow),
                span,
                frame,
                format,
                formula: None,
                legacy,
            });
        }
        let mut made = AmiTable {
            columns,
            rows: heights,
            cell_margins: CellMargins {
                horizontal,
                vertical,
            },
            spacing: TableSpacing {
                top: top.max(0),
                left: left.max(0),
                ..TableSpacing::default()
            },
            row_options,
            layout: TableLayout::default(),
            read_rows: BTreeMap::new(),
            cells,
            legacy: LegacyData::default(),
        };
        made.layout = TableLayout::inferred(&made.rows, &made.columns);
        if made.validate_spans().is_err() {
            for cell in &mut made.cells {
                cell.span = CellSpan::SINGLE;
            }
        }
        let width: Twips = made.spacing.left
            + made
                .columns
                .iter()
                .map(|width| width + horizontal)
                .sum::<Twips>();
        let height: Twips = made.spacing.top
            + made
                .rows
                .iter()
                .map(|height| height + vertical)
                .sum::<Twips>();
        let id = self.document.allocate_frame_id();
        self.document.insert_frame(Frame {
            id,
            placement: Placement::Inline {
                y_offset: 0,
                size: Size {
                    width: width.max(20),
                    height: height.max(20),
                },
            },
            style: FrameStyle {
                wrap: Wrap::Around,
                opaque: false,
                border: Border {
                    sides: Sides::NONE,
                    ..FrameStyle::default().border
                },
                interior: LayoutBox::single_column(Margins::uniform(0)),
                ..FrameStyle::default()
            },
            group: None,
            content: FrameContent::Table(made),
            legacy: LegacyData::default(),
        });
        self.stacked.push((id, 1));
        for frame in cell_frames {
            self.stacked.push((frame, 1));
        }
        if row_automatic {
            self.automatic_tables.push(id);
        }
        id
    }

    /// Ami Pro measures every Automatic row as it opens a file, growing and
    /// shrinking it to its text (VM RC1–RC3), so a row cannot keep a
    /// minimum height taller than its text. A row whose minimum is more
    /// than [`ROW_SHRINK`] taller than its text takes its text's height, as
    /// Ami Pro shows it; a row of empty cells keeps its height, as Ami Pro
    /// keeps it. Within the tolerance the minimum stays (a row Ami Pro
    /// wrote is its own measure with its printer's fonts).
    fn measure_rows(&mut self, shaper: &mut dyn Shaper) {
        let tables = std::mem::take(&mut self.automatic_tables);
        if self.exchange.settings.ami_pro_rows {
            return;
        }
        let mut minimums: HashMap<FrameId, Vec<Twips>> = HashMap::new();
        for id in &tables {
            let document = &self.document;
            let Some(FrameContent::Table(table)) = document.frame(*id).map(|frame| &frame.content)
            else {
                continue;
            };
            let count = table.columns.len().max(1);
            // Rows with text and no picture: a cell's picture fills its
            // cell, so its row keeps its height.
            let texted: Vec<bool> = (0..table.rows.len())
                .map(|row| {
                    let cells =
                        &table.cells[row * count..((row + 1) * count).min(table.cells.len())];
                    cells.iter().all(|cell| cell.frame.is_none())
                        && cells.iter().any(|cell| {
                            let CellContent::Text(flow) = cell.content;
                            document.flow(flow).is_some_and(|flow| {
                                flow.paragraphs
                                    .iter()
                                    .any(|paragraph| !paragraph.text.is_empty())
                            })
                        })
                })
                .collect();
            if !texted.iter().any(|texted| *texted) {
                continue;
            }
            let rows = table.rows.clone();
            if let Some(Frame {
                content: FrameContent::Table(table),
                ..
            }) = self.document.frame_mut(*id)
            {
                for (row, texted) in texted.iter().enumerate() {
                    if *texted {
                        table.rows[row] = 0;
                    }
                }
            }
            minimums.insert(*id, rows);
        }
        if minimums.is_empty() {
            return;
        }
        let mut cache = ShapeCache::default();
        let layout = layout_document(&self.document, shaper, &mut cache);
        let mut measured: HashMap<FrameId, BTreeMap<usize, Twips>> = HashMap::new();
        for page in &layout.pages {
            for frame_box in &page.frames {
                let FrameBoxKind::Table { part, cells } = &frame_box.kind else {
                    continue;
                };
                if !minimums.contains_key(&frame_box.frame) {
                    continue;
                }
                let rows = measured.entry(frame_box.frame).or_default();
                // A table laid out whole (a frame's) gives its cells' boxes:
                // a row's text is its single-row cells' boxes less the
                // margins above and below.
                if part.rows.is_empty() {
                    let Some(FrameContent::Table(table)) = self
                        .document
                        .frame(frame_box.frame)
                        .map(|frame| &frame.content)
                    else {
                        continue;
                    };
                    let count = table.columns.len().max(1);
                    for (index, (cell, bounds)) in table.cells.iter().zip(cells).enumerate() {
                        if cell.span.is_covered() || cell.span.rows > 1 {
                            continue;
                        }
                        let height =
                            (bounds.height.round() as Twips - table.cell_margins.vertical).max(0);
                        let row = rows.entry(index / count).or_insert(0);
                        *row = (*row).max(height);
                    }
                    continue;
                }
                let last = part.rows.end.saturating_sub(1);
                for (row, height) in part.rows.clone().zip(&part.heights) {
                    // A row split between parts shows only some of its text.
                    let split = (row == part.rows.start && !part.shown_before.is_empty())
                        || (row == last && part.cut);
                    if !split {
                        rows.entry(row).or_insert(height.round() as Twips);
                    }
                }
            }
        }
        for (id, minimum) in minimums {
            let measured = measured.remove(&id).unwrap_or_default();
            let Some(frame) = self.document.frame_mut(id) else {
                continue;
            };
            let FrameContent::Table(table) = &mut frame.content else {
                continue;
            };
            for (row, height) in table.rows.iter_mut().enumerate() {
                if *height != 0 || minimum[row] == 0 {
                    *height = minimum[row];
                    continue;
                }
                *height = match measured.get(&row) {
                    Some(text) if minimum[row] - text > ROW_SHRINK => *text,
                    _ => minimum[row],
                };
            }
            let table_height: Twips = table.spacing.top
                + table
                    .rows
                    .iter()
                    .map(|height| height + table.cell_margins.vertical)
                    .sum::<Twips>();
            if let Placement::Inline { size, .. } = &mut frame.placement {
                size.height = table_height.max(20);
            }
        }
    }

    /// The frame a cell holds.
    fn cell_frame(&mut self, shape: &Shape) -> Option<FrameId> {
        self.frame(shape, Placement::InCell)
    }

    // ---- Frames ----

    /// A shape as a frame anchored in paragraph `paragraph` of `flow`.
    fn shape_frame(
        &mut self,
        shape: &Shape,
        paragraph: ParagraphId,
        flow: FlowId,
    ) -> Option<FrameId> {
        let size = Size {
            width: shape.width.max(20),
            height: shape.height.max(20),
        };
        let mut below_top = None;
        let placement = match shape.anchor {
            Anchor::Inline { drop } => Placement::Inline {
                y_offset: drop,
                size,
            },
            Anchor::Floating {
                horizontal,
                vertical,
                wrap,
                ..
            } => {
                let margins = &self.page;
                let left = match horizontal.0 {
                    HorizontalFrom::Page => horizontal.1,
                    HorizontalFrom::Margin | HorizontalFrom::Column => {
                        horizontal.1 + margins.margin_left
                    }
                };
                match vertical.0 {
                    // Text beside it: Ami Pro's With para above frames have
                    // none, so it stays where it is on its page.
                    VerticalFrom::Paragraph if wrap == ShapeWrap::Square => {
                        below_top = Some(vertical.1);
                        Placement::Fixed {
                            page: 1,
                            rect: Rect::new(left, 0, left + size.width, size.height),
                        }
                    }
                    VerticalFrom::Paragraph => Placement::BelowParagraph {
                        x_offset: left - margins.margin_left,
                        size,
                    },
                    VerticalFrom::Page | VerticalFrom::Margin => {
                        let top = match vertical.0 {
                            VerticalFrom::Margin => vertical.1 + margins.margin_top,
                            _ => vertical.1,
                        };
                        Placement::Fixed {
                            page: 1,
                            rect: Rect::new(left, top, left + size.width, top + size.height),
                        }
                    }
                }
            }
        };
        let in_main = flow == self.document.main_flow_id();
        // Page frames belong to the main text; elsewhere they go in the line.
        let placement = match placement {
            Placement::Fixed { .. } | Placement::BelowParagraph { .. } if !in_main => {
                Placement::Inline { y_offset: 0, size }
            }
            other => other,
        };
        let id = self.frame(shape, placement)?;
        if matches!(placement, Placement::Fixed { .. }) {
            self.page_frames.push((id, paragraph, below_top));
        }
        self.stacked.push((id, shape.z));
        Some(id)
    }

    /// A shape in a header placed on the page: a repeating frame.
    fn repeating_frame(&mut self, shape: &Shape) -> Option<FrameId> {
        let Anchor::Floating {
            horizontal,
            vertical,
            ..
        } = shape.anchor
        else {
            return None;
        };
        let margins = &self.page;
        let left = match horizontal.0 {
            HorizontalFrom::Page => horizontal.1,
            _ => horizontal.1 + margins.margin_left,
        };
        let top = match vertical.0 {
            VerticalFrom::Page => vertical.1,
            VerticalFrom::Margin => vertical.1 + margins.margin_top,
            VerticalFrom::Paragraph => vertical.1 + margins.header_distance,
        };
        let rect = Rect::new(
            left,
            top,
            left + shape.width.max(20),
            top + shape.height.max(20),
        );
        self.frame(
            shape,
            Placement::Repeating {
                pages: RepeatPages::All,
                rect,
            },
        )
    }

    /// A frame for `shape` placed by `placement`.
    fn frame(&mut self, shape: &Shape, placement: Placement) -> Option<FrameId> {
        let (content, insets) = match &shape.content {
            ShapeContent::Text {
                blocks,
                insets,
                columns,
                column_gap,
            } => {
                let margins = Margins {
                    top: insets[0],
                    right: insets[1],
                    bottom: insets[2],
                    left: insets[3],
                };
                // A box holding only a table is a table frame.
                let only_table = blocks
                    .iter()
                    .filter(|block| !matches!(block, Block::Paragraph(paragraph) if paragraph.inlines.is_empty()))
                    .collect::<Vec<_>>();
                if let [Block::Table(table)] = only_table.as_slice() {
                    let frame = self.table_frame(table, Where::Other);
                    let style = self.frame_style(shape, margins);
                    if let Some(made) = self.document.frame_mut(frame) {
                        made.placement = placement;
                        made.style = style;
                    }
                    self.stacked.retain(|(id, _)| *id != frame);
                    self.findings.count(FindingKind::FloatingTable);
                    return Some(frame);
                }
                let flow = self.flow_from_blocks(blocks, Where::Other);
                let _ = (columns, column_gap);
                (FrameContent::Text(flow), margins)
            }
            ShapeContent::Picture(placed) => {
                let insets = picture_margins(placed, shape);
                match self.picture(placed, shape, insets) {
                    Some(picture) => (FrameContent::Picture(picture), insets),
                    None => {
                        let flow = self.document.create_flow(self.body);
                        (FrameContent::Text(flow), insets)
                    }
                }
            }
            ShapeContent::Empty => {
                let flow = self.document.create_flow(self.body);
                (FrameContent::Text(flow), Margins::uniform(0))
            }
        };
        let mut style = self.frame_style(shape, insets);
        if let ShapeContent::Text {
            columns,
            column_gap,
            ..
        } = &shape.content
        {
            style.interior.columns = (*columns).clamp(1, 8);
            if *columns > 1 {
                style.interior.gutter = *column_gap;
            }
            columns_tabs(&mut style.interior);
        }
        let id = self.document.allocate_frame_id();
        self.document.insert_frame(Frame {
            id,
            placement,
            style,
            group: None,
            content,
            legacy: LegacyData::default(),
        });
        Some(id)
    }

    /// A shape's look as a frame style with `margins` inside.
    fn frame_style(&self, shape: &Shape, margins: Margins) -> FrameStyle {
        let (wrap, behind) = match shape.anchor {
            Anchor::Inline { .. } => (Wrap::Around, false),
            Anchor::Floating { wrap, behind, .. } => (
                match wrap {
                    ShapeWrap::Square => Wrap::Around,
                    ShapeWrap::TopAndBottom => Wrap::NoWrapBeside,
                    ShapeWrap::None => Wrap::NoWrapAround,
                },
                behind,
            ),
        };
        let border = match shape.line {
            Some(line) => Border {
                sides: Sides {
                    top: line.sides[0],
                    right: line.sides[1],
                    bottom: line.sides[2],
                    left: line.sides[3],
                },
                style: line_style(line.kind, line.width),
                colour: line.colour,
                position: LinePosition::Middle,
            },
            None => Border {
                sides: Sides::NONE,
                ..FrameStyle::default().border
            },
        };
        let shadow = shape.shadow.map_or_else(Shadow::default, |shadow| Shadow {
            left: (-shadow.dx).max(0),
            top: (-shadow.dy).max(0),
            right: shadow.dx.max(0),
            bottom: shadow.dy.max(0),
            colour: shadow.colour,
        });
        let shorter = shape.width.min(shape.height).max(1);
        let corners_of = |shorter: i64| {
            ((i64::from(shape.corner_radius) * 200 + shorter - 1) / shorter).clamp(0, 100) as u8
        };
        // The export's radius is the percentage of half the shorter side,
        // truncated: rounding up gives the percentage back.
        let corners = corners_of(i64::from(shorter));
        // Over text that runs through it, not behind: opaque (the export
        // writes a transparent frame there behind the text); else by its
        // fill.
        let opaque = if wrap == Wrap::NoWrapAround {
            !behind
        } else {
            shape.fill.is_some()
        };
        let mut style = FrameStyle {
            wrap,
            opaque,
            border,
            shadow,
            corners,
            background: shape.fill.unwrap_or(amipro_document::WHITE),
            interior: LayoutBox::single_column(margins),
            name: (!shape.name.is_empty() && !shape.name.starts_with("Frame "))
                .then(|| shape.name.clone()),
            macro_name: None,
        };
        // The export's insets add the rounded corners' to the margins.
        if style.corners > 0 {
            let shown = style.text_insets(shape.width, shape.height);
            let less = |inset: Twips, shown: Twips| (2 * inset - shown).max(0);
            style.interior.margins = Margins {
                top: less(margins.top, shown.top),
                right: less(margins.right, shown.right),
                bottom: less(margins.bottom, shown.bottom),
                left: less(margins.left, shown.left),
            };
        }
        style
    }

    /// A placed picture as Ami Pro's: its bytes, and its size and place in
    /// the frame as Custom scaling and a pan.
    fn picture(
        &mut self,
        placed: &crate::PlacedPicture,
        shape: &Shape,
        insets: Margins,
    ) -> Option<Picture> {
        let object = (*self.images.get(placed.image)?)?;
        let image = self.exchange.images.get(placed.image)?;
        let mut picture = Picture::new(
            PictureSource::Embedded(object),
            extension(image),
            Size {
                width: placed.width.max(1),
                height: placed.height.max(1),
            },
        );
        let area = Size {
            width: (shape.width - insets.left - insets.right).max(1),
            height: (shape.height - insets.top - insets.bottom).max(1),
        };
        let fills = placed.left == insets.left
            && placed.top == insets.top
            && placed.width == area.width
            && placed.height == area.height;
        if fills {
            picture.scaling = PictureScaling {
                sizing: PictureSizing::FitInFrame,
                keep_aspect: false,
                percent: 100,
                size: area,
                rotation: 0,
            };
        } else {
            picture.scaling = PictureScaling {
                sizing: PictureSizing::Custom,
                keep_aspect: false,
                percent: 100,
                size: Size {
                    width: placed.width.max(1),
                    height: placed.height.max(1),
                },
                rotation: 0,
            };
            picture.pan = amipro_document::Offset {
                x: placed.left - insets.left,
                y: placed.top - insets.top,
            };
        }
        picture.description.clone_from(&placed.description);
        Some(picture)
    }

    /// Puts each Where placed frame on the page its anchor's paragraph is
    /// laid out on.
    fn settle_page_frames(&mut self, shaper: &mut dyn Shaper) {
        if self.page_frames.is_empty() {
            return;
        }
        // Their anchors were only for finding the page.
        remove_fixed_anchors(&mut self.document);
        let fixed: Vec<FrameId> = self.page_frames.iter().map(|(frame, ..)| *frame).collect();
        let stack = self.document.frame_stack().to_vec();
        let main = self.document.main_flow_id();
        let mut cache = ShapeCache::default();
        // First without them (their pages unknown, they would push text on
        // page 1), then with them where the first pass put them.
        for with_frames in [false, true] {
            *self.document.frame_stack_mut() = if with_frames {
                stack.clone()
            } else {
                stack
                    .iter()
                    .copied()
                    .filter(|id| !fixed.contains(id))
                    .collect()
            };
            let layout = layout_document(&self.document, shaper, &mut cache);
            let mut pages: HashMap<ParagraphId, (u32, Twips)> = HashMap::new();
            for page in &layout.pages {
                for line in &page.lines {
                    if line.container == Container::Main && line.flow == main {
                        pages
                            .entry(line.paragraph)
                            .or_insert((page.number, line.top.round() as Twips));
                    }
                }
            }
            for (frame, paragraph, below) in &self.page_frames {
                let (page, top) = pages.get(paragraph).copied().unwrap_or((1, 0));
                if let Some(frame) = self.document.frame_mut(*frame)
                    && let Placement::Fixed { page: slot, rect } = &mut frame.placement
                {
                    *slot = page.max(1);
                    if let Some(offset) = below {
                        let height = rect.height();
                        rect.top = (top + offset).max(0);
                        rect.bottom = rect.top + height;
                    }
                }
            }
        }
        *self.document.frame_stack_mut() = stack;
        self.page_frames.clear();
    }

    // ---- Notes, lists, properties ----

    /// The Footnote style, made from Body Text as Ami Pro makes it on the
    /// first footnote (First indent 720, a superscript arabic number).
    fn footnote_style(&mut self) -> StyleId {
        if let Some(style) = self
            .document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Footnote"))
        {
            return style.id;
        }
        let mut style = self.document.styles()[&self.body].clone();
        let id = StyleId(
            self.document
                .styles()
                .keys()
                .next_back()
                .map_or(1, |id| id.0 + 1),
        );
        style.id = id;
        style.name = "Footnote".into();
        style.indents.first = 720;
        style.bullets = amipro_document::Bullets {
            text: "\u{1A}".into(),
            attributes: 256,
            ..amipro_document::Bullets::default()
        };
        style.kept.function_key = 0;
        self.document.styles_mut().insert(id, style);
        id
    }

    /// Footnotes and endnotes as numbered paragraphs at the document's end
    /// (IM6).
    fn endnotes(&mut self) {
        // Every footnote an endnote: Ami Pro's Make endnotes.
        if self.typed_footnotes.1 > 0 && self.typed_footnotes.0 == 0 {
            let mut options = self.document.footnote_options().clone();
            options.set(amipro_document::FootnoteOptions::ENDNOTES, true);
            self.document.set_footnote_options(options);
        }
        let notes = std::mem::take(&mut self.footnotes);
        if notes.is_empty() {
            return;
        }
        let main = self.document.main_flow_id();
        let mut paragraphs: Vec<Arc<AmiParagraph>> = self.document.main_flow().paragraphs.clone();
        let (mut footnotes, mut endnotes) = (0u32, 0u32);
        for (endnote, blocks) in &notes {
            let number = if *endnote {
                endnotes += 1;
                crate::numbering::format_number(NumberFormat::LowerRoman, endnotes)
            } else {
                footnotes += 1;
                footnotes.to_string()
            };
            let place = self.main_place();
            let mut made = self.paragraphs_of(blocks, main, place);
            if made.is_empty() {
                made.push(self.document.new_paragraph("", self.body));
            }
            let first = &mut made[0];
            let prefix = format!("{number}\t");
            shift_paragraph(first, &prefix);
            let style = self.document.styles()[&first.style].clone();
            first.runs.insert(
                0,
                CharacterRun {
                    start: 0,
                    end: number.len(),
                    format: CharacterFormat {
                        superscript: Some(true),
                        ..CharacterFormat::default()
                    },
                },
            );
            let _ = style;
            paragraphs.extend(made.into_iter().map(Arc::new));
        }
        self.findings.add(FindingKind::Footnotes, footnotes, None);
        self.findings.add(FindingKind::Endnotes, endnotes, None);
        let _ = self.document.replace_flow_paragraphs(main, paragraphs);
    }

    /// Lists set on paragraphs: Ami Pro's list styles through its commands
    /// (Format › Bullets and Numbering, Promote and Demote), as the app's
    /// own make them (IM5).
    fn apply_lists(&mut self) {
        use amipro_editing::{Command, apply, outline::LevelChange};
        // The paragraphs' places, then runs of neighbours with the same
        // list and depth: one command a run.
        let mut places: Vec<(FlowId, usize, amipro_editing::lists::ListKind, u8)> = Vec::new();
        for request in std::mem::take(&mut self.lists) {
            let Some(index) = self.document.flow(request.flow).and_then(|flow| {
                flow.paragraphs
                    .iter()
                    .position(|paragraph| paragraph.id == request.paragraph)
            }) else {
                continue;
            };
            places.push((request.flow, index, request.kind, request.depth.clamp(1, 9)));
        }
        places.sort_by_key(|(flow, index, ..)| (*flow, *index));
        let mut runs: Vec<(FlowId, usize, usize, amipro_editing::lists::ListKind, u8)> = Vec::new();
        for (flow, index, kind, depth) in places {
            match runs.last_mut() {
                Some(run)
                    if run.0 == flow && run.2 + 1 == index && run.3 == kind && run.4 == depth =>
                {
                    run.2 = index;
                }
                _ => runs.push((flow, index, index, kind, depth)),
            }
        }
        // The style each kind and depth of list takes, made by the first
        // run that needs it (the commands validate the whole document, so
        // later runs take the style made).
        let mut made: Vec<((amipro_editing::lists::ListKind, u8), StyleId)> = Vec::new();
        for (flow, first, last, kind, depth) in runs {
            if let Some((_, style)) = made.iter().find(|(key, _)| *key == (kind, depth)) {
                if let Some(flow) = self.document.flow_mut(flow) {
                    for paragraph in flow.paragraphs.get_mut(first..=last).into_iter().flatten() {
                        Arc::make_mut(paragraph).style = *style;
                    }
                }
                continue;
            }
            // Each paragraph keeps its own alignment.
            let alignments: Vec<Option<AmiAlignment>> = self
                .document
                .flow(flow)
                .map(|flow| {
                    flow.paragraphs[first..=last]
                        .iter()
                        .map(|paragraph| paragraph.format.alignment)
                        .collect()
                })
                .unwrap_or_default();
            let set = Command::SetList {
                flow,
                first,
                last,
                list: Some(kind),
            };
            if let Ok(applied) = apply(&self.document, &set) {
                self.document = applied.document;
            }
            for _ in 1..depth {
                let demote = Command::ChangeLevel {
                    flow,
                    first,
                    last,
                    change: LevelChange::Demote,
                };
                if let Ok(applied) = apply(&self.document, &demote) {
                    self.document = applied.document;
                }
            }
            if let Some(flow) = self.document.flow_mut(flow) {
                if let Some(style) = flow.paragraphs.get(first).map(|paragraph| paragraph.style) {
                    made.push(((kind, depth), style));
                }
                for (offset, alignment) in alignments.into_iter().enumerate() {
                    if alignment.is_some()
                        && let Some(paragraph) = flow.paragraphs.get_mut(first + offset)
                    {
                        Arc::make_mut(paragraph).format.alignment = alignment;
                    }
                }
            }
        }
    }

    fn properties(&mut self) {
        let properties = &self.exchange.properties;
        let info = self.document.info_mut();
        info.description.clone_from(&properties.description);
        info.keywords.clone_from(&properties.keywords);
        for (name, value) in &properties.custom {
            let index = (0..8)
                .find(|index| info.field_name(*index).eq_ignore_ascii_case(name))
                .or_else(|| {
                    (0..8).find(|index| {
                        info.field_names[*index].trim().is_empty() && info.fields[*index].is_empty()
                    })
                });
            if let Some(index) = index {
                if !info.field_name(index).eq_ignore_ascii_case(name) {
                    info.field_names[index] = name.clone();
                }
                info.fields[index] = value.clone();
            }
        }
        info.created = properties.created.map(|clock| clock.to_ami_pro());
        info.saved = properties.modified.map(|clock| clock.to_ami_pro());
        info.revisions = properties.revisions;
        info.editing_minutes = properties.editing_minutes;
        info.statistics.pages = properties.pages;
        info.statistics.words = properties.words;
        info.statistics.characters = properties.characters;
    }
}

/// Character runs being collected: merged where they agree.
#[derive(Default)]
struct Runs {
    runs: Vec<CharacterRun>,
}

impl Runs {
    fn push(&mut self, start: usize, end: usize, format: CharacterFormat) {
        if start == end || format.is_empty() {
            return;
        }
        if let Some(last) = self.runs.last_mut()
            && last.end == start
            && last.format == format
        {
            last.end = end;
            return;
        }
        self.runs.push(CharacterRun { start, end, format });
    }

    fn finish(self, length: usize) -> Vec<CharacterRun> {
        self.runs
            .into_iter()
            .filter(|run| run.start < run.end && run.end <= length)
            .collect()
    }
}

/// Appends an object (its U+FFFC) to `paragraph`.
fn push_object(paragraph: &mut AmiParagraph, object: InlineObject) {
    paragraph.text.push(OBJECT_REPLACEMENT_CHARACTER);
    paragraph.objects.push(object);
}

/// Puts `prefix` before a paragraph's text, moving its runs and marks.
fn shift_paragraph(paragraph: &mut AmiParagraph, prefix: &str) {
    let shift = prefix.len();
    paragraph.text.insert_str(0, prefix);
    for run in &mut paragraph.runs {
        run.start += shift;
        run.end += shift;
    }
    for mark in &mut paragraph.marks {
        mark.byte += shift;
    }
}

/// Text without the characters the model keeps for its own (objects, the
/// line separator stands for a hard return).
fn clean_text(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            OBJECT_REPLACEMENT_CHARACTER => ' ',
            '\r' | '\n' => ' ',
            character => character,
        })
        .collect()
}

/// The text of `blocks`, paragraphs joined by spaces.
fn blocks_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        if let Block::Paragraph(paragraph) = block {
            if !out.is_empty() {
                out.push(' ');
            }
            for inline in &paragraph.inlines {
                match inline {
                    Inline::Text { text, .. } => out.push_str(text),
                    Inline::Tab { .. } => out.push(' '),
                    Inline::Field(field) => out.push_str(&field.result),
                    Inline::Symbol { character, .. } => out.push(*character),
                    _ => {}
                }
            }
        }
    }
    clean_text(&out)
}

fn blocks_empty(blocks: &[Block]) -> bool {
    blocks.iter().all(|block| match block {
        Block::Paragraph(paragraph) => paragraph.inlines.is_empty(),
        Block::Table(_) => false,
    })
}

/// A header's blocks without the shapes placed on the page (repeating
/// frames), and those shapes.
fn take_page_shapes(blocks: &[Block]) -> (Vec<Block>, Vec<Shape>) {
    let mut shapes = Vec::new();
    let blocks = blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph(paragraph) => {
                let mut paragraph = paragraph.clone();
                paragraph.inlines.retain(|inline| match inline {
                    Inline::Shape(shape) if matches!(shape.anchor, Anchor::Floating { .. }) => {
                        shapes.push((**shape).clone());
                        false
                    }
                    _ => true,
                });
                Block::Paragraph(paragraph)
            }
            other => other.clone(),
        })
        .collect();
    (blocks, shapes)
}

/// A band's blocks with indents that reach past the page's margins moved
/// into the band's own margins (Ami Pro's bands have margins of their own):
/// the blocks shifted, and the shift for the band's left and right.
fn band_shift(blocks: &[Block]) -> (Vec<Block>, (Twips, Twips)) {
    let mut left: Twips = 0;
    let mut right: Twips = 0;
    for block in blocks {
        if let Block::Paragraph(paragraph) = block {
            let props = &paragraph.props;
            if let Some(indent) = props.indent_left {
                left = left
                    .min(indent)
                    .min(indent + props.first_line.unwrap_or(0).min(0));
            }
            if let Some(indent) = props.indent_right {
                right = right.min(indent);
            }
        }
    }
    if left == 0 && right == 0 {
        return (blocks.to_vec(), (0, 0));
    }
    let blocks = blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph(paragraph) => {
                let mut paragraph = paragraph.clone();
                let props = &mut paragraph.props;
                props.indent_left = Some(props.indent_left.unwrap_or(0) - left);
                props.indent_right = Some(props.indent_right.unwrap_or(0) - right);
                if let Some(tabs) = &mut props.tabs {
                    for tab in tabs {
                        tab.position -= left;
                    }
                }
                Block::Paragraph(paragraph)
            }
            other => other.clone(),
        })
        .collect();
    (blocks, (left, right))
}

/// A cell's blocks without the picture at its start, and that picture.
fn take_cell_picture(blocks: &[Block]) -> (Vec<Block>, Option<Shape>) {
    let mut blocks = blocks.to_vec();
    if let Some(Block::Paragraph(paragraph)) = blocks.first_mut()
        && let Some(Inline::Shape(shape)) = paragraph.inlines.first()
        && !matches!(&shape.content, ShapeContent::Text { blocks, .. }
            if blocks.iter().any(|block| matches!(block, Block::Table(_))))
    {
        let shape = (**shape).clone();
        paragraph.inlines.remove(0);
        return (blocks, Some(shape));
    }
    (blocks, None)
}

/// Removes the anchors of Where placed frames from the main text: the frame
/// keeps its page, not a place in the text.
fn remove_fixed_anchors(document: &mut Document) {
    let fixed: Vec<FrameId> = document
        .frames()
        .filter(|frame| matches!(frame.placement, Placement::Fixed { .. }))
        .map(|frame| frame.id)
        .collect();
    if fixed.is_empty() {
        return;
    }
    let main = document.main_flow_id();
    let Some(flow) = document.flow_mut(main) else {
        return;
    };
    for paragraph in &mut flow.paragraphs {
        if !paragraph
            .objects
            .iter()
            .any(|object| matches!(object, InlineObject::FrameAnchor(id) if fixed.contains(id)))
        {
            continue;
        }
        let paragraph = Arc::make_mut(paragraph);
        let mut text = String::with_capacity(paragraph.text.len());
        let mut objects = Vec::new();
        let mut removed: Vec<usize> = Vec::new();
        let mut index = 0;
        for (byte, character) in paragraph.text.char_indices() {
            if character == OBJECT_REPLACEMENT_CHARACTER {
                let object = paragraph.objects[index].clone();
                index += 1;
                if matches!(object, InlineObject::FrameAnchor(id) if fixed.contains(&id)) {
                    removed.push(byte);
                    continue;
                }
                objects.push(object);
            }
            text.push(character);
        }
        let shift = |byte: usize| {
            byte - removed.iter().filter(|at| **at < byte).count()
                * OBJECT_REPLACEMENT_CHARACTER.len_utf8()
        };
        for run in &mut paragraph.runs {
            run.start = shift(run.start);
            run.end = shift(run.end);
        }
        paragraph.runs.retain(|run| run.start < run.end);
        for mark in &mut paragraph.marks {
            mark.byte = shift(mark.byte);
        }
        paragraph.text = text;
        paragraph.objects = objects;
    }
}

/// Whether two page set-ups differ in more than their columns.
fn page_differs(a: &PageSetup, b: &PageSetup) -> bool {
    a.width != b.width
        || a.height != b.height
        || a.landscape != b.landscape
        || a.margin_top != b.margin_top
        || a.margin_bottom != b.margin_bottom
        || a.margin_left != b.margin_left
        || a.margin_right != b.margin_right
}

fn letter_setup() -> PageSetup {
    PageSetup {
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
            gap: 720,
            widths: None,
            line_between: false,
        },
        border: None,
    }
}

/// The page geometry of a set-up, keeping `old`'s tabs.
fn geometry_of(setup: &PageSetup, old: &PageGeometry) -> PageGeometry {
    let width = setup.width.max(1_440);
    let height = setup.height.max(1_440);
    // Margins that leave at least half an inch of text.
    let fit = |first: Twips, second: Twips, room: Twips| {
        let (first, second) = (first.max(0), second.max(0));
        let most = (room - 720).max(0);
        if first + second <= most {
            (first, second)
        } else {
            let total = i64::from(first + second).max(1);
            (
                (i64::from(first) * i64::from(most) / total) as Twips,
                (i64::from(second) * i64::from(most) / total) as Twips,
            )
        }
    };
    let (left, right) = fit(setup.margin_left, setup.margin_right, width);
    let (top, bottom) = fit(setup.margin_top, setup.margin_bottom, height);
    let margins = Margins {
        top,
        right,
        bottom,
        left,
    };
    let text_width = width - left - right;
    let mut layout = old.layout.clone();
    layout.margins = margins;
    layout.columns = setup.columns.count.clamp(1, 8);
    columns_tabs(&mut layout);
    if setup.columns.count > 1 {
        layout.gutter = setup.columns.gap.max(0);
    }
    layout.column_edges = setup.columns.widths.as_ref().and_then(|widths| {
        let mut left = 0;
        let edges: Vec<amipro_document::ColumnEdges> = widths
            .iter()
            .map(|(width, gap)| {
                let edge = amipro_document::ColumnEdges {
                    left,
                    right: left + width,
                };
                left += width + gap;
                edge
            })
            .collect();
        // Edges inside the text area, in order.
        let fits = edges.len() > 1
            && edges.len() <= 8
            && edges.windows(2).all(|pair| pair[1].left >= pair[0].right)
            && edges.iter().all(|edge| edge.right > edge.left)
            && edges.last().is_some_and(|last| last.right <= text_width);
        fits.then_some(edges)
    });
    if let Some(edges) = &layout.column_edges {
        layout.columns = edges.len() as u8;
        columns_tabs(&mut layout);
    }
    layout.line_between = setup
        .columns
        .line_between
        .then_some(amipro_document::ColumnLine {
            style: 1,
            colour: amipro_document::BLACK,
        });
    let border = setup.border.map(|border| Border {
        sides: Sides {
            left: border.left,
            right: border.right,
            top: border.top,
            bottom: border.bottom,
        },
        style: line_style(border.line.kind, border.line.width),
        colour: border.line.colour,
        position: if border.distance <= 0 {
            LinePosition::Outside
        } else if border.distance >= margins.top - border.line.width {
            LinePosition::Inside
        } else if border.distance < margins.top * 3 / 8 {
            LinePosition::CloseToOutside
        } else if border.distance < margins.top * 5 / 8 {
            LinePosition::Middle
        } else {
            LinePosition::CloseToInside
        },
    });
    PageGeometry {
        width,
        height,
        layout,
        border,
    }
}

/// A picture frame's margins: the picture's place inside the frame, where
/// it lies inside it (so that it fills the frame's picture area).
fn picture_margins(placed: &crate::PlacedPicture, shape: &Shape) -> Margins {
    let right = shape.width - placed.left - placed.width;
    let bottom = shape.height - placed.top - placed.height;
    if placed.left >= 0 && placed.top >= 0 && right >= 0 && bottom >= 0 {
        Margins {
            top: placed.top,
            right,
            bottom,
            left: placed.left,
        }
    } else {
        Margins::uniform(0)
    }
}

/// One tab list for each of a box's columns (the first column's copied).
fn columns_tabs(layout: &mut LayoutBox) {
    let first = layout.tabs.first().cloned().unwrap_or_default();
    layout
        .tabs
        .resize(usize::from(layout.columns.max(1)), first);
}

/// A header or footer band frame's style: its text `distance` from the
/// paper's edge, its sides the page's margins.
fn band_style(kind: HeaderFooterKind, page: &PageGeometry, distance: Twips) -> FrameStyle {
    let margins = page.layout.margins;
    let (top, bottom) = match kind {
        HeaderFooterKind::Header => (distance.max(0), 72),
        HeaderFooterKind::Footer => (72, distance.max(0)),
    };
    let text_width = page.width - margins.left - margins.right;
    let mut interior = LayoutBox::single_column(Margins {
        top,
        right: margins.right,
        bottom,
        left: margins.left,
    });
    interior.tabs = vec![vec![
        TabStop::new(AmiTabKind::Centre, text_width / 2),
        TabStop::new(AmiTabKind::Right, text_width),
    ]];
    FrameStyle {
        border: Border {
            sides: Sides::NONE,
            ..FrameStyle::default().border
        },
        interior,
        ..FrameStyle::default()
    }
}

/// Ami Pro's default stops: left tabs every half inch.
fn default_tabs() -> Vec<Tab> {
    (1..=12)
        .map(|stop| Tab {
            position: 720 * stop,
            kind: TabKind::Left,
            leader: TabLeader::None,
        })
        .collect()
}

fn ami_tabs(tabs: &[Tab]) -> Vec<TabStop> {
    tabs.iter()
        .filter(|tab| tab.position > 0)
        .take(LayoutBox::MAX_TABS)
        .map(|tab| TabStop {
            kind: match tab.kind {
                TabKind::Left => AmiTabKind::Left,
                TabKind::Centre => AmiTabKind::Centre,
                TabKind::Right => AmiTabKind::Right,
                TabKind::Decimal => AmiTabKind::Decimal,
            },
            position: tab.position,
            leader: match tab.leader {
                TabLeader::None => AmiTabLeader::None,
                TabLeader::Dots => AmiTabLeader::Dots,
                TabLeader::Hyphens => AmiTabLeader::Hyphens,
                TabLeader::Line => AmiTabLeader::Line,
            },
        })
        .collect()
}

fn ami_alignment(alignment: Alignment) -> AmiAlignment {
    match alignment {
        Alignment::Left => AmiAlignment::Left,
        Alignment::Right => AmiAlignment::Right,
        Alignment::Centre => AmiAlignment::Centre,
        Alignment::Justify => AmiAlignment::Justify,
    }
}

/// Indents in Ami Pro's terms: the first line `first` past All, the others
/// `rest` past it; a hanging first line starts at All.
fn indents_of(left: Twips, first_line: Twips, right: Twips) -> Indents {
    if first_line >= 0 {
        Indents {
            all: left.max(0),
            first: first_line,
            rest: 0,
            right: right.max(0),
        }
    } else {
        let all = (left + first_line).max(0);
        Indents {
            all,
            first: 0,
            rest: left.max(0) - all,
            right: right.max(0),
        }
    }
}

fn ami_indents(props: &ParagraphProps) -> Indents {
    indents_of(
        props.indent_left.unwrap_or(0),
        props.first_line.unwrap_or(0),
        props.indent_right.unwrap_or(0),
    )
}

/// The indents `props` give, where they set any, else `style`'s.
fn ami_indents_over(props: &ParagraphProps, style: &Indents) -> Indents {
    if props.indent_left.is_none() && props.first_line.is_none() && props.indent_right.is_none() {
        return *style;
    }
    let left = props.indent_left.unwrap_or(style.all + style.rest);
    let first = props.first_line.unwrap_or(style.first - style.rest);
    let right = props.indent_right.unwrap_or(style.right);
    let made = indents_of(left, first, right);
    // The same lines as the style's: keep the style's own form.
    if made.all + made.rest == style.all + style.rest
        && made.all + made.first == style.all + style.first
        && made.right == style.right
    {
        *style
    } else {
        made
    }
}

fn overlay_paragraph(target: &mut ParagraphProps, over: &ParagraphProps) {
    fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
        if value.is_some() {
            target.clone_from(value);
        }
    }
    take(&mut target.alignment, &over.alignment);
    take(&mut target.indent_left, &over.indent_left);
    take(&mut target.indent_right, &over.indent_right);
    take(&mut target.first_line, &over.first_line);
    take(&mut target.space_before, &over.space_before);
    take(&mut target.space_after, &over.space_after);
    take(&mut target.line, &over.line);
    take(&mut target.keep_with_next, &over.keep_with_next);
    take(&mut target.keep_together, &over.keep_together);
    take(&mut target.tabs, &over.tabs);
    take(&mut target.border_top, &over.border_top);
    take(&mut target.border_bottom, &over.border_bottom);
    take(&mut target.no_hyphenation, &over.no_hyphenation);
}

fn attributes_of(props: &CharProps) -> CharacterAttributes {
    let underline = props.underline.unwrap_or(Underline::None);
    CharacterAttributes {
        underline: underline == Underline::Single,
        word_underline: underline == Underline::Words,
        double_underline: underline == Underline::Double,
        strikethrough: props.strike.unwrap_or(false),
        superscript: props.script == Some(Script::Super),
        subscript: props.script == Some(Script::Sub),
        caps: if props.caps == Some(true) {
            Some(Caps::Upper)
        } else if props.small_caps == Some(true) {
            Some(Caps::Small)
        } else {
            None
        },
        colour: props.colour.unwrap_or(0),
        protected: false,
        no_hyphenation: false,
        overstrike: None,
    }
}

/// What `props` set over `style`, as a run's formatting.
fn character_format(props: &CharProps, style: &ParagraphStyle) -> CharacterFormat {
    let mut format = CharacterFormat::default();
    if let Some(font) = props.font.as_deref().map(str::trim)
        && !font.is_empty()
        && !font.eq_ignore_ascii_case(&style.font_family)
    {
        format.font_family = Some(font.to_owned());
    }
    if let Some(size) = props.size.filter(|size| *size > 0)
        && (size as f32 / 20.0 - style.font_size_points).abs() > 0.01
    {
        format.font_size_points = Some(size as f32 / 20.0);
    }
    if let Some(bold) = props.bold
        && bold != style.bold
    {
        format.bold = Some(bold);
    }
    if let Some(italic) = props.italic
        && italic != style.italic
    {
        format.italic = Some(italic);
    }
    let attributes = &style.attributes;
    if let Some(underline) = props.underline {
        let (single, words, double) = (
            underline == Underline::Single,
            underline == Underline::Words,
            underline == Underline::Double,
        );
        if single != attributes.underline {
            format.underline = Some(single);
        }
        if words != attributes.word_underline {
            format.word_underline = Some(words);
        }
        if double != attributes.double_underline {
            format.double_underline = Some(double);
        }
    }
    if let Some(strike) = props.strike
        && strike != attributes.strikethrough
    {
        format.strikethrough = Some(strike);
    }
    if let Some(script) = props.script {
        let (superscript, subscript) = (script == Script::Super, script == Script::Sub);
        if superscript != attributes.superscript {
            format.superscript = Some(superscript);
        }
        if subscript != attributes.subscript {
            format.subscript = Some(subscript);
        }
    }
    let caps = if props.caps == Some(true) {
        Some(Caps::Upper)
    } else if props.small_caps == Some(true) {
        Some(Caps::Small)
    } else if props.caps == Some(false) || props.small_caps == Some(false) {
        if attributes.caps.is_some() {
            Some(Caps::Normal)
        } else {
            None
        }
    } else {
        attributes.caps
    };
    if caps != attributes.caps && !(caps == Some(Caps::Normal) && attributes.caps.is_none()) {
        format.caps = caps;
    }
    if let Some(colour) = props.colour.map(|colour| colour & amipro_document::WHITE)
        && colour != attributes.colour
    {
        format.colour = Some(colour);
    }
    format
}

fn rules_of(top: Option<RuleLine>, bottom: Option<RuleLine>) -> ParagraphRules {
    let side = |line: Option<RuleLine>| match line {
        Some(line) => RuleSide {
            on: true,
            line: line_style(line.kind, line.width) as u8,
            space: line.space + line.width,
        },
        None => RuleSide {
            on: false,
            line: 1,
            space: 10,
        },
    };
    let colour = top.or(bottom).map_or(0, |line| line.colour);
    ParagraphRules {
        above: side(top),
        below: side(bottom),
        length: RuleLength::Margins,
        other_length: 0,
        colour,
        other_flags: 0,
        unknown: [0, 0],
        unit: 1,
    }
}

/// Ami Pro's line style for a line's kind and width.
fn line_style(kind: LineKind, width: Twips) -> u16 {
    match kind {
        LineKind::Double if width <= 80 => 8,
        LineKind::Double => 9,
        LineKind::Triple => 10,
        LineKind::ThickThin => 11,
        LineKind::ThinThick => 12,
        LineKind::Single => {
            let widths = [
                (1, 10),
                (2, 20),
                (3, 40),
                (4, 80),
                (5, 120),
                (6, 160),
                (7, 240),
            ];
            widths
                .iter()
                .min_by_key(|(_, known)| (known - width).abs())
                .map_or(1, |(style, _)| *style)
        }
    }
}

/// A cell's borders as Ami Pro's line word (`CellLines::from_word`).
fn cell_line_word(borders: &[Option<RuleLine>; 4]) -> u16 {
    let style =
        |line: &Option<RuleLine>| line.map_or(0, |line| line_style(line.kind, line.width) & 0xF);
    let [top, right, bottom, left] = borders;
    style(left) | style(right) << 4 | style(top) << 8 | style(bottom) << 12
}

fn face_bits(family: FontFamily) -> u16 {
    match family {
        FontFamily::Roman => 0xC000,
        FontFamily::Swiss => 0x4000,
        FontFamily::Modern => 0x8000,
        _ => 0,
    }
}

/// Ami Pro's bullet code for a bullet character (the export's
/// `bullet_character` backwards).
fn bullet_code(character: char) -> Option<char> {
    (0x08u8..=0x19).map(char::from).find(|code| {
        crate::numbering::bullet_character(*code).and_then(|text| text.chars().next())
            == Some(character)
    })
}

/// A list level as a style's bullets and numbers.
fn bullets_of(level: &crate::ListLevel, outline: u8) -> amipro_document::Bullets {
    let code = match level.format {
        NumberFormat::Decimal => Some('\u{1A}'),
        NumberFormat::UpperRoman => Some('\u{1B}'),
        NumberFormat::LowerRoman => Some('\u{1C}'),
        NumberFormat::UpperLetter => Some('\u{1D}'),
        NumberFormat::LowerLetter => Some('\u{1E}'),
        NumberFormat::Bullet | NumberFormat::None => None,
    };
    let mut text = String::new();
    let mut cumulative = false;
    let mut chars = level.text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '%' {
            match chars.next() {
                Some('%') => text.push('%'),
                Some(digit) if digit.is_ascii_digit() => {
                    text.push(code.unwrap_or('\u{1A}'));
                    if text
                        .chars()
                        .filter(|c| matches!(c, '\u{1A}'..='\u{1E}'))
                        .count()
                        > 1
                    {
                        cumulative = true;
                    }
                }
                _ => {}
            }
        } else if level.format == NumberFormat::Bullet {
            // A bullet's character as Ami Pro's code, other text as itself.
            text.push(bullet_code(character).unwrap_or(match character {
                '\u{F0B7}' | '\u{F0A7}' | '\u{25CF}' => '\u{11}',
                other => other,
            }));
        } else {
            text.push(character);
        }
    }
    if level.format == NumberFormat::Bullet && text.is_empty() {
        text.push('\u{11}');
    }
    let mut flags = 0;
    if level.restart_after_higher {
        flags |= 2;
    }
    if cumulative {
        flags |= 8;
    }
    if level.right_align {
        flags |= 16;
    }
    let props = &level.props;
    let mut attributes = 0;
    if props.bold == Some(true) {
        attributes |= 1;
    }
    if props.italic == Some(true) {
        attributes |= 2;
    }
    match props.underline {
        Some(Underline::Single) => attributes |= 4,
        Some(Underline::Words) => attributes |= 8,
        _ => {}
    }
    if props.caps == Some(true) {
        attributes |= 16;
    }
    if props.script == Some(Script::Super) {
        attributes |= 256;
    }
    amipro_document::Bullets {
        text,
        level: outline,
        attributes,
        flags,
        ..amipro_document::Bullets::default()
    }
}

/// Ami Pro's date format letter for Word's date picture
/// (`fields::date_picture` backwards); capitals for upper case.
fn date_letter(picture: &str, upper: bool) -> u8 {
    let letter = (b'a'..=b'l')
        .find(|letter| crate::fields::date_picture(*letter).0 == picture)
        .unwrap_or(b'a');
    if upper {
        letter.to_ascii_uppercase()
    } else {
        letter
    }
}

/// The extension Ami Pro's picture record names for an image.
fn extension(image: &Image) -> &'static str {
    match image.kind {
        ImageKind::Png => "png",
        ImageKind::Jpeg => "jpg",
        ImageKind::Gif => "gif",
        ImageKind::Bmp => "bmp",
        ImageKind::Wmf => "wmf",
        ImageKind::Emf => "emf",
        ImageKind::Tiff => "tif",
        ImageKind::Svg => "svg",
    }
}

/// A comment as Ami Pro's note mark (VM NT1): `<:N` with its time, the
/// author's initials, the note's colour, and two values of 1, then its
/// paragraphs, each followed by a blank line, and a lone `>`.
/// A note's initials: the comment's, or its author's first letters.
fn note_initials(note: &Note) -> String {
    let initials: String = if note.initials.trim().is_empty() {
        note.author
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(3)
            .collect::<String>()
            .to_uppercase()
    } else {
        note.initials.trim().to_owned()
    };
    initials.replace(',', "").chars().take(6).collect()
}

#[allow(dead_code)]
fn note_mark(note: &Note, now: u32) -> String {
    let time = note.date.map_or(now, |date| date.to_ami_pro());
    let initials: String = if note.initials.trim().is_empty() {
        note.author
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(3)
            .collect::<String>()
            .to_uppercase()
    } else {
        note.initials.trim().to_owned()
    };
    let mut raw = format!("<:N{time},{},65535,1,1\n", escape_control(&initials));
    let mut any = false;
    for block in &note.blocks {
        let text = blocks_text(std::slice::from_ref(block));
        raw.push_str(&escape_control(&text));
        raw.push_str("\n\n");
        any = true;
    }
    if !any {
        raw.push_str("\n\n");
    }
    raw.push('>');
    raw
}

/// Text as it stands in a control's argument in a SAM file (the writer's
/// `escape_control_bytes`): `<` doubled, `>` as `<;>`, the bytes from 0x80
/// in Ami Pro's escapes.
fn escape_control(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        let byte = amipro_document::windows_1252(character);
        match byte {
            b'\t' => out.push('\t'),
            0x00..=0x1F => {}
            0x80..=0xBF => {
                out.push_str("</");
                out.push(char::from(byte - 0x40));
                out.push('>');
            }
            0xC0..=0xFF => {
                out.push_str("<\\");
                out.push(char::from(byte.wrapping_add(0x80)));
                out.push('>');
            }
            b'<' => out.push_str("<<"),
            b'>' => out.push_str("<;>"),
            b'[' => out.push_str("<[>"),
            _ => out.push(char::from(byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indents_turn_into_ami_pros() {
        assert_eq!(
            indents_of(720, -360, 0),
            Indents {
                all: 360,
                first: 0,
                rest: 360,
                right: 0
            }
        );
        assert_eq!(
            indents_of(720, 360, 100),
            Indents {
                all: 720,
                first: 360,
                rest: 0,
                right: 100
            }
        );
    }

    #[test]
    fn line_styles_and_dates_come_back() {
        assert_eq!(line_style(LineKind::Single, 40), 3);
        assert_eq!(line_style(LineKind::Double, 60), 8);
        assert_eq!(date_letter("MMMM d, yyyy", false), b'b');
        assert_eq!(date_letter("dddd, MMMM d, yyyy", true), b'D');
        assert_eq!(bullet_code('\u{2666}'), Some('\u{15}'));
    }

    #[test]
    fn automatic_rows_take_their_texts_height_unless_ami_pro_measured_them() {
        // A Word row at least an inch tall holding one line: Ami Pro would
        // measure it to its line (VM RC1–RC3; twin AIMO); a file the app
        // wrote keeps Ami Pro's own measure.
        let rows = |generator: &str| {
            let rtf = format!(
                r"{{\rtf1\ansi{generator}\trowd\trrh1440\cellx2880\cellx5760\pard\intbl One\cell Two\cell\row\pard Body\par}}"
            );
            let inflate = |_: &[u8], _: usize| None;
            let read =
                crate::import::read(rtf.as_bytes(), Some("rtf"), &Default::default(), &inflate)
                    .unwrap();
            let base = Document::default();
            let options = RealiseOptions {
                styles: StyleOption::Convert,
                base: &base,
                inserting: false,
                into_text: false,
                title: String::new(),
                now: 0,
            };
            let mut findings = read.findings.clone();
            let realised = realise(
                &read.document,
                &options,
                &mut amipro_layout::FixedShaper,
                &mut findings,
            );
            realised
                .document
                .frames()
                .find_map(|frame| match &frame.content {
                    FrameContent::Table(table) => Some(table.rows.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let foreign = rows("");
        assert!(foreign[0] < 1440 - ROW_SHRINK, "{foreign:?}");
        assert!(foreign[0] > 0);
        let own = rows(r"{\*\generator AmiPro-Linux;}");
        assert!(own[0] > 1440 - ROW_SHRINK, "{own:?}");
    }

    #[test]
    fn a_cells_picture_gives_its_row_the_pictures_height() {
        // A picture fills its cell (T2): a row holding one is at least the
        // picture's height, and is not shrunk to its text (twin AIMW).
        const PNG: &str = "89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415478DA63F8FFFF3F0005FE02FEA7D6A4C00000000049454E44AE426082";
        let rtf = format!(
            r"{{\rtf1\ansi\trowd\cellx2880\cellx5760\pard\intbl{{\pict\pngblip\picwgoal1000\pichgoal1500 {PNG}}}\cell Two\cell\row\pard Body\par}}"
        );
        let inflate = |_: &[u8], _: usize| None;
        let read = crate::import::read(rtf.as_bytes(), Some("rtf"), &Default::default(), &inflate)
            .unwrap();
        let base = Document::default();
        let options = RealiseOptions {
            styles: StyleOption::Convert,
            base: &base,
            inserting: false,
            into_text: false,
            title: String::new(),
            now: 0,
        };
        let mut findings = read.findings.clone();
        let realised = realise(
            &read.document,
            &options,
            &mut amipro_layout::FixedShaper,
            &mut findings,
        );
        let table = realised
            .document
            .frames()
            .find_map(|frame| match &frame.content {
                FrameContent::Table(table) => Some(table.clone()),
                _ => None,
            })
            .unwrap();
        assert!(table.cells[0].frame.is_some());
        assert!(table.rows[0] >= 1500, "{:?}", table.rows);
    }

    #[test]
    fn a_comment_is_an_ami_pro_note() {
        let note = Note {
            author: "Lee Smith".into(),
            initials: String::new(),
            date: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text {
                    text: "Check <this> é".into(),
                    props: CharProps::default(),
                }],
                ..Paragraph::default()
            })],
        };
        assert_eq!(
            note_mark(&note, 1_792_635_771),
            "<:N1792635771,LS,65535,1,1\nCheck <<this<;> <\\i>\n\n>"
        );
    }
}
