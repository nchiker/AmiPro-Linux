//! Rich Text read into an exchange document (RTF 1.9.1; what Word,
//! LibreOffice, WordPad, Ami Pro's `rtf.flt`, and the app's own writer
//! produce; `docs/research/IMPORT_NOTES.md`).
//!
//! A byte-level tokenizer feeds groups whose state (character and
//! paragraph formatting, the destination) is saved and restored as RTF
//! says. Text goes to the story being read: the main text, a header or
//! footer, a footnote, a comment, a text box. A story, shape, field, or
//! picture a group opens ends when that group closes. Paragraphs carry what
//! they set beyond their style. Tables are gathered row by row onto one
//! grid; shapes and positioned paragraphs become frames; `\listtable` and
//! the older `\pn` become lists. What has no place in an Ami Pro document
//! is a finding.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::import::{FindingKind, Findings, ImportError};
use crate::{
    Alignment, Anchor, Block, Cell, CharProps, Colour, Columns, DateSource, ExchangeDocument,
    Field, FieldKind, Font, FontFamily, Footnote, HeaderFooters, HorizontalFrom, Image, ImageKind,
    Inline, LineKind, LineSpacing, List, ListInstance, ListLevel, Note, NumberFormat, PageBorder,
    PageNumbering, PageSetup, Paragraph, ParagraphProps, PlacedPicture, Properties, Row, RowHeight,
    RuleLine, Script, Section, Settings, Shape, ShapeContent, ShapeLine, ShapeShadow, ShapeWrap,
    Style, Tab, TabKind, TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge,
};

/// How deeply groups may nest.
const MAX_DEPTH: usize = 1_000;

/// `bytes` (a file that starts `{\rtf`) as an exchange document.
pub fn read_rtf(bytes: &[u8], findings: &mut Findings) -> Result<ExchangeDocument, ImportError> {
    let start = bytes
        .windows(5)
        .take(512)
        .position(|window| window == b"{\\rtf")
        .ok_or_else(|| ImportError::Damaged("not Rich Text".into()))?;
    let mut reader = Reader::new(&bytes[start..], findings);
    reader.run()?;
    Ok(reader.finish())
}

// ---- Tokens ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token<'b> {
    Open,
    Close,
    /// A control word and its parameter.
    Word(&'b [u8], Option<i32>),
    /// A control symbol (`\~`, `\-`, `\*`, …).
    Symbol(u8),
    /// `\'hh`.
    Hex(u8),
    Text(&'b [u8]),
    /// `\binN`'s bytes.
    Binary(&'b [u8]),
}

struct Lexer<'b> {
    bytes: &'b [u8],
    at: usize,
}

impl<'b> Lexer<'b> {
    fn next(&mut self) -> Option<Token<'b>> {
        let bytes = self.bytes;
        loop {
            let byte = *bytes.get(self.at)?;
            match byte {
                b'{' => {
                    self.at += 1;
                    return Some(Token::Open);
                }
                b'}' => {
                    self.at += 1;
                    return Some(Token::Close);
                }
                b'\r' | b'\n' | 0 => self.at += 1,
                b'\\' => {
                    let Some(&next) = bytes.get(self.at + 1) else {
                        self.at += 1;
                        return None;
                    };
                    if next.is_ascii_alphabetic() {
                        let start = self.at + 1;
                        let mut end = start;
                        while end < bytes.len()
                            && bytes[end].is_ascii_alphabetic()
                            && end - start < 32
                        {
                            end += 1;
                        }
                        let word = &bytes[start..end];
                        let mut at = end;
                        let negative = bytes.get(at) == Some(&b'-')
                            && bytes.get(at + 1).is_some_and(u8::is_ascii_digit);
                        if negative {
                            at += 1;
                        }
                        let digits = at;
                        while at < bytes.len() && bytes[at].is_ascii_digit() && at - digits < 10 {
                            at += 1;
                        }
                        let parameter = (at > digits).then(|| {
                            let value: i64 = std::str::from_utf8(&bytes[digits..at])
                                .ok()
                                .and_then(|text| text.parse().ok())
                                .unwrap_or(0);
                            let value = if negative { -value } else { value };
                            value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                        });
                        if bytes.get(at) == Some(&b' ') {
                            at += 1;
                        }
                        self.at = at;
                        if word == b"bin" {
                            let length =
                                usize::try_from(parameter.unwrap_or(0).max(0)).unwrap_or(0);
                            let end = self.at.saturating_add(length).min(bytes.len());
                            let data = &bytes[self.at..end];
                            self.at = end;
                            return Some(Token::Binary(data));
                        }
                        return Some(Token::Word(word, parameter));
                    }
                    if next == b'\'' {
                        let digit =
                            |at: usize| bytes.get(at).and_then(|byte| (*byte as char).to_digit(16));
                        let (high, low) = (digit(self.at + 2), digit(self.at + 3));
                        self.at += 4;
                        if let (Some(high), Some(low)) = (high, low) {
                            return Some(Token::Hex((high * 16 + low) as u8));
                        }
                        continue;
                    }
                    self.at += 2;
                    // A backslash before a line end is a paragraph end.
                    if next == b'\r' || next == b'\n' {
                        return Some(Token::Word(b"par", None));
                    }
                    return Some(Token::Symbol(next));
                }
                _ => {
                    let start = self.at;
                    while self.at < bytes.len()
                        && !matches!(bytes[self.at], b'{' | b'}' | b'\\' | b'\r' | b'\n' | 0)
                    {
                        self.at += 1;
                    }
                    return Some(Token::Text(&bytes[start..self.at]));
                }
            }
        }
    }
}

// ---- State ----

/// Where a group's text goes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Dest {
    /// The story at the top of the stack.
    Text,
    Skip,
    FontTable,
    ColorTable,
    StyleSheet,
    StyleEntry,
    Info,
    InfoText(&'static str),
    InfoDate(&'static str),
    UserProps,
    PropName,
    StaticValue,
    FieldInstruction,
    Picture,
    ShapeInstruction,
    ShapeName,
    ShapeValue,
    ListTable,
    List,
    ListLevel,
    LevelText,
    ListOverrideTable,
    ListOverride,
    /// The shown number of a list paragraph (`\listtext`, `\pntext`).
    ListText,
    OldNumbering,
    OldNumberingText(bool),
    BookmarkStart,
    BookmarkEnd,
    /// An annotation's date (`\atndate`, a DTTM).
    AnnotationDate,
    /// The program that wrote the file (`{\*\generator …}`).
    Generator,
    AnnotationId,
    AnnotationAuthor,
}

#[derive(Clone, Debug, PartialEq)]
struct CharState {
    font: Option<i32>,
    /// Half points.
    size: i32,
    bold: bool,
    italic: bool,
    underline: Underline,
    strike: bool,
    script: Script,
    caps: bool,
    small_caps: bool,
    colour: usize,
    hidden: bool,
    deleted: bool,
    inserted: bool,
    highlight: bool,
    spacing: bool,
    character_style: Option<i32>,
}

impl Default for CharState {
    fn default() -> Self {
        Self {
            font: None,
            size: 24,
            bold: false,
            italic: false,
            underline: Underline::None,
            strike: false,
            script: Script::Normal,
            caps: false,
            small_caps: false,
            colour: 0,
            hidden: false,
            deleted: false,
            inserted: false,
            highlight: false,
            spacing: false,
            character_style: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ParaState {
    style: i32,
    alignment: Alignment,
    left: Twips,
    right: Twips,
    first: Twips,
    before: Twips,
    after: Twips,
    /// `\sl` and `\slmult`.
    line: Option<(i32, bool)>,
    keep_next: bool,
    keep: bool,
    page_before: bool,
    tabs: Vec<Tab>,
    pending_tab: (Option<TabKind>, TabLeader),
    border_top: Option<RuleLine>,
    border_bottom: Option<RuleLine>,
    /// The border being described: t, b, l, r, or o (another side, a box).
    border: Option<u8>,
    other_borders: bool,
    in_table: bool,
    nest: i32,
    /// `\ls` and `\ilvl`.
    list: Option<i32>,
    level: i32,
    no_hyphenation: bool,
    frame: Option<FrameProps>,
    outline: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FrameProps {
    horizontal: Option<HorizontalFrom>,
    vertical: Option<VerticalFrom>,
    x: Twips,
    y: Twips,
    width: Twips,
    height: Twips,
    no_wrap: bool,
}

#[derive(Clone, Debug)]
struct Group {
    dest: Dest,
    chars: CharState,
    para: ParaState,
    unicode_skip: u32,
}

/// Text being gathered into blocks.
#[derive(Default)]
struct Story {
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
    /// The next text starts an inline of its own (a field's result).
    split: bool,
    /// The rows of the table being read, the cells of the row, the cell.
    rows: Vec<(RowProps, Vec<Vec<Block>>)>,
    cells: Vec<Vec<Block>>,
    cell: Vec<Block>,
    /// Positioned paragraphs waiting for their frame.
    framed: Vec<(FrameProps, Paragraph)>,
    number_text: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct RowProps {
    left: Twips,
    gap: Twips,
    height: i32,
    heading: bool,
    keep: bool,
    centred: bool,
    padding: [Option<Twips>; 4],
    cells: Vec<CellProps>,
}

#[derive(Clone, Debug, Default)]
struct CellProps {
    right: Twips,
    merge_first: bool,
    merged: bool,
    vertical_first: bool,
    vertical_merged: bool,
    borders: [Option<RuleLine>; 4],
    shading: usize,
}

/// What a story belongs to.
enum StoryKind {
    Main,
    Header(&'static str),
    Footer(&'static str),
    Footnote { endnote: bool, props: CharProps },
    Annotation,
    ShapeText,
}

/// What a group opened, to end when it closes.
enum Scope {
    Story,
    Shape,
    Field,
    Picture,
}

struct OpenField {
    instruction: String,
    /// Where the result began: the story's depth and its inline count.
    result_at: Option<(usize, usize)>,
}

struct PictureData {
    kind: Option<&'static str>,
    data: Vec<u8>,
    half: Option<u8>,
    width: i32,
    height: i32,
    goal_width: i32,
    goal_height: i32,
    scale_x: i32,
    scale_y: i32,
    crop: [i32; 4],
}

#[derive(Default)]
struct ShapeData {
    left: Twips,
    top: Twips,
    right: Twips,
    bottom: Twips,
    horizontal: Option<HorizontalFrom>,
    vertical: Option<VerticalFrom>,
    wrap: i32,
    behind: bool,
    z: i32,
    properties: BTreeMap<String, String>,
    name: String,
    value: String,
    picture: Option<(usize, i32, i32)>,
    text: Option<Vec<Block>>,
}

#[derive(Default, Clone)]
struct StyleDef {
    number: i32,
    character_style: bool,
    name: String,
    based_on: Option<i32>,
    next: Option<i32>,
    chars: CharState,
    para: ParaState,
}

struct SectionProps {
    page: PageSetup,
    new_page: bool,
    title_page: bool,
    page_start: Option<u32>,
    page_format: Option<NumberFormat>,
    restart: bool,
    headers: HeaderFooters,
    footers: HeaderFooters,
}

struct Reader<'b, 'f> {
    lexer: Lexer<'b>,
    findings: &'f mut Findings,
    groups: Vec<Group>,
    scopes: Vec<(usize, Scope)>,
    stories: Vec<(StoryKind, Story)>,
    /// Characters still to skip after `\u`.
    skip: u32,
    fonts: Vec<(i32, Font, i32)>,
    font_number: i32,
    font_charset: i32,
    font_family: FontFamily,
    font_fixed: bool,
    font_name: String,
    colours: Vec<Option<Colour>>,
    colour: (u8, u8, u8, bool),
    styles: Vec<StyleDef>,
    style: StyleDef,
    default_font: i32,
    code_page: u32,
    document_page: PageSetup,
    section: SectionProps,
    sections: Vec<Section>,
    settings: Settings,
    /// The `\generator` destination's text.
    generator: String,
    properties: Properties,
    info_text: String,
    info_date: [i32; 6],
    prop_name: String,
    fields: Vec<OpenField>,
    picture: Option<PictureData>,
    shapes: Vec<ShapeData>,
    images: Vec<Image>,
    lists: Vec<(i32, List)>,
    list: List,
    list_id: i32,
    level: Option<ListLevel>,
    level_text: Vec<u8>,
    overrides: Vec<(i32, i32)>,
    override_list: (i32, i32),
    old: Option<(NumberFormat, String, String, u32)>,
    old_lists: Vec<(NumberFormat, String, String, u32)>,
    row: RowProps,
    cell_props: CellProps,
    annotation: (String, String),
    /// A bookmark's name as read (`\bkmkstart`, `\bkmkend`).
    bookmark_name: String,
    /// The annotation's date as read.
    annotation_date: String,
    tab_default: Twips,
    run: (bool, bool, bool, bool, bool),
    next_shape: u32,
}

fn default_page() -> PageSetup {
    PageSetup {
        width: 12_240,
        height: 15_840,
        landscape: false,
        margin_top: 1_440,
        margin_bottom: 1_440,
        margin_left: 1_800,
        margin_right: 1_800,
        header_distance: 720,
        footer_distance: 720,
        columns: Columns {
            count: 1,
            gap: 720,
            widths: None,
            line_between: false,
        },
        border: None,
    }
}

impl<'b, 'f> Reader<'b, 'f> {
    fn new(bytes: &'b [u8], findings: &'f mut Findings) -> Self {
        let page = default_page();
        Self {
            lexer: Lexer { bytes, at: 0 },
            findings,
            groups: Vec::new(),
            scopes: Vec::new(),
            stories: vec![(StoryKind::Main, Story::default())],
            skip: 0,
            fonts: Vec::new(),
            font_number: 0,
            font_charset: 0,
            font_family: FontFamily::Unknown,
            font_fixed: false,
            font_name: String::new(),
            colours: Vec::new(),
            colour: (0, 0, 0, false),
            styles: Vec::new(),
            style: StyleDef::default(),
            default_font: 0,
            code_page: 1252,
            document_page: page.clone(),
            section: SectionProps {
                page,
                new_page: true,
                title_page: false,
                page_start: None,
                page_format: None,
                restart: false,
                headers: HeaderFooters::default(),
                footers: HeaderFooters::default(),
            },
            sections: Vec::new(),
            settings: Settings::default(),
            generator: String::new(),
            properties: Properties::default(),
            info_text: String::new(),
            info_date: [0; 6],
            prop_name: String::new(),
            fields: Vec::new(),
            picture: None,
            shapes: Vec::new(),
            images: Vec::new(),
            lists: Vec::new(),
            list: List { levels: Vec::new() },
            list_id: 0,
            level: None,
            level_text: Vec::new(),
            overrides: Vec::new(),
            override_list: (0, 0),
            old: None,
            old_lists: Vec::new(),
            row: RowProps::default(),
            cell_props: CellProps::default(),
            annotation: (String::new(), String::new()),
            bookmark_name: String::new(),
            annotation_date: String::new(),
            tab_default: 720,
            run: (false, false, false, false, false),
            next_shape: 1,
        }
    }

    fn dest(&self) -> Dest {
        self.groups.last().map_or(Dest::Skip, |group| group.dest)
    }

    fn chars(&mut self) -> &mut CharState {
        &mut self.groups.last_mut().expect("a group").chars
    }

    fn para(&mut self) -> &mut ParaState {
        &mut self.groups.last_mut().expect("a group").para
    }

    fn story(&mut self) -> &mut Story {
        &mut self.stories.last_mut().expect("the main story").1
    }

    /// Marks what the group just opened, so that it ends with the group.
    fn open_scope(&mut self, scope: Scope) {
        let depth = self.groups.len() - 1;
        self.scopes.push((depth, scope));
    }

    fn run(&mut self) -> Result<(), ImportError> {
        let mut fresh = false;
        while let Some(token) = self.lexer.next() {
            match token {
                Token::Open => {
                    if self.groups.len() >= MAX_DEPTH {
                        return Err(ImportError::Damaged("groups nested too deeply".into()));
                    }
                    let mut group = self.groups.last().cloned().unwrap_or(Group {
                        dest: Dest::Text,
                        chars: CharState::default(),
                        para: ParaState::default(),
                        unicode_skip: 1,
                    });
                    // A style sheet's entry (Word writes style 0 without
                    // `\s0`).
                    if group.dest == Dest::StyleSheet {
                        group.dest = Dest::StyleEntry;
                        group.chars = CharState::default();
                        group.para = ParaState::default();
                        self.style = StyleDef::default();
                    }
                    self.groups.push(group);
                    self.skip = 0;
                    fresh = true;
                    continue;
                }
                Token::Close => {
                    self.skip = 0;
                    self.close_group();
                    if self.groups.is_empty() {
                        break;
                    }
                }
                Token::Symbol(b'*') if fresh => {
                    // An ignorable destination: skipped unless known.
                    match self.lexer.next() {
                        Some(Token::Word(word, parameter)) => {
                            if !self.destination(word, parameter) {
                                self.groups.last_mut().expect("a group").dest = Dest::Skip;
                            }
                        }
                        _ => self.groups.last_mut().expect("a group").dest = Dest::Skip,
                    }
                }
                Token::Word(word, parameter) => {
                    if self.skip > 0 {
                        self.skip -= 1;
                    } else if !(fresh && self.destination(word, parameter)) {
                        self.word(word, parameter);
                    }
                }
                Token::Symbol(symbol) => {
                    if self.skip > 0 {
                        self.skip -= 1;
                    } else {
                        self.symbol(symbol);
                    }
                }
                Token::Hex(byte) => {
                    if self.skip > 0 {
                        self.skip -= 1;
                    } else {
                        self.byte(byte);
                    }
                }
                Token::Text(text) => {
                    for &byte in text {
                        if self.skip > 0 {
                            self.skip -= 1;
                        } else {
                            self.byte(byte);
                        }
                    }
                }
                Token::Binary(data) => {
                    if let Some(picture) = &mut self.picture {
                        picture.data = data.to_vec();
                        picture.half = None;
                    }
                }
            }
            fresh = false;
        }
        while !self.groups.is_empty() {
            self.close_group();
        }
        Ok(())
    }

    /// A byte of text in the font's code page, or a picture's hex digit.
    fn byte(&mut self, byte: u8) {
        if self.dest() == Dest::Picture {
            if let Some(picture) = &mut self.picture
                && let Some(digit) = (byte as char).to_digit(16)
            {
                match picture.half.take() {
                    Some(high) => picture.data.push(high << 4 | digit as u8),
                    None => picture.half = Some(digit as u8),
                }
            }
            return;
        }
        let character = if self.symbol_font() {
            symbol_character(byte)
        } else if self.code_page == 437 {
            crate::read_text::decode(&[byte], crate::import::Characters::PcAscii)
                .chars()
                .next()
                .unwrap_or('?')
        } else if byte < 0x80 {
            char::from(byte)
        } else {
            amipro_document::decode_windows_1252(&[byte])
                .chars()
                .next()
                .unwrap_or('?')
        };
        self.text(character);
    }

    fn symbol_font(&self) -> bool {
        let Some(font) = self.groups.last().and_then(|group| group.chars.font) else {
            return false;
        };
        self.fonts.iter().any(|(number, font_entry, charset)| {
            *number == font && (*charset == 2 || font_entry.name.eq_ignore_ascii_case("Symbol"))
        })
    }

    /// Starts a destination group for `word`; `false` when it is no
    /// destination.
    fn destination(&mut self, word: &[u8], parameter: Option<i32>) -> bool {
        let in_info = self.dest() == Dest::Info;
        let dest = match word {
            b"fonttbl" => Dest::FontTable,
            b"colortbl" => Dest::ColorTable,
            b"stylesheet" => Dest::StyleSheet,
            b"info" => Dest::Info,
            b"title" if in_info => Dest::InfoText("title"),
            b"subject" if in_info => Dest::InfoText("subject"),
            b"author" if in_info => Dest::InfoText("author"),
            b"keywords" if in_info => Dest::InfoText("keywords"),
            b"doccomm" if in_info => Dest::InfoText("doccomm"),
            b"creatim" => Dest::InfoDate("created"),
            b"revtim" => Dest::InfoDate("modified"),
            b"userprops" => Dest::UserProps,
            b"propname" => Dest::PropName,
            b"staticval" => Dest::StaticValue,
            b"field" => {
                self.fields.push(OpenField {
                    instruction: String::new(),
                    result_at: None,
                });
                self.groups.last_mut().expect("a group").dest = Dest::Text;
                self.open_scope(Scope::Field);
                return true;
            }
            b"fldinst" => Dest::FieldInstruction,
            b"fldrslt" => {
                let depth = self.stories.len();
                let count = self
                    .stories
                    .last()
                    .map_or(0, |(_, story)| story.inlines.len());
                if let Some(field) = self.fields.last_mut() {
                    field.result_at = Some((depth, count));
                }
                self.story().split = true;
                Dest::Text
            }
            b"pict" => {
                self.picture = Some(PictureData {
                    kind: None,
                    data: Vec::new(),
                    half: None,
                    width: 0,
                    height: 0,
                    goal_width: 0,
                    goal_height: 0,
                    scale_x: 100,
                    scale_y: 100,
                    crop: [0; 4],
                });
                self.groups.last_mut().expect("a group").dest = Dest::Picture;
                self.open_scope(Scope::Picture);
                return true;
            }
            b"shppict" => Dest::Text,
            b"nonshppict" | b"shprslt" => Dest::Skip,
            b"shp" => {
                self.shapes.push(ShapeData {
                    wrap: 2,
                    ..ShapeData::default()
                });
                self.groups.last_mut().expect("a group").dest = Dest::Text;
                self.open_scope(Scope::Shape);
                return true;
            }
            b"shpinst" => Dest::ShapeInstruction,
            b"sn" => Dest::ShapeName,
            b"sv" => Dest::ShapeValue,
            b"shptxt" => {
                self.start_story(StoryKind::ShapeText);
                return true;
            }
            b"header" | b"headerr" | b"headerl" | b"headerf" | b"footer" | b"footerr"
            | b"footerl" | b"footerf" => {
                let kind = match word {
                    b"header" | b"footer" | b"headerr" | b"footerr" => "default",
                    b"headerl" | b"footerl" => "even",
                    _ => "first",
                };
                let story = if word.starts_with(b"header") {
                    StoryKind::Header(kind)
                } else {
                    StoryKind::Footer(kind)
                };
                self.start_story(story);
                return true;
            }
            b"footnote" => {
                let state = self.groups.last().expect("a group").chars.clone();
                let props = self.char_props(&state);
                self.start_story(StoryKind::Footnote {
                    endnote: false,
                    props,
                });
                return true;
            }
            b"annotation" => {
                self.start_story(StoryKind::Annotation);
                return true;
            }
            b"atnid" => Dest::AnnotationId,
            b"generator" => Dest::Generator,
            b"atnauthor" => Dest::AnnotationAuthor,
            b"listtable" => Dest::ListTable,
            b"list" if self.dest() == Dest::ListTable => {
                self.list = List { levels: Vec::new() };
                self.list_id = 0;
                Dest::List
            }
            b"listlevel" => {
                self.level = Some(ListLevel {
                    format: NumberFormat::Decimal,
                    text: String::new(),
                    start: 1,
                    restart_after_higher: true,
                    font: None,
                    props: CharProps::default(),
                    right_align: false,
                    indent_left: 0,
                    hanging: 0,
                });
                Dest::ListLevel
            }
            b"leveltext" => {
                self.level_text.clear();
                Dest::LevelText
            }
            b"levelnumbers" | b"listname" | b"liststylename" => Dest::Skip,
            b"listoverridetable" => Dest::ListOverrideTable,
            b"listoverride" => {
                self.override_list = (0, 0);
                Dest::ListOverride
            }
            b"listtext" | b"pntext" => {
                self.story().number_text = Some(String::new());
                Dest::ListText
            }
            b"pn" => {
                self.old = Some((NumberFormat::Decimal, String::new(), String::new(), 1));
                Dest::OldNumbering
            }
            b"pntxta" => Dest::OldNumberingText(true),
            b"pntxtb" => Dest::OldNumberingText(false),
            b"bkmkstart" => Dest::BookmarkStart,
            b"bkmkend" => Dest::BookmarkEnd,
            b"atndate" => Dest::AnnotationDate,
            b"xe" | b"tc" | b"tcn" => {
                self.findings.count(FindingKind::GeneratedText);
                Dest::Skip
            }
            b"s" | b"cs" | b"ds" | b"ts" if self.dest() == Dest::StyleEntry => {
                self.style.number = if word == b"ds" || word == b"ts" {
                    -1
                } else {
                    parameter.unwrap_or(0)
                };
                self.style.character_style = word == b"cs";
                return true;
            }
            b"template"
            | b"xmlnstbl"
            | b"rsidtbl"
            | b"themedata"
            | b"colorschememapping"
            | b"latentstyles"
            | b"datastore"
            | b"mmathPr"
            | b"pgdsctbl"
            | b"fchars"
            | b"lchars"
            | b"falt"
            | b"panose"
            | b"objdata"
            | b"objclass"
            | b"blipuid"
            | b"ftnsep"
            | b"ftnsepc"
            | b"aftnsep"
            | b"aftnsepc"
            | b"revtbl"
            | b"protusertbl"
            | b"docvar"
            | b"filetbl"
            | b"listpicture"
            | b"atrfstart"
            | b"atrfend"
            | b"atnref"
            | b"atnparent"
            | b"atnicn"
            | b"nesttableprops"
            | b"background"
            | b"wgrffmtfilter"
            | b"pnseclvl"
            | b"oldcprops"
            | b"oldpprops"
            | b"oldsprops"
            | b"oldtprops"
            | b"bkmkcoll"
            | b"mergefld"
            | b"formfield"
            | b"datafield"
            | b"hlinkbase"
            | b"comment"
            | b"operator"
            | b"id"
            | b"buptim"
            | b"printim"
            | b"company"
            | b"manager"
            | b"category"
            | b"hlinkbasevalue"
            | b"passwordhash"
            | b"password"
            | b"xmlopen"
            | b"xmlclose"
            | b"xmlattrname"
            | b"xmlattrvalue"
            | b"mhtmltag"
            | b"htmltag"
            | b"factoidname"
            | b"pxe"
            | b"ud" => Dest::Skip,
            _ => return false,
        };
        self.groups.last_mut().expect("a group").dest = dest;
        true
    }

    /// A story the current group holds (a header, a footnote, a text box).
    fn start_story(&mut self, kind: StoryKind) {
        let group = self.groups.last_mut().expect("a group");
        group.dest = Dest::Text;
        group.para = ParaState::default();
        self.stories.push((kind, Story::default()));
        self.open_scope(Scope::Story);
    }

    fn close_group(&mut self) {
        let Some(group) = self.groups.pop() else {
            return;
        };
        let parent = self.groups.last().map(|group| group.dest);
        match group.dest {
            Dest::BookmarkStart | Dest::BookmarkEnd => {
                let name = std::mem::take(&mut self.bookmark_name).trim().to_owned();
                if !name.is_empty() && !name.starts_with('_') {
                    let story = self.story();
                    story.split = true;
                    story.inlines.push(Inline::Bookmark {
                        name,
                        end: group.dest == Dest::BookmarkEnd,
                    });
                }
            }
            Dest::StyleEntry if parent != Some(Dest::StyleEntry) => {
                let mut style = std::mem::take(&mut self.style);
                style.chars = group.chars.clone();
                style.para = group.para.clone();
                style.name = style.name.trim().trim_end_matches(';').trim().to_owned();
                if style.number >= 0 {
                    self.styles.push(style);
                }
            }
            Dest::FontTable if parent == Some(Dest::FontTable) => self.end_font(),
            Dest::InfoText(kind) => {
                let text = std::mem::take(&mut self.info_text).trim().to_owned();
                match kind {
                    "title" => self.properties.title = text,
                    "keywords" => self.properties.keywords = text,
                    "doccomm" => self.properties.description = text,
                    "subject" if self.properties.description.is_empty() => {
                        self.properties.description = text;
                    }
                    _ => {}
                }
            }
            Dest::InfoDate(kind) => {
                let [year, month, day, hour, minute, second] = self.info_date;
                if year > 1900 {
                    let clock = amipro_document::WallClock {
                        year,
                        month: month.clamp(1, 12) as u8,
                        day: day.clamp(1, 31) as u8,
                        hour: hour.clamp(0, 23) as u8,
                        minute: minute.clamp(0, 59) as u8,
                        second: second.clamp(0, 59) as u8,
                    };
                    match kind {
                        "created" => self.properties.created = Some(clock),
                        _ => self.properties.modified = Some(clock),
                    }
                }
                self.info_date = [0; 6];
            }
            Dest::StaticValue => {
                let value = std::mem::take(&mut self.info_text);
                let name = std::mem::take(&mut self.prop_name);
                if !name.trim().is_empty() {
                    self.properties
                        .custom
                        .push((name.trim().to_owned(), value.trim().to_owned()));
                }
            }
            Dest::ShapeValue if parent != Some(Dest::ShapeValue) => {
                if let Some(shape) = self.shapes.last_mut() {
                    let name = std::mem::take(&mut shape.name);
                    let value = std::mem::take(&mut shape.value);
                    shape
                        .properties
                        .insert(name.trim().to_owned(), value.trim().to_owned());
                }
            }
            Dest::ListLevel if parent == Some(Dest::List) => {
                if let Some(mut level) = self.level.take() {
                    level.indent_left = group.para.left;
                    level.hanging = -group.para.first;
                    level.props = char_difference(
                        &self.char_props(&group.chars),
                        &self.char_props(&CharState::default()),
                    );
                    level.font = group
                        .chars
                        .font
                        .and_then(|number| self.font_name_of(number));
                    self.list.levels.push(level);
                }
            }
            Dest::LevelText if parent == Some(Dest::ListLevel) => self.end_level_text(),
            Dest::List if parent == Some(Dest::ListTable) => {
                let list = std::mem::replace(&mut self.list, List { levels: Vec::new() });
                self.lists.push((self.list_id, list));
            }
            Dest::ListOverride if parent == Some(Dest::ListOverrideTable) => {
                self.overrides.push(self.override_list);
            }
            Dest::OldNumbering if parent != Some(Dest::OldNumbering) => {
                if let Some((format, before, after, start)) = self.old.take() {
                    let index = match self.old_lists.iter().position(|known| {
                        (known.0, &known.1, &known.2) == (format, &before, &after)
                    }) {
                        Some(index) => index,
                        None => {
                            self.old_lists.push((format, before, after, start));
                            self.old_lists.len() - 1
                        }
                    };
                    // Old numbering's lists are numbered below zero.
                    if let Some(parent) = self.groups.last_mut() {
                        parent.para.list = Some(-(index as i32) - 1);
                        parent.para.level = 0;
                    }
                }
            }
            _ => {}
        }
        // What the group opened ends with it.
        while let Some((depth, _)) = self.scopes.last() {
            if *depth != self.groups.len() {
                break;
            }
            let (_, scope) = self.scopes.pop().expect("checked");
            match scope {
                Scope::Story => self.end_story(),
                Scope::Shape => self.end_shape(),
                Scope::Field => self.end_field(),
                Scope::Picture => self.end_picture(),
            }
        }
    }

    // ---- Control words ----

    fn word(&mut self, word: &[u8], parameter: Option<i32>) {
        let value = parameter.unwrap_or(1);
        let on = value != 0;
        let dest = self.dest();
        match dest {
            Dest::Skip | Dest::FieldInstruction => {
                if dest == Dest::FieldInstruction {
                    // Words in an instruction are its text's escapes.
                    if let Some(field) = self.fields.last_mut() {
                        field.instruction.push(' ');
                    }
                }
                return;
            }
            Dest::StyleEntry => match word {
                b"sbasedon" => {
                    self.style.based_on = Some(value);
                    return;
                }
                b"snext" => {
                    self.style.next = Some(value);
                    return;
                }
                _ => {}
            },
            Dest::Picture => {
                if let Some(picture) = &mut self.picture {
                    match word {
                        b"pngblip" => picture.kind = Some("png"),
                        b"jpegblip" => picture.kind = Some("jpeg"),
                        b"wmetafile" => picture.kind = Some("wmf"),
                        b"emfblip" => picture.kind = Some("emf"),
                        b"dibitmap" => picture.kind = Some("dib"),
                        b"wbitmap" => picture.kind = Some("ddb"),
                        b"macpict" => picture.kind = Some("pict"),
                        b"pmmetafile" => picture.kind = Some("os2"),
                        b"picw" => picture.width = value,
                        b"pich" => picture.height = value,
                        b"picwgoal" => picture.goal_width = value,
                        b"pichgoal" => picture.goal_height = value,
                        b"picscalex" => picture.scale_x = value,
                        b"picscaley" => picture.scale_y = value,
                        b"piccropl" => picture.crop[0] = value,
                        b"piccropt" => picture.crop[1] = value,
                        b"piccropr" => picture.crop[2] = value,
                        b"piccropb" => picture.crop[3] = value,
                        _ => {}
                    }
                }
                return;
            }
            Dest::FontTable => {
                match word {
                    b"f" => self.font_number = value,
                    b"froman" => self.font_family = FontFamily::Roman,
                    b"fswiss" => self.font_family = FontFamily::Swiss,
                    b"fmodern" => self.font_family = FontFamily::Modern,
                    b"fscript" => self.font_family = FontFamily::Script,
                    b"fdecor" => self.font_family = FontFamily::Decorative,
                    b"fcharset" => self.font_charset = value,
                    b"fprq" => self.font_fixed = value == 1,
                    _ => {}
                }
                return;
            }
            Dest::ColorTable => {
                match word {
                    b"red" => self.colour = (value as u8, self.colour.1, self.colour.2, true),
                    b"green" => self.colour = (self.colour.0, value as u8, self.colour.2, true),
                    b"blue" => self.colour = (self.colour.0, self.colour.1, value as u8, true),
                    _ => {}
                }
                return;
            }
            Dest::InfoDate(_) => {
                let slot = match word {
                    b"yr" => 0,
                    b"mo" => 1,
                    b"dy" => 2,
                    b"hr" => 3,
                    b"min" => 4,
                    b"sec" => 5,
                    _ => return,
                };
                self.info_date[slot] = value;
                return;
            }
            Dest::Info => {
                match word {
                    b"version" => self.properties.revisions = value.max(0) as u32,
                    b"edmins" => self.properties.editing_minutes = value.max(0) as u32,
                    b"nofpages" => self.properties.pages = value.max(0) as u32,
                    b"nofwords" => self.properties.words = value.max(0) as u32,
                    b"nofchars" => self.properties.characters = value.max(0) as u32,
                    _ => {}
                }
                return;
            }
            Dest::List => {
                if word == b"listid" {
                    self.list_id = value;
                }
                return;
            }
            Dest::ListLevel => {
                if let Some(level) = &mut self.level {
                    match word {
                        b"levelnfc" | b"levelnfcn" => {
                            level.format = match value {
                                0 => NumberFormat::Decimal,
                                1 => NumberFormat::UpperRoman,
                                2 => NumberFormat::LowerRoman,
                                3 => NumberFormat::UpperLetter,
                                4 => NumberFormat::LowerLetter,
                                23 => NumberFormat::Bullet,
                                255 => NumberFormat::None,
                                _ => NumberFormat::Decimal,
                            };
                        }
                        b"levelstartat" => level.start = value.max(0) as u32,
                        b"levelnorestart" => level.restart_after_higher = !on,
                        b"leveljc" | b"leveljcn" => level.right_align = value == 2,
                        _ => {}
                    }
                }
                // Indents and fonts of the level are paragraph and
                // character words below.
            }
            Dest::ListOverride => {
                match word {
                    b"listid" => self.override_list.0 = value,
                    b"ls" => self.override_list.1 = value,
                    _ => {}
                }
                return;
            }
            Dest::OldNumbering => {
                if let Some(old) = &mut self.old {
                    match word {
                        b"pnlvlblt" => old.0 = NumberFormat::Bullet,
                        b"pndec" | b"pnlvlbody" if old.0 != NumberFormat::Bullet => {
                            old.0 = NumberFormat::Decimal;
                        }
                        b"pnucrm" => old.0 = NumberFormat::UpperRoman,
                        b"pnlcrm" => old.0 = NumberFormat::LowerRoman,
                        b"pnucltr" => old.0 = NumberFormat::UpperLetter,
                        b"pnlcltr" => old.0 = NumberFormat::LowerLetter,
                        b"pnstart" => old.3 = value.max(0) as u32,
                        _ => {}
                    }
                }
                return;
            }
            Dest::ShapeInstruction => {
                if let Some(shape) = self.shapes.last_mut() {
                    match word {
                        b"shpleft" => shape.left = value,
                        b"shptop" => shape.top = value,
                        b"shpright" => shape.right = value,
                        b"shpbottom" => shape.bottom = value,
                        b"shpbxpage" => shape.horizontal = Some(HorizontalFrom::Page),
                        b"shpbxmargin" => shape.horizontal = Some(HorizontalFrom::Margin),
                        b"shpbxcolumn" => shape.horizontal = Some(HorizontalFrom::Column),
                        b"shpbypage" => shape.vertical = Some(VerticalFrom::Page),
                        b"shpbymargin" => shape.vertical = Some(VerticalFrom::Margin),
                        b"shpbypara" => shape.vertical = Some(VerticalFrom::Paragraph),
                        b"shpwr" => shape.wrap = value,
                        b"shpfblwtxt" => shape.behind = on,
                        b"shpz" => shape.z = value,
                        _ => {}
                    }
                }
                return;
            }
            _ => {}
        }
        // The document's and the section's set-up.
        if self.page_word(word, value) {
            return;
        }
        match word {
            b"ansicpg" => self.code_page = value.max(0) as u32,
            b"pc" => self.code_page = 437,
            b"pca" => self.code_page = 850,
            b"deff" => self.default_font = value,
            b"deftab" => self.tab_default = value.max(1),
            b"uc" => self.groups.last_mut().expect("a group").unicode_skip = value.max(0) as u32,
            b"u" => {
                let code = if value < 0 { value + 65_536 } else { value } as u32;
                let character = char::from_u32(code).unwrap_or('\u{FFFD}');
                self.text(character);
                self.skip = self.groups.last().map_or(1, |group| group.unicode_skip);
            }
            // Character formatting.
            b"plain" => {
                let style = self.chars().character_style;
                let size = 24;
                *self.chars() = CharState {
                    size,
                    character_style: style,
                    ..CharState::default()
                };
            }
            b"f" => self.chars().font = Some(value),
            b"fs" => self.chars().size = value.max(1),
            b"b" => self.chars().bold = on,
            b"i" => self.chars().italic = on,
            b"ul" | b"uld" | b"uldash" | b"uldashd" | b"uldashdd" | b"ulhwave" | b"ulth"
            | b"ulwave" | b"ulldash" | b"ulthd" | b"ulthdash" => {
                self.chars().underline = if on {
                    Underline::Single
                } else {
                    Underline::None
                };
            }
            b"uldb" | b"ululdbwave" => {
                self.chars().underline = if on {
                    Underline::Double
                } else {
                    Underline::None
                };
            }
            b"ulw" => {
                self.chars().underline = if on {
                    Underline::Words
                } else {
                    Underline::None
                }
            }
            b"ulnone" => self.chars().underline = Underline::None,
            b"strike" | b"striked" => self.chars().strike = on,
            b"super" => self.chars().script = Script::Super,
            b"sub" => self.chars().script = Script::Sub,
            b"nosupersub" => self.chars().script = Script::Normal,
            b"up" => self.chars().script = if on { Script::Super } else { Script::Normal },
            b"dn" => self.chars().script = if on { Script::Sub } else { Script::Normal },
            b"caps" => self.chars().caps = on,
            b"scaps" => self.chars().small_caps = on,
            b"cf" => self.chars().colour = value.max(0) as usize,
            b"v" | b"webhidden" => self.chars().hidden = on,
            b"deleted" => self.chars().deleted = on,
            b"revised" => self.chars().inserted = on,
            b"highlight" | b"chcbpat" | b"cb" => self.chars().highlight = value > 0,
            b"expnd" | b"expndtw" | b"charscalex" | b"kerning" => {
                if word != b"kerning" && value != 0 && !(word == b"charscalex" && value == 100) {
                    self.chars().spacing = true;
                }
            }
            b"cs" => {
                self.chars().character_style = Some(value);
                if dest == Dest::Text
                    && let Some(style) = self
                        .styles
                        .iter()
                        .find(|style| style.character_style && style.number == value)
                {
                    let name = style.name.clone();
                    self.findings.count(FindingKind::CharacterStyle(name));
                }
            }
            // Paragraph formatting.
            b"pard" => {
                let in_table = self.para().in_table;
                *self.para() = ParaState::default();
                // `\pard` ends a table only when no `\intbl` follows.
                let _ = in_table;
            }
            b"s" => self.para().style = value,
            b"ql" => self.para().alignment = Alignment::Left,
            b"qr" => self.para().alignment = Alignment::Right,
            b"qc" => self.para().alignment = Alignment::Centre,
            b"qj" | b"qd" => self.para().alignment = Alignment::Justify,
            b"li" | b"lin" => self.para().left = value,
            b"ri" | b"rin" => self.para().right = value,
            b"fi" => self.para().first = value,
            b"sb" => self.para().before = value.max(0),
            b"sa" => self.para().after = value.max(0),
            b"sl" => {
                let multiple = self.para().line.is_some_and(|(_, multiple)| multiple);
                self.para().line = Some((value, multiple));
            }
            b"slmult" => {
                if let Some((height, _)) = self.para().line {
                    self.para().line = Some((height, on));
                }
            }
            b"keepn" => self.para().keep_next = on,
            b"keep" => self.para().keep = on,
            b"pagebb" => self.para().page_before = on,
            b"hyphpar" => self.para().no_hyphenation = !on,
            b"outlinelevel" => self.para().outline = Some((value + 1).clamp(1, 9) as u8),
            b"lotusoutlinelevel" => self.para().outline = Some(value.clamp(1, 9) as u8),
            b"tqr" => self.para().pending_tab.0 = Some(TabKind::Right),
            b"tqc" => self.para().pending_tab.0 = Some(TabKind::Centre),
            b"tqdec" => self.para().pending_tab.0 = Some(TabKind::Decimal),
            b"tldot" | b"tlmdot" => self.para().pending_tab.1 = TabLeader::Dots,
            b"tlhyph" => self.para().pending_tab.1 = TabLeader::Hyphens,
            b"tlul" | b"tlth" | b"tleq" => self.para().pending_tab.1 = TabLeader::Line,
            b"tx" | b"tb" => {
                let (kind, leader) = std::mem::take(&mut self.para().pending_tab);
                self.para().tabs.push(Tab {
                    position: value,
                    kind: kind.unwrap_or(TabKind::Left),
                    leader,
                });
                self.para().pending_tab = (None, TabLeader::None);
            }
            b"pgbrdrt" | b"pgbrdrl" | b"pgbrdrb" | b"pgbrdrr" => {
                self.para().border = Some(b'p');
                let side = word[6];
                let border = self.section.page.border.get_or_insert(PageBorder {
                    top: false,
                    bottom: false,
                    left: false,
                    right: false,
                    line: RuleLine {
                        kind: LineKind::Single,
                        width: 15,
                        colour: 0,
                        space: 0,
                    },
                    distance: 0,
                });
                match side {
                    b't' => border.top = true,
                    b'l' => border.left = true,
                    b'b' => border.bottom = true,
                    _ => border.right = true,
                }
            }
            b"brdrt" => self.para().border = Some(b't'),
            b"brdrb" => self.para().border = Some(b'b'),
            b"brdrl" | b"brdrr" | b"box" | b"brdrbar" | b"brdrbtw" => {
                self.para().border = Some(b'o');
            }
            b"shading" | b"cbpat" | b"cfpat" if !self.para().in_table => {
                if value > 0 {
                    self.para().other_borders = true;
                }
            }
            b"brdrs" | b"brdrth" | b"brdrsh" | b"brdrdb" | b"brdrdot" | b"brdrdash"
            | b"brdrhair" | b"brdrtriple" | b"brdrthtnsg" | b"brdrtnthsg" | b"brdrtnthtnsg"
            | b"brdrw" | b"brdrcf" | b"brsp" | b"brdrnone" | b"brdrinset" | b"brdroutset"
            | b"brdrdashsm" | b"brdrdashd" | b"brdrdashdd" | b"brdrwavy" | b"brdremboss"
            | b"brdrengrave" | b"brdrframe" => {
                self.border_word(word, value);
            }
            b"intbl" => self.para().in_table = true,
            b"itap" => {
                self.para().nest = value;
                self.para().in_table = value > 0;
            }
            b"ls" => self.para().list = Some(value),
            b"ilvl" => self.para().level = value.clamp(0, 8),
            // Positioned paragraphs (Word 2's frames).
            b"posx" | b"posxc" | b"posxr" | b"posxi" | b"posxo" | b"posxl" | b"posy" | b"posyt"
            | b"posyc" | b"posyb" | b"posyil" | b"posyin" | b"absw" | b"absh" | b"phpg"
            | b"phmrg" | b"phcol" | b"pvpg" | b"pvmrg" | b"pvpara" | b"nowrap" | b"dxfrtext" => {
                self.frame_word(word, value)
            }
            // Text.
            b"par" => self.end_paragraph(),
            b"line" => self.inline(Inline::LineBreak),
            b"page" => self.inline(Inline::PageBreak),
            b"column" => self.inline(Inline::ColumnBreak),
            b"sect" => self.end_section(),
            b"tab" => self.tab(),
            b"cell" | b"nestcell" => {
                if word == b"nestcell" || self.para().nest > 1 {
                    self.text('\t');
                } else {
                    self.end_cell();
                }
            }
            b"row" => self.end_row(),
            b"nestrow" => {
                self.findings.count(FindingKind::NestedTable);
                self.end_paragraph();
            }
            b"emdash" => self.text('\u{2014}'),
            b"endash" => self.text('\u{2013}'),
            b"emspace" | b"enspace" | b"qmspace" => self.text(' '),
            b"bullet" => self.text('\u{2022}'),
            b"lquote" => self.text('\u{2018}'),
            b"rquote" => self.text('\u{2019}'),
            b"ldblquote" => self.text('\u{201C}'),
            b"rdblquote" => self.text('\u{201D}'),
            b"chpgn" => self.inline(Inline::Field(Field {
                kind: FieldKind::Page {
                    format: NumberFormat::Decimal,
                },
                result: "1".into(),
                props: CharProps::default(),
            })),
            b"chdate" => self.inline(Inline::Field(Field {
                kind: FieldKind::Date {
                    source: DateSource::Now,
                    picture: "MM/dd/yy".into(),
                    upper: false,
                },
                result: String::new(),
                props: CharProps::default(),
            })),
            b"chtime" => self.inline(Inline::Field(Field {
                kind: FieldKind::Time {
                    source: DateSource::Now,
                },
                result: String::new(),
                props: CharProps::default(),
            })),
            b"ftnalt" => {
                if let Some((StoryKind::Footnote { endnote, .. }, _)) = self.stories.last_mut() {
                    *endnote = true;
                }
            }
            // Tables.
            b"trowd" => {
                self.row = RowProps {
                    gap: 108,
                    ..RowProps::default()
                };
                self.cell_props = CellProps::default();
            }
            b"trgaph" => self.row.gap = value.max(0),
            b"trleft" => self.row.left = value,
            b"trrh" => self.row.height = value,
            b"trhdr" => self.row.heading = on,
            b"trkeep" => self.row.keep = on,
            b"trqc" => self.row.centred = true,
            b"trpaddl" => self.row.padding[3] = Some(value.max(0)),
            b"trpaddr" => self.row.padding[1] = Some(value.max(0)),
            b"trpaddt" => self.row.padding[0] = Some(value.max(0)),
            b"trpaddb" => self.row.padding[2] = Some(value.max(0)),
            b"clmgf" => self.cell_props.merge_first = true,
            b"clmrg" => self.cell_props.merged = true,
            b"clvmgf" => self.cell_props.vertical_first = true,
            b"clvmrg" => self.cell_props.vertical_merged = true,
            b"clbrdrt" => self.para().border = Some(b'T'),
            b"clbrdrr" => self.para().border = Some(b'R'),
            b"clbrdrb" => self.para().border = Some(b'B'),
            b"clbrdrl" => self.para().border = Some(b'L'),
            b"clcbpat" | b"clcbpatraw" => self.cell_props.shading = value.max(0) as usize,
            b"cellx" => {
                let mut cell = std::mem::take(&mut self.cell_props);
                cell.right = value;
                self.row.cells.push(cell);
            }
            _ => {}
        }
    }

    /// Words of the document's page and of sections.
    fn page_word(&mut self, word: &[u8], value: i32) -> bool {
        let section = &mut self.section;
        let page = &mut section.page;
        match word {
            b"paperw" => {
                self.document_page.width = value;
                page.width = value;
            }
            b"paperh" => {
                self.document_page.height = value;
                page.height = value;
            }
            b"margl" => {
                self.document_page.margin_left = value;
                page.margin_left = value;
            }
            b"margr" => {
                self.document_page.margin_right = value;
                page.margin_right = value;
            }
            b"margt" => {
                self.document_page.margin_top = value;
                page.margin_top = value;
            }
            b"margb" => {
                self.document_page.margin_bottom = value;
                page.margin_bottom = value;
            }
            b"landscape" => {
                self.document_page.landscape = true;
                page.landscape = true;
            }
            b"margmirror" => self.settings.mirror_margins = true,
            b"facingp" => self.settings.even_and_odd_headers = true,
            b"hyphauto" => self.settings.hyphenation = value != 0,
            b"hyphhotz" => self.settings.hyphenation_zone = value.max(0),
            b"sectd" => {
                let new_page = section.new_page;
                *page = self.document_page.clone();
                section.title_page = false;
                section.new_page = new_page;
            }
            b"pgwsxn" => page.width = value,
            b"pghsxn" => page.height = value,
            b"marglsxn" => page.margin_left = value,
            b"margrsxn" => page.margin_right = value,
            b"margtsxn" => page.margin_top = value,
            b"margbsxn" => page.margin_bottom = value,
            b"lndscpsxn" => page.landscape = true,
            b"headery" => page.header_distance = value.max(0),
            b"footery" => page.footer_distance = value.max(0),
            b"cols" => page.columns.count = value.clamp(1, 8) as u8,
            b"colsx" => page.columns.gap = value.max(0),
            b"linebetcol" => page.columns.line_between = true,
            b"colw" => {
                let widths = page.columns.widths.get_or_insert_with(Vec::new);
                widths.push((value.max(0), 0));
            }
            b"colsr" => {
                if let Some(last) = page.columns.widths.as_mut().and_then(|w| w.last_mut()) {
                    last.1 = value.max(0);
                }
            }
            b"titlepg" => section.title_page = true,
            b"sbknone" => section.new_page = false,
            b"sbkpage" | b"sbkodd" | b"sbkeven" | b"sbkcol" => section.new_page = true,
            b"pgnstarts" => section.page_start = Some(value.max(0) as u32),
            b"pgnrestart" => section.restart = true,
            b"pgndec" => section.page_format = Some(NumberFormat::Decimal),
            b"pgnucrm" => section.page_format = Some(NumberFormat::UpperRoman),
            b"pgnlcrm" => section.page_format = Some(NumberFormat::LowerRoman),
            b"pgnucltr" => section.page_format = Some(NumberFormat::UpperLetter),
            b"pgnlcltr" => section.page_format = Some(NumberFormat::LowerLetter),
            _ => return false,
        }
        true
    }

    fn border_word(&mut self, word: &[u8], value: i32) {
        let side = self.para().border;
        let Some(side) = side else { return };
        if side == b'o' {
            self.para().other_borders = true;
            return;
        }
        if side == b'p' {
            let colour = self.colours.get(value.max(0) as usize).copied().flatten();
            if let Some(border) = &mut self.section.page.border {
                match word {
                    b"brdrdb" => border.line.kind = LineKind::Double,
                    b"brdrtriple" => border.line.kind = LineKind::Triple,
                    b"brdrthtnsg" => border.line.kind = LineKind::ThickThin,
                    b"brdrtnthsg" => border.line.kind = LineKind::ThinThick,
                    b"brdrth" => border.line.space = -1,
                    b"brdrw" => {
                        let thick = border.line.space == -1;
                        border.line.width = value.max(1) * if thick { 2 } else { 1 };
                        border.line.space = 0;
                    }
                    b"brdrcf" => border.line.colour = colour.unwrap_or(0),
                    // Points from the paper's edge (`\pgbrdropt32`).
                    b"brsp" => border.distance = value.max(0) * 20,
                    _ => {}
                }
            }
            return;
        }
        let colours = self.colours.clone();
        let line = match side {
            b't' => &mut self.para().border_top,
            b'b' => &mut self.para().border_bottom,
            b'T' => &mut self.cell_props.borders[0],
            b'R' => &mut self.cell_props.borders[1],
            b'B' => &mut self.cell_props.borders[2],
            b'L' => &mut self.cell_props.borders[3],
            _ => return,
        };
        if word == b"brdrnone" {
            *line = None;
            return;
        }
        let entry = line.get_or_insert(RuleLine {
            kind: LineKind::Single,
            width: 15,
            colour: 0,
            space: 0,
        });
        match word {
            b"brdrdb" => entry.kind = LineKind::Double,
            b"brdrtriple" => entry.kind = LineKind::Triple,
            b"brdrthtnsg" => entry.kind = LineKind::ThickThin,
            b"brdrtnthsg" => entry.kind = LineKind::ThinThick,
            // A thick line is twice its width.
            b"brdrth" => {
                entry.kind = LineKind::Single;
                entry.space = -1;
            }
            b"brdrw" => {
                let thick = entry.space == -1;
                entry.width = value.max(1) * if thick { 2 } else { 1 };
                if thick {
                    entry.space = 0;
                }
            }
            b"brdrcf" => {
                entry.colour = colours
                    .get(value.max(0) as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(0);
            }
            b"brsp" => entry.space = value.max(0),
            _ => {}
        }
    }

    fn frame_word(&mut self, word: &[u8], value: i32) {
        let frame = self.para().frame.get_or_insert_with(FrameProps::default);
        match word {
            b"posx" => frame.x = value,
            b"posy" => frame.y = value,
            b"absw" => frame.width = value,
            b"absh" => frame.height = value,
            b"phpg" => frame.horizontal = Some(HorizontalFrom::Page),
            b"phmrg" => frame.horizontal = Some(HorizontalFrom::Margin),
            b"phcol" => frame.horizontal = Some(HorizontalFrom::Column),
            b"pvpg" => frame.vertical = Some(VerticalFrom::Page),
            b"pvmrg" => frame.vertical = Some(VerticalFrom::Margin),
            b"pvpara" => frame.vertical = Some(VerticalFrom::Paragraph),
            b"nowrap" => frame.no_wrap = true,
            _ => {}
        }
    }

    fn symbol(&mut self, symbol: u8) {
        match symbol {
            b'\\' | b'{' | b'}' => self.text(char::from(symbol)),
            b'~' => self.text('\u{A0}'),
            b'-' => self.text('\u{AD}'),
            b'_' => self.text('\u{2011}'),
            b'\t' => self.tab(),
            b'|' | b':' => {}
            _ => {}
        }
    }

    // ---- Text ----

    /// A character in the current destination.
    fn text(&mut self, character: char) {
        match self.dest() {
            Dest::Text => {}
            Dest::FontTable => {
                if character != ';' {
                    self.font_name.push(character);
                } else {
                    self.end_font();
                }
                return;
            }
            Dest::ColorTable => {
                if character == ';' {
                    let (red, green, blue, set) = self.colour;
                    self.colours.push(
                        set.then_some(
                            u32::from(red) | u32::from(green) << 8 | u32::from(blue) << 16,
                        ),
                    );
                    self.colour = (0, 0, 0, false);
                }
                return;
            }
            Dest::StyleEntry => {
                self.style.name.push(character);
                return;
            }
            Dest::InfoText(_) | Dest::StaticValue => {
                self.info_text.push(character);
                return;
            }
            Dest::PropName => {
                self.prop_name.push(character);
                return;
            }
            Dest::FieldInstruction => {
                if let Some(field) = self.fields.last_mut() {
                    field.instruction.push(character);
                }
                return;
            }
            Dest::ShapeName => {
                if let Some(shape) = self.shapes.last_mut() {
                    shape.name.push(character);
                }
                return;
            }
            Dest::ShapeValue => {
                if let Some(shape) = self.shapes.last_mut() {
                    shape.value.push(character);
                }
                return;
            }
            Dest::LevelText => {
                // Bytes: a length, then text with level numbers 0 to 8.
                let mut buffer = [0u8; 4];
                self.level_text
                    .extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                return;
            }
            Dest::ListText => {
                if let Some(text) = &mut self.story().number_text {
                    text.push(character);
                }
                return;
            }
            Dest::OldNumberingText(after) => {
                if let Some(old) = &mut self.old {
                    if after {
                        old.2.push(character);
                    } else {
                        old.1.push(character);
                    }
                }
                return;
            }
            Dest::AnnotationId => {
                self.annotation.0.push(character);
                return;
            }
            Dest::Generator => {
                self.generator.push(character);
                return;
            }
            Dest::AnnotationAuthor => {
                self.annotation.1.push(character);
                return;
            }
            Dest::BookmarkStart | Dest::BookmarkEnd => {
                self.bookmark_name.push(character);
                return;
            }
            Dest::AnnotationDate => {
                self.annotation_date.push(character);
                return;
            }
            _ => return,
        }
        let chars = self.groups.last().expect("a group").chars.clone();
        if chars.hidden || chars.deleted {
            self.note_run(&chars);
            return;
        }
        self.note_run(&chars);
        let props = self.char_props(&chars);
        let story = self.story();
        if !std::mem::take(&mut story.split)
            && let Some(Inline::Text { text, props: last }) = story.inlines.last_mut()
            && *last == props
        {
            text.push(character);
            return;
        }
        story.inlines.push(Inline::Text {
            text: character.to_string(),
            props,
        });
    }

    /// Counts runs of hidden, deleted, inserted, highlighted, and spaced
    /// text as they start.
    fn note_run(&mut self, chars: &CharState) {
        let now = (
            chars.hidden,
            chars.deleted,
            chars.inserted,
            chars.highlight,
            chars.spacing,
        );
        let before = self.run;
        if now.0 && !before.0 {
            self.findings.count(FindingKind::HiddenText);
        }
        if now.1 && !before.1 {
            self.findings.count(FindingKind::Deletions);
        }
        if now.2 && !before.2 {
            self.findings.count(FindingKind::Insertions);
        }
        if now.3 && !before.3 {
            self.findings.count(FindingKind::Highlight);
        }
        if now.4 && !before.4 {
            self.findings.count(FindingKind::CharacterSpacing);
        }
        self.run = now;
    }

    fn tab(&mut self) {
        if self.dest() != Dest::Text {
            return;
        }
        let chars = self.groups.last().expect("a group").chars.clone();
        if chars.hidden || chars.deleted {
            return;
        }
        let props = self.char_props(&chars);
        self.story().inlines.push(Inline::Tab { props });
    }

    fn inline(&mut self, inline: Inline) {
        if self.dest() == Dest::Text {
            self.story().inlines.push(inline);
        }
    }

    /// The character formatting of `state`, every field set.
    fn char_props(&self, state: &CharState) -> CharProps {
        let font = state.font.unwrap_or(self.default_font);
        CharProps {
            font: self.font_name_of(font),
            size: Some(state.size * 10),
            bold: Some(state.bold),
            italic: Some(state.italic),
            underline: Some(state.underline),
            strike: Some(state.strike),
            script: Some(state.script),
            caps: Some(state.caps),
            small_caps: Some(state.small_caps),
            colour: Some(
                self.colours
                    .get(state.colour)
                    .copied()
                    .flatten()
                    .unwrap_or(0),
            ),
        }
    }

    fn font_name_of(&self, number: i32) -> Option<String> {
        self.fonts
            .iter()
            .find(|(known, _, _)| *known == number)
            .map(|(_, font, _)| font.name.clone())
    }

    fn end_font(&mut self) {
        let name = std::mem::take(&mut self.font_name).trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.fonts
            .retain(|(number, _, _)| *number != self.font_number);
        self.fonts.push((
            self.font_number,
            Font {
                name,
                family: std::mem::replace(&mut self.font_family, FontFamily::Unknown),
                fixed: std::mem::take(&mut self.font_fixed),
            },
            std::mem::take(&mut self.font_charset),
        ));
    }

    /// `\leveltext`: its length byte, then text whose bytes 0 to 8 stand
    /// for the levels' numbers.
    fn end_level_text(&mut self) {
        let bytes = std::mem::take(&mut self.level_text);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let mut chars = text.chars();
        let length = chars.next().map_or(0, |first| first as usize);
        let mut out = String::new();
        for character in chars.take(length) {
            match character as u32 {
                0..=8 => out.push_str(&format!("%{}", character as u32 + 1)),
                _ if character == '%' => out.push_str("%%"),
                _ => out.push(character),
            }
        }
        if let Some(level) = &mut self.level {
            level.text = out;
        }
    }

    /// The paragraph's formatting as the exchange document's (every field
    /// set).
    fn para_props(&self, state: &ParaState) -> ParagraphProps {
        let line = state.line.and_then(|(height, multiple)| {
            if height == 0 {
                return None;
            }
            Some(if multiple {
                LineSpacing::Multiple(height.abs() as f32 / 240.0)
            } else if height < 0 {
                LineSpacing::Exact(-height)
            } else {
                LineSpacing::AtLeast(height)
            })
        });
        let mut tabs = state.tabs.clone();
        tabs.sort_by_key(|tab| tab.position);
        // Without stops of its own, the default stops (Ami Pro's own tabs
        // replace the default ones).
        let mut position = if tabs.is_empty() {
            self.tab_default
        } else {
            i32::MAX
        };
        while position <= 8_640 && tabs.len() < 22 {
            tabs.push(Tab {
                position,
                kind: TabKind::Left,
                leader: TabLeader::None,
            });
            position += self.tab_default;
        }
        ParagraphProps {
            alignment: Some(state.alignment),
            indent_left: Some(state.left),
            indent_right: Some(state.right),
            first_line: Some(state.first),
            space_before: Some(state.before),
            space_after: Some(state.after),
            line: Some(line.unwrap_or(LineSpacing::Single)),
            keep_with_next: Some(state.keep_next),
            keep_together: Some(state.keep),
            page_break_before: Some(state.page_before),
            column_break_before: Some(false),
            tabs: Some(tabs),
            border_top: Some(state.border_top),
            border_bottom: Some(state.border_bottom),
            no_hyphenation: Some(state.no_hyphenation),
        }
    }

    fn end_paragraph(&mut self) {
        if self.dest() != Dest::Text {
            return;
        }
        let state = self.groups.last().expect("a group").para.clone();
        let chars = self.groups.last().expect("a group").chars.clone();
        if state.other_borders {
            self.findings.count(FindingKind::ParagraphBorders);
        }
        let mark = self.char_props(&chars);
        let mut paragraph = Paragraph {
            style: 0,
            props: self.para_props(&state),
            list: None,
            number_text: None,
            inlines: std::mem::take(&mut self.story().inlines),
            mark,
        };
        // The style's number for now, its index at the end.
        paragraph.style = state.style.max(0) as usize;
        if let Some(list) = state.list {
            paragraph.list = Some((Some(list_key(list)), state.level as u8));
        }
        paragraph.number_text = self.story().number_text.take();
        if let Some(outline) = state.outline {
            // A paragraph's own outline level: kept in its style's list
            // place (Ami Pro's levels live in styles).
            let _ = outline;
        }
        if let Some(frame) = state.frame.clone().filter(|frame| frame.width > 0) {
            self.story().framed.push((frame, paragraph));
            return;
        }
        self.flush_framed();
        if state.in_table && state.nest <= 1 {
            self.story().cell.push(Block::Paragraph(paragraph));
            return;
        }
        if state.nest > 1 {
            self.story().cell.push(Block::Paragraph(paragraph));
            return;
        }
        self.finish_table();
        self.story().blocks.push(Block::Paragraph(paragraph));
    }

    /// Positioned paragraphs before the next ordinary one: their frame, as
    /// a shape at the next paragraph's start.
    fn flush_framed(&mut self) {
        let framed = std::mem::take(&mut self.story().framed);
        if framed.is_empty() {
            return;
        }
        let mut groups: Vec<(FrameProps, Vec<Block>)> = Vec::new();
        for (frame, paragraph) in framed {
            match groups.last_mut() {
                Some((last, blocks)) if *last == frame => blocks.push(Block::Paragraph(paragraph)),
                _ => groups.push((frame, vec![Block::Paragraph(paragraph)])),
            }
        }
        for (frame, blocks) in groups {
            let height = if frame.height != 0 {
                frame.height.abs()
            } else {
                // Grown to its text: about a line for each paragraph.
                (blocks.len() as Twips * 300).max(300)
            };
            let id = self.next_shape_id();
            let shape = Shape {
                id,
                name: String::new(),
                anchor: Anchor::Floating {
                    horizontal: (frame.horizontal.unwrap_or(HorizontalFrom::Column), frame.x),
                    vertical: (frame.vertical.unwrap_or(VerticalFrom::Paragraph), frame.y),
                    wrap: if frame.no_wrap {
                        ShapeWrap::TopAndBottom
                    } else {
                        ShapeWrap::Square
                    },
                    behind: false,
                },
                width: frame.width,
                height,
                z: id,
                line: None,
                fill: None,
                shadow: None,
                corner_radius: 0,
                content: ShapeContent::Text {
                    blocks,
                    insets: [0; 4],
                    columns: 1,
                    column_gap: 0,
                },
            };
            self.story()
                .inlines
                .insert(0, Inline::Shape(Box::new(shape)));
        }
    }

    fn next_shape_id(&mut self) -> u32 {
        let id = self.next_shape;
        self.next_shape += 1;
        id
    }

    fn end_cell(&mut self) {
        // The text before `\cell` is the cell's last paragraph.
        let pending = !self.story().inlines.is_empty() || self.story().cell.is_empty();
        if pending {
            self.groups.last_mut().expect("a group").para.in_table = true;
            self.end_paragraph();
        }
        let story = self.story();
        let mut cell = std::mem::take(&mut story.cell);
        if cell.is_empty() {
            cell.push(Block::Paragraph(Paragraph::default()));
        }
        story.cells.push(cell);
    }

    fn end_row(&mut self) {
        if !self.story().inlines.is_empty() || !self.story().cell.is_empty() {
            self.end_cell();
        }
        let row = self.row.clone();
        let story = self.story();
        let cells = std::mem::take(&mut story.cells);
        story.rows.push((row, cells));
    }

    /// The rows read so far as a table on one grid.
    fn finish_table(&mut self) {
        let colours = self.colours.clone();
        let story = self.story();
        if story.rows.is_empty() {
            return;
        }
        let rows = std::mem::take(&mut story.rows);
        // The grid: every row's cell edges.
        let mut edges: Vec<Twips> = Vec::new();
        for (props, _) in &rows {
            let mut left = props.left;
            edges.push(left);
            for cell in &props.cells {
                let right = cell.right.max(left + 1);
                edges.push(right);
                left = right;
            }
        }
        edges.sort_unstable();
        edges.dedup_by(|a, b| (*a - *b).abs() < 10);
        // Rows without cell edges: a column an inch wide for each cell.
        let most = rows
            .iter()
            .map(|(_, cells)| cells.len())
            .max()
            .unwrap_or(1)
            .max(1);
        if edges.len() < 2 {
            let left = edges.first().copied().unwrap_or(0);
            edges = (0..=most)
                .map(|column| left + column as Twips * 1_440)
                .collect();
        }
        let first = rows[0].0.clone();
        let indent = first.left;
        let gap = first.gap;
        let padding = [
            first.padding[0].unwrap_or(0),
            first.padding[1].unwrap_or(gap),
            first.padding[2].unwrap_or(0),
            first.padding[3].unwrap_or(gap),
        ];
        let grid_index = |position: Twips| {
            edges
                .iter()
                .enumerate()
                .min_by_key(|(_, edge)| (**edge - position).abs())
                .map_or(0, |(index, _)| index)
        };
        let mut table_rows = Vec::with_capacity(rows.len());
        for (mut props, mut cells) in rows {
            let mut out = Vec::new();
            if props.cells.is_empty() {
                props.cells = (1..=cells.len())
                    .map(|column| CellProps {
                        right: edges[column.min(edges.len() - 1)],
                        ..CellProps::default()
                    })
                    .collect();
            }
            let mut left = props.left;
            for (index, cell) in props.cells.iter().enumerate() {
                let start = grid_index(left);
                let right = cell.right.max(left + 1);
                let end = grid_index(right).max(start + 1);
                left = right;
                let blocks = if index < cells.len() {
                    std::mem::take(&mut cells[index])
                } else {
                    vec![Block::Paragraph(Paragraph::default())]
                };
                if cell.merged {
                    // Covered by the cell before (old horizontal merges).
                    if let Some(Cell { span, .. }) = out.last_mut() {
                        *span += (end - start) as u16;
                    }
                    continue;
                }
                out.push(Cell {
                    span: (end - start) as u16,
                    merge: if cell.vertical_first {
                        VerticalMerge::Start
                    } else if cell.vertical_merged {
                        VerticalMerge::Continue
                    } else {
                        VerticalMerge::None
                    },
                    blocks,
                    borders: cell.borders,
                    shading: (cell.shading > 0)
                        .then(|| colours.get(cell.shading).copied().flatten())
                        .flatten(),
                });
            }
            // A row starting a page: its first paragraph breaks before.
            let mut page_break = false;
            if let Some(Cell { blocks, .. }) = out.first_mut()
                && let Some(Block::Paragraph(first)) = blocks.first_mut()
                && first.props.page_break_before == Some(true)
            {
                first.props.page_break_before = Some(false);
                page_break = true;
            }
            table_rows.push(Row {
                height: if props.height < 0 {
                    RowHeight::Exact(-props.height)
                } else {
                    RowHeight::AtLeast(props.height.max(0))
                },
                heading: props.heading,
                keep_together: props.keep,
                page_break_before: page_break,
                cells: out,
            });
        }
        let columns: Vec<Twips> = edges.windows(2).map(|pair| pair[1] - pair[0]).collect();
        let table = Table {
            columns,
            rows: table_rows,
            indent: indent.max(0),
            align: if first.centred {
                Alignment::Centre
            } else {
                Alignment::Left
            },
            cell_margins: padding,
        };
        self.story().blocks.push(Block::Table(table));
    }

    fn end_story(&mut self) {
        // The text after the last paragraph end is a paragraph too.
        if !self.story().inlines.is_empty() {
            self.groups.push(Group {
                dest: Dest::Text,
                chars: CharState::default(),
                para: ParaState::default(),
                unicode_skip: 1,
            });
            self.end_paragraph();
            self.groups.pop();
        }
        self.flush_framed();
        self.finish_table();
        let Some((kind, story)) = self.stories.pop() else {
            return;
        };
        if self.stories.is_empty() {
            self.stories.push((kind, story));
            return;
        }
        let blocks = story.blocks;
        match kind {
            StoryKind::Main => {}
            StoryKind::Header(part) | StoryKind::Footer(part) => {
                let set = if matches!(kind, StoryKind::Header(_)) {
                    &mut self.section.headers
                } else {
                    &mut self.section.footers
                };
                match part {
                    "even" => set.even = Some(blocks),
                    "first" => set.first = Some(blocks),
                    _ => set.default = Some(blocks),
                }
            }
            StoryKind::Footnote { endnote, props } => {
                self.inline(Inline::Footnote(Box::new(Footnote {
                    endnote,
                    blocks,
                    props,
                })));
            }
            StoryKind::Annotation => {
                let (initials, author) = std::mem::take(&mut self.annotation);
                let date = std::mem::take(&mut self.annotation_date)
                    .trim()
                    .parse::<i64>()
                    .ok()
                    .and_then(dttm);
                let marker = self.findings.at(FindingKind::Comments);
                self.inline(Inline::Marker(marker));
                self.inline(Inline::Note(Box::new(Note {
                    author: author.trim().to_owned(),
                    initials: initials.trim().to_owned(),
                    date,
                    blocks,
                })));
            }
            StoryKind::ShapeText => {
                if let Some(shape) = self.shapes.last_mut() {
                    shape.text = Some(blocks);
                }
            }
        }
    }

    fn end_picture(&mut self) {
        let Some(picture) = self.picture.take() else {
            return;
        };
        let Some((image, width, height)) = self.image(picture) else {
            return;
        };
        // In a shape, the shape's picture; otherwise a picture in the line.
        if let Some(shape) = self.shapes.last_mut()
            && self
                .groups
                .iter()
                .any(|group| group.dest == Dest::ShapeValue)
        {
            shape.picture = Some((image, width, height));
            return;
        }
        let id = self.next_shape_id();
        self.inline(Inline::Shape(Box::new(Shape {
            id,
            name: String::new(),
            anchor: Anchor::Inline { drop: 0 },
            width,
            height,
            z: id,
            line: None,
            fill: None,
            shadow: None,
            corner_radius: 0,
            content: ShapeContent::Picture(PlacedPicture {
                image,
                left: 0,
                top: 0,
                width,
                height,
                description: String::new(),
            }),
        })));
    }

    /// A picture's bytes as an image, and its size in twips.
    fn image(&mut self, picture: PictureData) -> Option<(usize, Twips, Twips)> {
        let scale = |goal: i32, size: i32, scale: i32| {
            let base = if goal > 0 { goal } else { size * 15 };
            (base.max(1) * scale.max(1) / 100).max(1)
        };
        let (width, height) = (
            scale(picture.goal_width, picture.width, picture.scale_x),
            scale(picture.goal_height, picture.height, picture.scale_y),
        );
        let data = picture.data;
        let (kind, bytes) = match picture.kind? {
            "png" => (ImageKind::Png, data),
            "jpeg" => (ImageKind::Jpeg, data),
            "wmf" => (ImageKind::Wmf, placeable(&data, width, height)),
            "emf" => (ImageKind::Emf, data),
            "dib" => (ImageKind::Bmp, bitmap_file(&data)?),
            other => {
                self.findings.count(FindingKind::Picture(other.to_owned()));
                return None;
            }
        };
        if bytes.is_empty() {
            return None;
        }
        let _ = picture.crop;
        self.images.push(Image {
            bytes: Arc::from(bytes),
            kind,
            fallback: None,
            pixels: None,
        });
        Some((self.images.len() - 1, width, height))
    }

    fn end_shape(&mut self) {
        let Some(shape) = self.shapes.pop() else {
            return;
        };
        let property = |name: &str| shape.properties.get(name).map(String::as_str);
        let number = |name: &str| property(name).and_then(|value| value.parse::<i64>().ok());
        let flag = |name: &str, default: bool| number(name).map_or(default, |value| value != 0);
        let width = (shape.right - shape.left).max(20);
        let height = (shape.bottom - shape.top).max(20);
        // Word's newer files say where in `posrelh` and `posrelv`.
        let horizontal = match number("posrelh") {
            Some(1) => HorizontalFrom::Page,
            Some(2) => HorizontalFrom::Column,
            Some(_) => HorizontalFrom::Margin,
            None => shape.horizontal.unwrap_or(HorizontalFrom::Column),
        };
        let vertical = match number("posrelv") {
            Some(1) => VerticalFrom::Page,
            Some(2) | Some(3) => VerticalFrom::Paragraph,
            Some(_) => VerticalFrom::Margin,
            None => shape.vertical.unwrap_or(VerticalFrom::Paragraph),
        };
        let wrap = match shape.wrap {
            1 => ShapeWrap::TopAndBottom,
            3 => ShapeWrap::None,
            _ => ShapeWrap::Square,
        };
        let kind = number("shapeType").unwrap_or(202);
        let filled = flag("fFilled", kind != 75);
        let fill = filled.then(|| colour_of(number("fillColor").unwrap_or(0xFF_FFFF)));
        let lined = flag("fLine", kind != 75 && kind != 202);
        let line = lined.then(|| ShapeLine {
            kind: match number("lineStyle") {
                Some(1) => LineKind::Double,
                Some(2) => LineKind::ThickThin,
                Some(3) => LineKind::ThinThick,
                Some(4) => LineKind::Triple,
                _ => LineKind::Single,
            },
            width: (number("lineWidth").unwrap_or(9_525) / 635).max(1) as Twips,
            colour: colour_of(number("lineColor").unwrap_or(0)),
            sides: [true; 4],
        });
        let shadow = flag("fShadow", false).then(|| ShapeShadow {
            dx: (number("shadowOffsetX").unwrap_or(25_400) / 635) as Twips,
            dy: (number("shadowOffsetY").unwrap_or(25_400) / 635) as Twips,
            colour: colour_of(number("shadowColor").unwrap_or(0x80_8080)),
        });
        let corner_radius = if kind == 2 {
            let adjust = number("adjustValue").unwrap_or(3_600);
            (adjust * i64::from(width.min(height)) / 21_600) as Twips
        } else {
            0
        };
        let inset = |name: &str, default: i64| (number(name).unwrap_or(default) / 635) as Twips;
        let content = match (shape.text, shape.picture) {
            (_, Some((image, _, _))) => ShapeContent::Picture(PlacedPicture {
                image,
                left: 0,
                top: 0,
                width,
                height,
                description: property("wzDescription").unwrap_or("").to_owned(),
            }),
            (Some(blocks), None) => ShapeContent::Text {
                blocks,
                insets: [
                    inset("dyTextTop", 45_720),
                    inset("dxTextRight", 91_440),
                    inset("dyTextBottom", 45_720),
                    inset("dxTextLeft", 91_440),
                ],
                columns: 1,
                column_gap: 0,
            },
            (None, None) => {
                if !matches!(kind, 1 | 2 | 202) {
                    self.findings.count(FindingKind::Shape("drawn".into()));
                    return;
                }
                ShapeContent::Empty
            }
        };
        let id = self.next_shape_id();
        self.inline(Inline::Shape(Box::new(Shape {
            id,
            name: property("wzName").unwrap_or("").to_owned(),
            anchor: Anchor::Floating {
                horizontal: (horizontal, shape.left),
                vertical: (vertical, shape.top),
                wrap,
                behind: shape.behind,
            },
            width,
            height,
            z: shape.z.max(0) as u32,
            line,
            fill,
            shadow,
            corner_radius,
            content,
        })));
    }

    fn end_field(&mut self) {
        let Some(field) = self.fields.pop() else {
            return;
        };
        let instruction = field.instruction.trim().to_owned();
        let mut words = split_instruction(&instruction);
        let name = words
            .first()
            .map(|word| word.to_ascii_uppercase())
            .unwrap_or_default();
        let Some((depth, start)) = field.result_at else {
            // No result: a field Ami Pro cannot show but by its kind.
            if name == "XE" || name == "TC" {
                self.findings.count(FindingKind::GeneratedText);
            }
            return;
        };
        if depth != self.stories.len() {
            return;
        }
        let story = &mut self.stories.last_mut().expect("the main story").1;
        let result: Vec<Inline> = if start <= story.inlines.len() {
            story.inlines.split_off(start)
        } else {
            Vec::new()
        };
        let text: String = result
            .iter()
            .map(|inline| match inline {
                Inline::Text { text, .. } => text.as_str(),
                _ => "",
            })
            .collect();
        let props = result
            .iter()
            .find_map(|inline| match inline {
                Inline::Text { props, .. } => Some(props.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let picture = words
            .iter()
            .position(|word| word == "\\@")
            .and_then(|at| words.get(at + 1))
            .cloned()
            .unwrap_or_else(|| "MM/dd/yy".into());
        let upper = instruction.contains("\\* Upper") || instruction.contains("\\*Upper");
        let format = if instruction.contains("ROMAN") {
            NumberFormat::UpperRoman
        } else if instruction.contains("roman") {
            NumberFormat::LowerRoman
        } else if instruction.contains("ALPHABETIC") {
            NumberFormat::UpperLetter
        } else if instruction.contains("alphabetic") {
            NumberFormat::LowerLetter
        } else {
            NumberFormat::Decimal
        };
        let argument = words.get(1).cloned().unwrap_or_default();
        let kind = match name.as_str() {
            "PAGE" => Some(FieldKind::Page { format }),
            "NUMPAGES" | "SECTIONPAGES" => Some(FieldKind::NumPages),
            "DATE" | "PRINTDATE" => Some(if is_time(&picture) {
                FieldKind::Time {
                    source: DateSource::Now,
                }
            } else {
                FieldKind::Date {
                    source: DateSource::Now,
                    picture: date_picture(&picture),
                    upper,
                }
            }),
            "TIME" => Some(FieldKind::Time {
                source: DateSource::Now,
            }),
            "CREATEDATE" | "SAVEDATE" => {
                let source = if name == "CREATEDATE" {
                    DateSource::Created
                } else {
                    DateSource::Saved
                };
                Some(if is_time(&picture) {
                    FieldKind::Time { source }
                } else {
                    FieldKind::Date {
                        source,
                        picture: date_picture(&picture),
                        upper,
                    }
                })
            }
            "TITLE" => Some(FieldKind::Title),
            "COMMENTS" | "SUBJECT" => Some(FieldKind::Description),
            "KEYWORDS" => Some(FieldKind::Keywords),
            "FILENAME" => Some(FieldKind::FileName {
                path: instruction.contains("\\p"),
            }),
            "TEMPLATE" => Some(FieldKind::Template),
            "REVNUM" => Some(FieldKind::Revisions),
            "EDITTIME" => Some(FieldKind::EditingTime),
            "NUMWORDS" => Some(FieldKind::Words),
            "NUMCHARS" => Some(FieldKind::Characters),
            "DOCPROPERTY" => Some(FieldKind::Custom(argument.trim_matches('"').to_owned())),
            "SEQ" => Some(FieldKind::Seq {
                name: argument.clone(),
            }),
            _ => None,
        };
        let story = &mut self.stories.last_mut().expect("the main story").1;
        match kind {
            Some(kind) => story.inlines.push(Inline::Field(Field {
                kind,
                result: text,
                props,
            })),
            None => {
                // The result as it stands (a link's text, a contents list,
                // a cross reference's number).
                story.inlines.extend(result);
                let found = match name.as_str() {
                    "HYPERLINK" => Some(FindingKind::Links),
                    "TOC" | "INDEX" | "TOA" => Some(FindingKind::GeneratedText),
                    "SYMBOL" => None,
                    "" => None,
                    "FORMTEXT" | "FORMCHECKBOX" | "FORMDROPDOWN" => Some(FindingKind::FormFields),
                    "INCLUDEPICTURE" | "EMBED" | "LINK" => Some(FindingKind::Objects),
                    other => Some(FindingKind::Field(other.to_owned())),
                };
                if let Some(found) = found {
                    let marker = self.findings.at(found);
                    let story = &mut self.stories.last_mut().expect("the main story").1;
                    story
                        .inlines
                        .insert(start.min(story.inlines.len()), Inline::Marker(marker));
                }
            }
        }
        words.clear();
    }

    /// A section ends: its blocks and set-up.
    fn end_section(&mut self) {
        if self.stories.len() != 1 {
            return;
        }
        if !self.story().inlines.is_empty() {
            self.end_paragraph();
        }
        self.flush_framed();
        self.finish_table();
        let blocks = std::mem::take(&mut self.story().blocks);
        let section = std::mem::replace(
            &mut self.section,
            SectionProps {
                page: self.document_page.clone(),
                new_page: true,
                title_page: false,
                page_start: None,
                page_format: None,
                restart: false,
                headers: HeaderFooters::default(),
                footers: HeaderFooters::default(),
            },
        );
        // Headers go on into the next section unless it has its own.
        self.section.headers = section.headers.clone();
        self.section.footers = section.footers.clone();
        self.sections.push(made_section(section, blocks));
    }

    fn finish(mut self) -> ExchangeDocument {
        // The main story's last paragraph and section.
        while self.stories.len() > 1 {
            self.end_story();
        }
        if !self.story().inlines.is_empty() {
            self.groups.push(Group {
                dest: Dest::Text,
                chars: CharState::default(),
                para: ParaState::default(),
                unicode_skip: 1,
            });
            self.end_paragraph();
            self.groups.pop();
        }
        self.flush_framed();
        self.finish_table();
        let blocks = std::mem::take(&mut self.story().blocks);
        let section = std::mem::replace(
            &mut self.section,
            SectionProps {
                page: default_page(),
                new_page: true,
                title_page: false,
                page_start: None,
                page_format: None,
                restart: false,
                headers: HeaderFooters::default(),
                footers: HeaderFooters::default(),
            },
        );
        if !blocks.is_empty() || self.sections.is_empty() {
            self.sections.push(made_section(section, blocks));
        }
        // Styles: the default (\s0) first, paragraph styles in order.
        let mut order: Vec<StyleDef> = Vec::new();
        let default = self
            .styles
            .iter()
            .find(|style| style.number == 0 && !style.character_style)
            .cloned()
            .unwrap_or_else(|| StyleDef {
                number: 0,
                name: "Normal".into(),
                ..StyleDef::default()
            });
        order.push(default);
        for style in &self.styles {
            if !style.character_style && style.number != 0 {
                order.push(style.clone());
            }
        }
        let index_of: BTreeMap<i32, usize> = order
            .iter()
            .enumerate()
            .map(|(index, style)| (style.number, index))
            .collect();
        let list_of = self.list_instances();
        let styles: Vec<Style> = order
            .iter()
            .map(|style| Style {
                name: style.name.clone(),
                based_on: style
                    .based_on
                    .and_then(|number| index_of.get(&number).copied()),
                next: style.next.and_then(|number| index_of.get(&number).copied()),
                paragraph: self.para_props(&style.para),
                character: self.char_props(&style.chars),
                outline_level: style.para.outline,
                list: style.para.list.and_then(|ls| {
                    list_of
                        .get(&ls)
                        .map(|(list, _)| (*list, style.para.level as u8))
                }),
            })
            .collect();
        let (lists, instances) = self.lists_and_instances(&list_of);
        let mut document = ExchangeDocument {
            properties: std::mem::take(&mut self.properties),
            fonts: self.fonts.iter().map(|(_, font, _)| font.clone()).collect(),
            styles,
            lists,
            list_instances: instances,
            sections: std::mem::take(&mut self.sections),
            images: std::mem::take(&mut self.images),
            settings: Settings {
                ami_pro_rows: self.generator.trim().starts_with(crate::GENERATOR),
                ..self.settings.clone()
            },
        };
        // Paragraphs: their styles' numbers as indices, their formatting
        // beyond their styles'.
        let styles = document.styles.clone();
        let lists = list_of;
        for section in &mut document.sections {
            relate_blocks(&mut section.blocks, &styles, &index_of, &lists);
            for set in [&mut section.headers, &mut section.footers] {
                for part in [&mut set.default, &mut set.even, &mut set.first]
                    .into_iter()
                    .flatten()
                {
                    relate_blocks(part, &styles, &index_of, &lists);
                }
            }
        }
        document
    }

    /// Each `\ls` number's list and instance (old numbering's below zero).
    fn list_instances(&self) -> BTreeMap<i32, (usize, usize)> {
        let mut out = BTreeMap::new();
        let mut instance = 0;
        for (listid, ls) in &self.overrides {
            if let Some(list) = self.lists.iter().position(|(id, _)| id == listid) {
                out.insert(*ls, (list, instance));
                instance += 1;
            }
        }
        for index in 0..self.old_lists.len() {
            out.insert(-(index as i32) - 1, (self.lists.len() + index, instance));
            instance += 1;
        }
        out
    }

    fn lists_and_instances(
        &self,
        map: &BTreeMap<i32, (usize, usize)>,
    ) -> (Vec<List>, Vec<ListInstance>) {
        let mut lists: Vec<List> = self.lists.iter().map(|(_, list)| list.clone()).collect();
        for (format, before, after, start) in &self.old_lists {
            let text = if *format == NumberFormat::Bullet {
                if before.is_empty() {
                    "\u{2022}".to_owned()
                } else {
                    before.clone()
                }
            } else {
                format!("{before}%1{after}")
            };
            lists.push(List {
                levels: vec![ListLevel {
                    format: *format,
                    text,
                    start: *start,
                    restart_after_higher: true,
                    font: None,
                    props: CharProps::default(),
                    right_align: false,
                    indent_left: 360,
                    hanging: 360,
                }],
            });
        }
        let mut instances: Vec<(usize, ListInstance)> = map
            .values()
            .map(|(list, instance)| {
                (
                    *instance,
                    ListInstance {
                        list: *list,
                        starts: [None; 9],
                    },
                )
            })
            .collect();
        instances.sort_by_key(|(instance, _)| *instance);
        (
            lists,
            instances
                .into_iter()
                .map(|(_, instance)| instance)
                .collect(),
        )
    }
}

/// A list's key as paragraphs carry it until the end: the `\ls` number.
fn list_key(ls: i32) -> usize {
    (ls as i64 + (1 << 20)) as usize
}

/// Paragraphs' style numbers as indices into `styles`, their formatting
/// reduced to what differs from their style, their lists to instances.
fn relate_blocks(
    blocks: &mut [Block],
    styles: &[Style],
    index_of: &BTreeMap<i32, usize>,
    lists: &BTreeMap<i32, (usize, usize)>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                let style_index = index_of
                    .get(&(paragraph.style as i32))
                    .copied()
                    .unwrap_or(0);
                paragraph.style = style_index;
                let style = &styles[style_index];
                paragraph.props = difference(&paragraph.props, &style.paragraph);
                for inline in &mut paragraph.inlines {
                    match inline {
                        Inline::Text { props, .. } | Inline::Tab { props } => {
                            *props = char_difference(props, &style.character);
                        }
                        Inline::Field(field) => {
                            field.props = char_difference(&field.props, &style.character);
                        }
                        Inline::Shape(shape) => {
                            if let ShapeContent::Text { blocks, .. } = &mut shape.content {
                                relate_blocks(blocks, styles, index_of, lists);
                            }
                        }
                        Inline::Footnote(footnote) => {
                            relate_blocks(&mut footnote.blocks, styles, index_of, lists);
                            footnote.props = char_difference(&footnote.props, &style.character);
                        }
                        Inline::Note(note) => {
                            relate_blocks(&mut note.blocks, styles, index_of, lists);
                        }
                        _ => {}
                    }
                }
                paragraph.mark = char_difference(&paragraph.mark, &style.character);
                paragraph.list = paragraph.list.and_then(|(key, level)| {
                    let ls = key? as i64 - (1 << 20);
                    lists
                        .get(&(ls as i32))
                        .map(|(_, instance)| (Some(*instance), level))
                });
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        relate_blocks(&mut cell.blocks, styles, index_of, lists);
                    }
                }
            }
        }
    }
}

/// What `props` sets beyond `style`.
fn difference(props: &ParagraphProps, style: &ParagraphProps) -> ParagraphProps {
    fn keep<T: PartialEq + Clone>(value: &Option<T>, base: &Option<T>) -> Option<T> {
        (value != base).then(|| value.clone()).flatten()
    }
    ParagraphProps {
        alignment: keep(&props.alignment, &style.alignment),
        indent_left: keep(&props.indent_left, &style.indent_left),
        indent_right: keep(&props.indent_right, &style.indent_right),
        first_line: keep(&props.first_line, &style.first_line),
        space_before: keep(&props.space_before, &style.space_before),
        space_after: keep(&props.space_after, &style.space_after),
        line: keep(&props.line, &style.line),
        keep_with_next: keep(&props.keep_with_next, &style.keep_with_next),
        keep_together: keep(&props.keep_together, &style.keep_together),
        page_break_before: keep(&props.page_break_before, &style.page_break_before),
        column_break_before: None,
        tabs: keep(&props.tabs, &style.tabs),
        border_top: keep(&props.border_top, &style.border_top),
        border_bottom: keep(&props.border_bottom, &style.border_bottom),
        no_hyphenation: keep(&props.no_hyphenation, &style.no_hyphenation),
    }
}

fn char_difference(props: &CharProps, style: &CharProps) -> CharProps {
    fn keep<T: PartialEq + Clone>(value: &Option<T>, base: &Option<T>) -> Option<T> {
        (value != base).then(|| value.clone()).flatten()
    }
    CharProps {
        font: keep(&props.font, &style.font),
        size: keep(&props.size, &style.size),
        bold: keep(&props.bold, &style.bold),
        italic: keep(&props.italic, &style.italic),
        underline: keep(&props.underline, &style.underline),
        strike: keep(&props.strike, &style.strike),
        script: keep(&props.script, &style.script),
        caps: keep(&props.caps, &style.caps),
        small_caps: keep(&props.small_caps, &style.small_caps),
        colour: keep(&props.colour, &style.colour),
    }
}

fn made_section(section: SectionProps, mut blocks: Vec<Block>) -> Section {
    if blocks.is_empty() {
        blocks.push(Block::Paragraph(Paragraph::default()));
    }
    let mut headers = section.headers;
    let mut footers = section.footers;
    if !section.title_page {
        headers.first = None;
        footers.first = None;
    }
    let page_numbers = (section.page_format.is_some() || section.restart).then(|| PageNumbering {
        format: section.page_format.unwrap_or(NumberFormat::Decimal),
        start: section.restart.then_some(section.page_start.unwrap_or(1)),
    });
    Section {
        page: section.page,
        new_page: section.new_page,
        headers,
        footers,
        page_numbers,
        blocks,
    }
}

/// An instruction's words, quoted ones kept whole without their quotes.
fn split_instruction(instruction: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in instruction.chars() {
        match character {
            '"' => {
                if quoted {
                    words.push(std::mem::take(&mut current));
                }
                quoted = !quoted;
            }
            character if character.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether a date picture shows a time (hours and no day).
pub(crate) fn is_time(picture: &str) -> bool {
    (picture.contains('h') || picture.contains('H')) && !picture.contains('d')
}

/// A date picture as one of Ami Pro's (the nearest), so that it shows as
/// Ami Pro shows dates.
pub(crate) fn date_picture(picture: &str) -> String {
    let known = (b'a'..=b'l').map(|letter| crate::fields::date_picture(letter).0);
    for candidate in known {
        if candidate == picture {
            return candidate.to_owned();
        }
    }
    let lower = picture.to_ascii_lowercase();
    if lower.contains("dddd") {
        "dddd, MMMM d, yyyy".into()
    } else if lower.contains("mmmm") && lower.contains("yyyy") {
        "MMMM d, yyyy".into()
    } else if lower.contains("yyyy") {
        "MM/dd/yyyy".into()
    } else {
        "MM/dd/yy".into()
    }
}

/// A `COLORREF` from Word's shape colour (also `0x00BBGGRR`); system
/// colours (flag `0x08000000` and up) as black.
fn colour_of(value: i64) -> Colour {
    if !(0..=0xFF_FFFF).contains(&value) {
        return 0;
    }
    value as Colour
}

/// A metafile with Aldus's placeable header for a `width` × `height`
/// twips frame (RTF's `\wmetafile` has none).
fn placeable(data: &[u8], width: Twips, height: Twips) -> Vec<u8> {
    if data.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len() + 22);
    out.extend_from_slice(&0x9AC6_CDD7u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let (right, bottom) = (
        width.clamp(1, 32_767) as i16,
        height.clamp(1, 32_767) as i16,
    );
    for value in [0i16, 0, right, bottom] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&1440u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let words: Vec<u16> = out
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let checksum = words.iter().fold(0u16, |sum, word| sum ^ word);
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(data);
    out
}

/// A packed bitmap (`\dibitmap`) as a BMP file.
fn bitmap_file(data: &[u8]) -> Option<Vec<u8>> {
    let header = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
    let bits = u16::from_le_bytes(data.get(14..16)?.try_into().ok()?);
    let used = u32::from_le_bytes(data.get(32..36)?.try_into().ok()?);
    let palette = if bits <= 8 {
        if used > 0 { used } else { 1 << bits }
    } else {
        0
    };
    let offset = 14 + header + palette * 4;
    let mut out = Vec::with_capacity(data.len() + 14);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(data.len() as u32 + 14).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(data);
    Some(out)
}

/// The Symbol font's character for `byte`, the nearest Unicode has (the
/// Greek letters, arrows, and signs; others in the private area Symbol
/// fonts map them to).
fn symbol_character(byte: u8) -> char {
    const GREEK: &str = "ΑΒΧΔΕΦΓΗΙϑΚΛΜΝΟΠΘΡΣΤΥςΩΞΨΖ";
    const GREEK_LOWER: &str = "αβχδεφγηιϕκλμνοπθρστυϖωξψζ";
    match byte {
        b'A'..=b'Z' => GREEK.chars().nth(usize::from(byte - b'A')).unwrap_or('?'),
        b'a'..=b'z' => GREEK_LOWER
            .chars()
            .nth(usize::from(byte - b'a'))
            .unwrap_or('?'),
        0x22 => '∀',
        0x24 => '∃',
        0x27 => '∋',
        0x2A => '∗',
        0x2D => '−',
        0x40 => '≅',
        0x5C => '∴',
        0x5E => '⊥',
        0xA5 => '∞',
        0xA7 => '♣',
        0xA8 => '♦',
        0xA9 => '♥',
        0xAA => '♠',
        0xAB => '↔',
        0xAC => '←',
        0xAD => '↑',
        0xAE => '→',
        0xAF => '↓',
        0xB0 => '°',
        0xB1 => '±',
        0xB4 => '×',
        0xB7 => '•',
        0xB8 => '÷',
        0xB9 => '≠',
        0xBA => '≡',
        0xBB => '≈',
        0xBC => '…',
        0xD0 => '∠',
        0xD1 => '∇',
        0xD2 | 0xE2 => '®',
        0xD3 | 0xE3 => '©',
        0xD4 | 0xE4 => '™',
        0xD5 => '∏',
        0xD6 => '√',
        0xE5 => '∑',
        0x20..=0x7F => char::from(byte),
        _ => char::from_u32(0xF000 + u32::from(byte)).unwrap_or('?'),
    }
}

/// RTF's DTTM date (`\atndate`): minutes, hours, day, month, and years
/// since 1900 in bit fields; no seconds.
fn dttm(value: i64) -> Option<amipro_document::WallClock> {
    let value = value as u32;
    let field = |shift: u32, bits: u32| (value >> shift) & ((1 << bits) - 1);
    let (minute, hour, day, month, year) = (
        field(0, 6),
        field(6, 5),
        field(11, 5),
        field(16, 4),
        field(20, 9),
    );
    (day >= 1 && (1..=12).contains(&month)).then(|| amipro_document::WallClock {
        year: 1900 + year as i32,
        month: month as u8,
        day: day as u8,
        hour: hour.min(23) as u8,
        minute: minute.min(59) as u8,
        second: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> (ExchangeDocument, Findings) {
        let mut findings = Findings::default();
        let document = read_rtf(text.as_bytes(), &mut findings).unwrap();
        (document, findings)
    }

    fn paragraph_texts(blocks: &[Block]) -> Vec<String> {
        blocks
            .iter()
            .map(|block| match block {
                Block::Paragraph(paragraph) => paragraph
                    .inlines
                    .iter()
                    .map(|inline| match inline {
                        Inline::Text { text, .. } => text.clone(),
                        Inline::Tab { .. } => "\t".into(),
                        Inline::Field(field) => format!("[{}]", field.result),
                        Inline::Shape(_) => "[shape]".into(),
                        Inline::Footnote(_) => "[footnote]".into(),
                        Inline::Note(_) => "[note]".into(),
                        _ => String::new(),
                    })
                    .collect(),
                Block::Table(_) => "[table]".into(),
            })
            .collect()
    }

    /// VM fixture IM1's RTF, as written for the run.
    const IM1: &str = r"{\rtf1\ansi\ansicpg1252\deff0
{\fonttbl{\f0\froman Times New Roman;}{\f1\fswiss Arial;}{\f2\fmodern Courier New;}}
{\colortbl;\red0\green0\blue0;\red255\green0\blue0;}
{\stylesheet{\s0\f0\fs24 Normal;}{\s1\sbasedon0\snext2\f1\fs32\b\sb240\sa120 heading 1;}{\s2\sbasedon0\snext2\f1\fs28\sa120 Body Text;}{\s3\sbasedon0\snext3\f2\fs20\i\li720\cf2 My Style;}{\s4\sbasedon0\snext4\qc\f1\fs28 Title;}}
\paperw12240\paperh15840\margl1800\margr1800\margt1440\margb1440
\pard\plain\s4\qc\f1\fs28 Import Title\par
\pard\plain\s1\f1\fs32\b\sb240\sa120 First Heading\par
\pard\plain\s2\f1\fs28\sa120 Body text paragraph one with {\b bold} and {\i italic} words.\par
\pard\plain\s3\f2\fs20\i\li720\cf2 A paragraph in My Style.\par
\pard\plain\s0\f0\fs24 A Normal paragraph.\par
\pard\plain\s2\qr\f1\fs28\sa120 Body Text set right by direct formatting.\par
\pard\plain\f0\fs24 A paragraph with no style at all.\par
}";

    #[test]
    fn styles_paragraphs_and_runs() {
        let (document, _) = read(IM1);
        let names: Vec<&str> = document
            .styles
            .iter()
            .map(|style| style.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Normal", "heading 1", "Body Text", "My Style", "Title"]
        );
        let section = &document.sections[0];
        assert_eq!(section.page.margin_left, 1_800);
        assert_eq!(
            paragraph_texts(&section.blocks),
            [
                "Import Title",
                "First Heading",
                "Body text paragraph one with bold and italic words.",
                "A paragraph in My Style.",
                "A Normal paragraph.",
                "Body Text set right by direct formatting.",
                "A paragraph with no style at all.",
            ]
        );
        let Block::Paragraph(set_right) = &section.blocks[5] else {
            panic!("a paragraph")
        };
        assert_eq!(document.styles[set_right.style].name, "Body Text");
        assert_eq!(set_right.props.alignment, Some(Alignment::Right));
        assert_eq!(set_right.props.space_after, None);
        let Block::Paragraph(runs) = &section.blocks[2] else {
            panic!("a paragraph")
        };
        let bold = runs.inlines.iter().find_map(|inline| match inline {
            Inline::Text { text, props } if text == "bold" => Some(props.bold),
            _ => None,
        });
        assert_eq!(bold, Some(Some(true)));
        assert_eq!(document.styles[3].character.colour, Some(255));
    }

    #[test]
    fn unicode_fields_and_tables() {
        let (document, findings) = read(
            r#"{\rtf1\ansi\uc1 Caf\u233?{\field{\*\fldinst PAGE}{\fldrslt 3}} {\field{\*\fldinst HYPERLINK "http://x"}{\fldrslt link}}\par
\trowd\trgaph108\cellx2000\cellx4000\pard\intbl one\cell two\cell\row
\trowd\trgaph108\clmgf\cellx2000\clmrg\cellx4000\pard\intbl both\cell\cell\row
\pard after\par}"#,
        );
        let blocks = &document.sections[0].blocks;
        assert_eq!(
            paragraph_texts(blocks),
            ["Café[3] link", "[table]", "after"]
        );
        let Block::Table(table) = &blocks[1] else {
            panic!("a table")
        };
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.rows[1].cells.len(), 1);
        assert_eq!(table.rows[1].cells[0].span, 2);
        assert!(
            findings
                .iter()
                .any(|finding| finding.kind == FindingKind::Links)
        );
    }

    #[test]
    fn headers_footnotes_comments_and_hidden_text() {
        let (document, findings) = read(
            r"{\rtf1\ansi{\header \pard Head\par}Text{\footnote \pard Foot\par} {\v hidden}{\*\atnid LS}{\*\atnauthor Lee}\chatn{\*\annotation \pard Note\par}\par}",
        );
        let section = &document.sections[0];
        assert_eq!(
            paragraph_texts(section.headers.default.as_ref().unwrap()),
            ["Head"]
        );
        assert_eq!(paragraph_texts(&section.blocks), ["Text[footnote] [note]"]);
        assert!(
            findings
                .iter()
                .any(|finding| finding.kind == FindingKind::HiddenText)
        );
    }

    #[test]
    fn damaged_files_do_not_panic() {
        for text in [
            r"{\rtf1",
            r"{\rtf1{{{\b",
            r"{\rtf1\u-3?\'zz\bin5",
            r"{\rtf1}}}}",
        ] {
            let mut findings = Findings::default();
            let _ = read_rtf(text.as_bytes(), &mut findings);
        }
        let mut findings = Findings::default();
        assert!(read_rtf(b"hello", &mut findings).is_err());
    }
}
