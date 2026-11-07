//! Print and PDF (`docs/Design/UI_SPEC.md` §29; `docs/research/PRINT_NOTES.md`):
//! a document's pages drawn by the page surface in its print mode, page by
//! page, into a PDF. The same drawing serves Export PDF… and printing,
//! whose desktop dialog takes a PDF.

use std::path::{Path, PathBuf};

use amipro_document::Document;
use amipro_editing::merge::labels::LabelSheet;
use amipro_layout::DocumentLayout;
use cosmic::iced::advanced::widget::{self, Widget};
use cosmic::iced::advanced::{Layout, layout, mouse, renderer};
use cosmic::iced::{Point, Rectangle, Size};
use krilla::geom::Transform;

use crate::file_workflow::DocumentLocation;
use crate::page_editing::PageSelection;
use crate::page_surface::{PrintMode, page_surface};
use crate::page_view::PageView;
use crate::pdf::{PdfRenderer, PdfWriter, VectorPictures};
use crate::pictures::Pictures;

/// Twips to a point.
const TWIPS_PER_POINT: f32 = 20.0;
/// The surface draws for paper at a pixel a twip: rules, rounding, and text
/// as fine as the layout's own unit.
const PRINT_SCALE: f32 = 1.0;

/// What a PDF holds and how its pages are drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PdfOptions {
    /// Ami Pro's Without pictures and On preprinted form.
    pub mode: PrintMode,
    /// The pages to include, by position in the document (0-based), in
    /// order; every page when `None`.
    pub pages: Option<Vec<usize>>,
    /// Ami Pro's Crop marks: on what paper.
    pub crop_marks: Option<CropMarks>,
    /// Ami Pro's With doc description: the cover sheet's lines.
    pub cover: Option<CoverSheet>,
    /// Ami Pro's With notes: the notes on pages after the document.
    pub with_notes: bool,
    /// Bookmarks, properties, page labels, and PDF/A (PO4).
    pub additions: PdfAdditions,
    /// The page put on larger paper at this place, in twips: the paper's
    /// width and height and the page's left and top (an envelope fed at the
    /// left, centre, or right of the printer's paper path, MG8).
    pub placed: Option<[f32; 4]>,
}

/// What the PDF adds to the pages (`UI_SPEC.md` §29, PO4), from the
/// settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PdfAdditions {
    /// Bookmarks from the outline levels of the main text's styles.
    pub bookmarks: bool,
    /// Doc Info as the PDF's properties, and Ami Pro's page numbers as its
    /// page labels.
    pub properties: bool,
    /// PDF/A-2b, for archiving.
    pub archival: bool,
}

impl Default for PdfAdditions {
    fn default() -> Self {
        Self {
            bookmarks: true,
            properties: true,
            archival: false,
        }
    }
}

/// Ami Pro's Print Options (`UI_SPEC.md` §29): kept per document until it
/// closes, as Ami Pro keeps them, for printing and PDF alike.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PrintOptions {
    pub crop_marks: bool,
    pub without_pictures: bool,
    pub doc_description: bool,
    pub with_notes: bool,
    pub preprinted_form: bool,
    pub update_fields: bool,
}

impl PrintOptions {
    /// How the page surface draws with these options: for paper, or as
    /// the Print panel's preview.
    pub(crate) fn mode(self, preview: bool) -> PrintMode {
        PrintMode {
            without_pictures: self.without_pictures,
            preprinted_form: self.preprinted_form,
            preview,
        }
    }

    /// The PDF these options make of `document`, read from `file`; crop
    /// marks on `paper` (twips) when printing, around the page otherwise.
    pub(crate) fn pdf_options(
        self,
        document: &Document,
        file: Option<&Path>,
        paper: Option<(f32, f32)>,
    ) -> PdfOptions {
        PdfOptions {
            mode: self.mode(false),
            pages: None,
            crop_marks: self.crop_marks.then_some(match paper {
                Some((width, height)) => CropMarks::OnPaper { width, height },
                None => CropMarks::AroundPage,
            }),
            cover: self.doc_description.then(|| CoverSheet::of(document, file)),
            with_notes: self.with_notes,
            additions: PdfAdditions::default(),
            placed: None,
        }
    }
}

/// Where crop marks put the page (`PRINT_NOTES.md`, "Crop marks"): ½ in
/// right of and below the paper's corner, the marks in the half inch
/// around it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CropMarks {
    /// A PDF: the paper is the page with ½ in all round, its trim box the
    /// page.
    AroundPage,
    /// Printing: the paper the desktop's dialog chose, in twips.
    OnPaper { width: f32, height: f32 },
}

/// Half an inch, in twips: how far crop marks move the page.
const HALF_INCH: f32 = 720.0;
/// A crop mark's length, 7/16 in, ending 1/16 in short of the corner.
const MARK_LENGTH: f32 = 630.0;
const MARK_GAP: f32 = 90.0;

/// The eight crop marks around a page `width` × `height` twips set ½ in in
/// from the paper's corner: lines (x1, y1, x2, y2) in twips on the paper,
/// each on an extension of the page's edge.
pub(crate) fn crop_marks(width: f32, height: f32) -> Vec<[f32; 4]> {
    let (left, top) = (HALF_INCH, HALF_INCH);
    let (right, bottom) = (HALF_INCH + width, HALF_INCH + height);
    let before = |edge: f32| [edge - MARK_GAP - MARK_LENGTH, edge - MARK_GAP];
    let after = |edge: f32| [edge + MARK_GAP, edge + MARK_GAP + MARK_LENGTH];
    let mut marks = Vec::with_capacity(8);
    for (y, x_span) in [(top, before(left)), (top, after(right))] {
        marks.push([x_span[0], y, x_span[1], y]);
    }
    for (y, x_span) in [(bottom, before(left)), (bottom, after(right))] {
        marks.push([x_span[0], y, x_span[1], y]);
    }
    for (x, y_span) in [(left, before(top)), (right, before(top))] {
        marks.push([x, y_span[0], x, y_span[1]]);
    }
    for (x, y_span) in [(left, after(bottom)), (right, after(bottom))] {
        marks.push([x, y_span[0], x, y_span[1]]);
    }
    marks
}

/// Ami Pro's With doc description: a sheet before the document listing its
/// Doc Info (`PRINT_NOTES.md`; VM PFC1D).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CoverSheet {
    /// Each label padded to 20 characters with its value, double-spaced.
    pub lines: Vec<String>,
    /// The description, wrapped after its label, single-spaced.
    pub description: String,
    /// The linked pictures' files, after "Imported files:".
    pub imported: Vec<String>,
}

impl CoverSheet {
    /// The cover sheet of `document`, read from `file` (`None` for a new
    /// one): as Ami Pro's, with four-digit years.
    pub(crate) fn of(document: &Document, file: Option<&Path>) -> Self {
        use amipro_document::{DateSettings, WallClock, format_time};
        let info = document.info();
        let label = |label: &str, value: &str| format!("{label:<19} {value}");
        let date = |seconds: Option<u32>| {
            seconds.map_or_else(String::new, |seconds| {
                let clock = WallClock::from_ami_pro(seconds);
                format!("{:02}/{:02}/{}", clock.month, clock.day, clock.year)
            })
        };
        let time = |seconds: Option<u32>| {
            seconds.map_or_else(String::new, |seconds| {
                format_time(&WallClock::from_ami_pro(seconds), &DateSettings::US)
            })
        };
        let (name, folder) = match file {
            Some(file) => (
                file.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                file.parent()
                    .map(|folder| folder.display().to_string())
                    .unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };
        let mut lines = vec![
            label("Filename:", &name),
            label("Path:", &folder),
            label(
                "Style Sheet:",
                document
                    .style_sheet()
                    .map_or("None", |sheet| sheet.name.as_str()),
            ),
            label("Date Created:", &date(info.created)),
            label("Time Created:", &time(info.created)),
            label("Date Last Rev:", &date(info.saved)),
            label("Total Editing Time:", &info.editing_minutes.to_string()),
            label("Total Revisions:", &info.revisions.to_string()),
        ];
        for (index, (field_name, value)) in info.field_names.iter().zip(&info.fields).enumerate() {
            let field_name = if field_name.is_empty() {
                format!("Field{}", index + 1)
            } else {
                field_name.clone()
            };
            lines.push(label(&format!("{field_name}:"), value));
        }
        let imported = document
            .frames()
            .filter_map(|frame| match &frame.content {
                amipro_document::FrameContent::Picture(picture) => match &picture.source {
                    amipro_document::PictureSource::Linked(path) => Some(path.clone()),
                    amipro_document::PictureSource::Embedded(_) => None,
                },
                _ => None,
            })
            .collect();
        Self {
            lines,
            description: info.description.clone(),
            imported,
        }
    }
}

/// The cover sheet's face, size, and place: a fixed pitch of 10 points, 1
/// in from the paper's left and top, lines 20 points apart.
const COVER_FACE: &str = "Liberation Mono";
const COVER_SIZE: f32 = 200.0;
const COVER_MARGIN: f32 = 1_440.0;

/// Draws `cover` on a sheet `width` twips wide, at a pixel a twip.
fn draw_cover(renderer: &mut PdfRenderer, cover: &CoverSheet, width: f32) {
    use cosmic::iced::advanced::text::{self, Renderer as _};
    let text = |content: String, wrap_width: f32| text::Text {
        content,
        bounds: Size::new(wrap_width, f32::INFINITY),
        size: cosmic::iced::Pixels(COVER_SIZE),
        line_height: text::LineHeight::Absolute(cosmic::iced::Pixels(COVER_SIZE)),
        font: cosmic::iced::Font::with_name(COVER_FACE),
        align_x: text::Alignment::Left,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::Word,
        ellipsize: text::Ellipsize::None,
    };
    let colour = cosmic::iced::Color::BLACK;
    let mut y = COVER_MARGIN;
    for line in &cover.lines {
        renderer.fill_text(
            text(line.clone(), f32::INFINITY),
            Point::new(COVER_MARGIN, y),
            colour,
            Rectangle::INFINITE,
        );
        y += 2.0 * COVER_SIZE;
    }
    // "Doc Desc:" with the description wrapped after it, as DrawText wraps
    // it, to 1 in from the paper's right edge; then the imported files.
    renderer.fill_text(
        text("Doc Desc:".into(), f32::INFINITY),
        Point::new(COVER_MARGIN, y),
        colour,
        Rectangle::INFINITE,
    );
    let column = COVER_MARGIN + 20.0 * COVER_SIZE * 0.6;
    let description_lines = if cover.description.trim().is_empty() {
        1.0
    } else {
        use cosmic::iced::advanced::text::Paragraph as _;
        let wrapped = text(
            cover.description.clone(),
            (width - COVER_MARGIN - column).max(COVER_SIZE),
        );
        let paragraph =
            cosmic::iced::advanced::graphics::text::Paragraph::with_text(wrapped.as_ref());
        let lines = (paragraph.min_bounds().height / COVER_SIZE)
            .round()
            .max(1.0);
        renderer.fill_paragraph(
            &paragraph,
            Point::new(column, y),
            colour,
            Rectangle::INFINITE,
        );
        lines
    };
    y += description_lines * COVER_SIZE;
    renderer.fill_text(
        text("Imported files:".into(), f32::INFINITY),
        Point::new(COVER_MARGIN, y),
        colour,
        Rectangle::INFINITE,
    );
    for file in &cover.imported {
        renderer.fill_text(
            text(file.clone(), f32::INFINITY),
            Point::new(column, y),
            colour,
            Rectangle::INFINITE,
        );
        y += COVER_SIZE;
    }
}

/// Each bookmark's place: its name, the page (0-based) and the top of the
/// line (twips) where it starts.
fn bookmark_places(document: &Document, layout: &DocumentLayout) -> Vec<(String, usize, f32)> {
    use amipro_document::bookmarks::{BookmarkTarget, bookmarks};
    let mut places = Vec::new();
    for bookmark in bookmarks(document) {
        let (flow, paragraph, byte) = match &bookmark.target {
            BookmarkTarget::Text { start, .. } => (start.flow, start.paragraph, start.byte),
            BookmarkTarget::Cells { mark, .. } => (mark.flow, 0, 0),
            _ => continue,
        };
        let found = layout.pages.iter().enumerate().find_map(|(index, page)| {
            page.lines
                .iter()
                .find(|line| {
                    line.flow == flow
                        && line.paragraph_index == paragraph
                        && line.bytes.start <= byte
                        && byte <= line.bytes.end
                })
                .map(|line| (index, line.top))
        });
        if let Some((page, top)) = found {
            places.push((bookmark.name, page, top));
        }
    }
    places
}

/// The notes' pages (With notes): a "Page N" heading for each page with
/// notes, then each note's initials and number and its text, 12 points,
/// 1 in margins.
fn note_pages(
    document: &Document,
    layout: &DocumentLayout,
    width: f32,
    height: f32,
) -> Vec<PdfRenderer> {
    use cosmic::iced::advanced::text::{self, Paragraph as _, Renderer as _};
    let mut notes: Vec<(u32, String)> = amipro_editing::references::notes(document)
        .into_iter()
        .map(|(at, note, flow)| {
            let page =
                crate::reference_commands::page_showing(layout, at.flow, at.paragraph, at.byte)
                    .unwrap_or(0);
            let label = format!(
                "[{}{}]",
                note.initials.as_deref().unwrap_or(""),
                note.number
                    .filter(|number| *number > 0)
                    .map_or_else(String::new, |n| n.to_string())
            );
            let text = amipro_editing::references::note_text(document, flow);
            (page, format!("{label}  {text}"))
        })
        .collect();
    notes.sort_by_key(|(page, _)| *page);
    if notes.is_empty() {
        return Vec::new();
    }
    const SIZE: f32 = 240.0;
    const MARGIN: f32 = 1_440.0;
    let paragraph = |content: String, bold: bool| {
        let font = cosmic::iced::Font {
            weight: if bold {
                cosmic::iced::font::Weight::Bold
            } else {
                cosmic::iced::font::Weight::Normal
            },
            ..cosmic::iced::Font::with_name("Liberation Serif")
        };
        cosmic::iced::advanced::graphics::text::Paragraph::with_text(text::Text {
            content: content.as_str(),
            bounds: Size::new(width - 2.0 * MARGIN, f32::INFINITY),
            size: cosmic::iced::Pixels(SIZE),
            line_height: text::LineHeight::Absolute(cosmic::iced::Pixels(SIZE * 1.2)),
            font,
            align_x: text::Alignment::Left,
            align_y: cosmic::iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::Word,
            ellipsize: text::Ellipsize::None,
        })
    };
    let mut pages = vec![PdfRenderer::new()];
    let mut y = MARGIN;
    let mut last_page = None;
    for (page, text) in notes {
        let mut pieces = Vec::new();
        if last_page != Some(page) {
            pieces.push((paragraph(format!("Page {page}"), true), SIZE * 0.6));
            last_page = Some(page);
        }
        pieces.push((paragraph(text, false), SIZE * 0.4));
        for (shaped, after) in pieces {
            let tall = shaped.min_bounds().height;
            if y + tall > height - MARGIN && y > MARGIN {
                pages.push(PdfRenderer::new());
                y = MARGIN;
            }
            let renderer = pages.last_mut().expect("a page");
            renderer.fill_paragraph(
                &shaped,
                Point::new(MARGIN, y),
                cosmic::iced::Color::BLACK,
                Rectangle::INFINITE,
            );
            y += tall + after;
        }
    }
    pages
}

/// A PDF to draw off the interface's thread: the document and its pages
/// are shared roots, so the copies are cheap.
pub(crate) struct PdfJob {
    pub document: Document,
    pub layout: DocumentLayout,
    /// Where linked pictures are looked for.
    pub folder: Option<PathBuf>,
    /// A master document's set, printed instead of the document (MS6):
    /// its TOC file, its files, and its index file, in order.
    pub set: Vec<SetPart>,
    pub options: PdfOptions,
    /// Merged labels (MG7): the sheet, and the run of labels by part (the
    /// set's parts are the records; `None` leaves a label empty).
    pub labels: Option<(LabelSheet, Vec<Option<usize>>)>,
}

/// One file of a master document's set, as it prints.
#[derive(Clone)]
pub(crate) struct SetPart {
    pub document: Document,
    pub layout: DocumentLayout,
    pub folder: Option<PathBuf>,
    /// Its bookmark: the file's name.
    pub title: String,
    /// The pages it prints (all when `None`).
    pub pages: Option<Vec<usize>>,
}

impl PdfJob {
    /// Draws the PDF, with a picture cache of its own.
    pub(crate) fn render(&self) -> Result<Vec<u8>, String> {
        let pictures = Pictures::default();
        if !self.set.is_empty() {
            let parts: Vec<PdfPart<'_>> = self
                .set
                .iter()
                .map(|part| PdfPart {
                    document: &part.document,
                    layout: &part.layout,
                    folder: part.folder.as_deref(),
                    title: Some(part.title.clone()),
                    pages: part.pages.clone(),
                })
                .collect();
            if let Some((sheet, run)) = &self.labels {
                return labels_pdf(&parts, sheet, run, Some(&pictures), &self.options);
            }
            // The set prints every page of each file (Ami Pro resets the
            // range for each one, `1568:0c93`).
            let options = PdfOptions {
                pages: None,
                ..self.options.clone()
            };
            return set_pdf(&parts, Some(&pictures), &options);
        }
        document_pdf(
            &self.document,
            &self.layout,
            Some((&pictures, self.folder.as_deref())),
            &self.options,
        )
    }

    /// The document's pages it holds.
    pub(crate) fn pages(&self) -> usize {
        if let Some((sheet, run)) = &self.labels {
            return run.len().div_ceil(sheet.per_sheet());
        }
        if !self.set.is_empty() {
            return self
                .set
                .iter()
                .map(|part| {
                    part.pages
                        .as_ref()
                        .map_or(part.layout.pages.len(), Vec::len)
                })
                .sum();
        }
        self.options
            .pages
            .as_ref()
            .map_or(self.layout.pages.len(), Vec::len)
    }
}

/// An export to run off the interface's thread.
pub(crate) struct ExportJob {
    pub job: PdfJob,
    pub path: PathBuf,
}

/// What an export wrote.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Exported {
    pub path: PathBuf,
    pub pages: usize,
}

impl Exported {
    /// The status bar's report.
    pub(crate) fn message(&self) -> String {
        let name = self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let pages = if self.pages == 1 {
            "1 page".to_owned()
        } else {
            format!("{} pages", self.pages)
        };
        format!("Exported {pages} to {name}")
    }
}

impl ExportJob {
    /// Draws the PDF and writes it.
    pub(crate) fn run(self) -> Result<Exported, String> {
        let pdf = self.job.render()?;
        crate::file_workflow::write_file_replacing(&self.path, &pdf)?;
        Ok(Exported {
            path: self.path,
            pages: self.job.pages(),
        })
    }
}

/// Runs `work` on a thread of its own and waits for it without blocking.
pub(crate) async fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = cosmic::iced::futures::channel::oneshot::channel();
    std::thread::Builder::new()
        .name("pdf".into())
        .spawn(move || {
            let _ = sender.send(work());
        })
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "the PDF stopped unexpectedly".to_owned())?
}

/// What printing sent, for the status bar.
pub(crate) fn printed_message(pages: usize) -> String {
    if pages == 1 {
        "Sent 1 page to the printer".to_owned()
    } else {
        format!("Sent {pages} pages to the printer")
    }
}

/// The PDF's suggested name: the document's file name with `.pdf`, else
/// its title's.
pub(crate) fn pdf_name(location: &DocumentLocation, title: &str) -> String {
    let stem = location
        .path()
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned());
    let stem = stem.unwrap_or_else(|| {
        let title = title.trim();
        if title.is_empty() {
            "Untitled".to_owned()
        } else {
            title.to_owned()
        }
    });
    format!("{stem}.pdf")
}

/// A document's pages as a PDF: each page at its layout's size and
/// orientation, drawn as the page surface draws it for paper.
pub(crate) fn document_pdf(
    document: &Document,
    layout: &DocumentLayout,
    pictures: Option<(&Pictures, Option<&Path>)>,
    options: &PdfOptions,
) -> Result<Vec<u8>, String> {
    let folder = pictures.and_then(|(_, folder)| folder);
    set_pdf(
        &[PdfPart {
            document,
            layout,
            folder,
            title: None,
            pages: None,
        }],
        pictures.map(|(pictures, _)| pictures),
        options,
    )
}

/// One document of a PDF of several (a master document's set, MS6): its
/// bookmark is `title` when there is one.
pub(crate) struct PdfPart<'a> {
    pub document: &'a Document,
    pub layout: &'a DocumentLayout,
    pub folder: Option<&'a Path>,
    pub title: Option<String>,
    /// The pages it prints; `None` takes the options' selection for the
    /// first part and every page for the rest.
    pub pages: Option<Vec<usize>>,
}

/// Several documents' pages as one PDF, one after the other, each as
/// [`document_pdf`] draws it (MS6: Ami Pro prints a master's TOC file, its
/// files, then its index file). The page selection applies to the first
/// document only; the cover sheet goes once before everything; each part
/// with a title gets a bookmark over its own.
pub(crate) fn set_pdf(
    parts: &[PdfPart<'_>],
    pictures: Option<&Pictures>,
    options: &PdfOptions,
) -> Result<Vec<u8>, String> {
    let mut writer = PdfWriter::new(serialize_settings(options.additions.archival)?);
    let vectors = pictures.map(|pictures| pictures as &dyn VectorPictures);
    let points = 1.0 / (PRINT_SCALE * TWIPS_PER_POINT);
    let mut outline_nodes = Vec::new();
    let mut first_page = 0;
    for (part_index, part) in parts.iter().enumerate() {
        let document = part.document;
        let layout = part.layout;
        let pictures = pictures.map(|pictures| (pictures, part.folder));
        let pages = if let Some(pages) = &part.pages {
            pages.clone()
        } else if part_index == 0 {
            options
                .pages
                .clone()
                .unwrap_or_else(|| (0..layout.pages.len()).collect())
        } else {
            (0..layout.pages.len()).collect()
        };
        if pages.is_empty() {
            continue;
        }
        // The paper a page goes on, in twips, and where the page sits on it.
        let paper_of = |width: f32, height: f32| match options.crop_marks {
            None => match options.placed {
                Some([paper_width, paper_height, left, top]) => {
                    ((paper_width, paper_height), (left, top))
                }
                None => ((width, height), (0.0, 0.0)),
            },
            Some(CropMarks::AroundPage) => (
                (width + 2.0 * HALF_INCH, height + 2.0 * HALF_INCH),
                (HALF_INCH, HALF_INCH),
            ),
            Some(CropMarks::OnPaper { width, height }) => ((width, height), (HALF_INCH, HALF_INCH)),
        };
        // With doc description: one sheet before the document, of the first
        // page's paper (the chosen paper when printing with crop marks),
        // printed once.
        if part_index == 0
            && let Some(cover) = &options.cover
        {
            let first = pages.first().and_then(|page| layout.pages.get(*page));
            let (width, height) = match options.crop_marks {
                Some(CropMarks::OnPaper { width, height }) => (width, height),
                _ => first.map_or((12_240.0, 15_840.0), |page| (page.width, page.height)),
            };
            let mut renderer = PdfRenderer::new();
            draw_cover(&mut renderer, cover, width * PRINT_SCALE);
            writer.page(
                (width / TWIPS_PER_POINT, height / TWIPS_PER_POINT),
                |settings| settings,
                &renderer,
                Transform::from_scale(points, points),
                None,
                crate::pdf::PageExtras::default(),
            );
        }
        // Page labels: Ami Pro's numbers where a page number field shows one.
        let labels = options
            .additions
            .properties
            .then(|| page_labels(document, layout, &pages));
        if part_index == 0 {
            first_page = usize::from(options.cover.is_some());
        }
        let link_offset = if options.crop_marks.is_some() {
            HALF_INCH
        } else {
            0.0
        };
        let links = toc_links(document, layout, &pages, first_page, link_offset);
        let mut written = 0;
        draw_pages(
            document,
            layout,
            pictures,
            Some(options.mode),
            &pages,
            |page, renderer| {
                let label = labels
                    .as_ref()
                    .and_then(|labels| labels.get(written).cloned().flatten());
                let page_links = pages
                    .get(written)
                    .and_then(|index| links.get(index))
                    .map_or(&[][..], Vec::as_slice);
                written += 1;
                let ((width, height), (left, top)) =
                    paper_of(page.width / PRINT_SCALE, page.height / PRINT_SCALE);
                let to_page = Transform::from_row(
                    points,
                    0.0,
                    0.0,
                    points,
                    left / TWIPS_PER_POINT - page.x * points,
                    top / TWIPS_PER_POINT - page.y * points,
                );
                let trim = matches!(options.crop_marks, Some(CropMarks::AroundPage)).then(|| {
                    krilla::geom::Rect::from_xywh(
                        left / TWIPS_PER_POINT,
                        top / TWIPS_PER_POINT,
                        page.width * points,
                        page.height * points,
                    )
                });
                let marks: Vec<[f32; 4]> = if options.crop_marks.is_some() {
                    crop_marks(page.width / PRINT_SCALE, page.height / PRINT_SCALE)
                        .into_iter()
                        .map(|mark| mark.map(|value| value / TWIPS_PER_POINT))
                        .collect()
                } else {
                    Vec::new()
                };
                writer.page(
                    (width / TWIPS_PER_POINT, height / TWIPS_PER_POINT),
                    |settings| {
                        let settings = settings.with_trim_box(trim.flatten());
                        match label {
                            Some(label) => settings.with_page_label(label),
                            None => settings,
                        }
                    },
                    renderer,
                    to_page,
                    vectors,
                    crate::pdf::PageExtras {
                        crop: &marks,
                        links: page_links,
                    },
                );
            },
        );
        let mut notes_written = 0;
        // With notes: after the document, each page's notes under its number
        // (the guide, ch. 24), on the last page's paper.
        if options.with_notes {
            let (width, height) = pages
                .last()
                .and_then(|page| layout.pages.get(*page))
                .map_or((12_240.0, 15_840.0), |page| (page.width, page.height));
            for renderer in note_pages(document, layout, width, height) {
                notes_written += 1;
                writer.page(
                    (width / TWIPS_PER_POINT, height / TWIPS_PER_POINT),
                    |settings| settings,
                    &renderer,
                    Transform::from_scale(points, points),
                    None,
                    crate::pdf::PageExtras::default(),
                );
            }
        }
        let offset = if options.crop_marks.is_some() {
            HALF_INCH
        } else {
            0.0
        };
        if options.additions.bookmarks {
            let nodes = bookmark_nodes(document, layout, &pages, first_page, offset);
            match &part.title {
                Some(title) => {
                    let mut node = krilla::outline::OutlineNode::new(
                        title.clone(),
                        krilla::destination::XyzDestination::new(
                            first_page,
                            krilla::geom::Point::from_xy(0.0, 0.0),
                        ),
                    );
                    for child in nodes {
                        node.push_child(child);
                    }
                    outline_nodes.push(node);
                }
                None => outline_nodes.extend(nodes),
            }
        }
        // Ami Pro's bookmarks as named destinations, for links to them (NR6).
        for (name, page, y) in bookmark_places(document, layout) {
            let Some(index) = pages.iter().position(|shown| *shown == page) else {
                continue;
            };
            let point = krilla::geom::Point::from_xy(
                offset / TWIPS_PER_POINT,
                (y + offset) / TWIPS_PER_POINT,
            );
            let destination = krilla::destination::NamedDestination::new(
                name,
                krilla::destination::XyzDestination::new(index + first_page, point),
            );
            let _ = writer
                .document_mut()
                .register_named_destination(destination);
        }
        if part_index == 0 && (options.additions.properties || options.additions.archival) {
            writer.document_mut().set_metadata(metadata(document));
        }
        first_page += written + if options.with_notes { notes_written } else { 0 };
    }
    if options.additions.bookmarks {
        let mut outline = krilla::outline::Outline::new();
        for node in outline_nodes {
            outline.push_child(node);
        }
        writer.document_mut().set_outline(outline);
    }
    writer.finish()
}

/// The generated TOC's rows as links to their headings (TX6), by layout
/// page: each row's rectangle over its cells' lines, and the heading's
/// line as the destination; none when the rows are not the TOC's entries
/// as they are now.
pub(crate) fn toc_links(
    document: &Document,
    layout: &DocumentLayout,
    pages: &[usize],
    first_page: usize,
    offset: f32,
) -> std::collections::HashMap<usize, Vec<(krilla::geom::Rect, krilla::destination::XyzDestination)>>
{
    let mut links: std::collections::HashMap<usize, Vec<_>> = std::collections::HashMap::new();
    let Some(frame) = amipro_editing::contents::toc_table(document) else {
        return links;
    };
    let Some(amipro_document::FrameContent::Table(table)) =
        document.frame(frame).map(|frame| &frame.content)
    else {
        return links;
    };
    let host = crate::contents_commands::LayoutHost { document, layout };
    let levels = amipro_editing::contents::toc_levels(document);
    let entries = amipro_editing::contents::collect_toc(document, &levels, &host);
    let columns = table.columns.len().max(1);
    if entries.len() * columns != table.cells.len() {
        return links;
    }
    for (row, entry) in entries.iter().enumerate() {
        let target = entry.source;
        let Some(number) = crate::reference_commands::page_showing(
            layout,
            target.flow,
            target.paragraph,
            target.byte,
        ) else {
            continue;
        };
        let Some(written) = pages.iter().position(|page| *page + 1 == number as usize) else {
            continue;
        };
        let Some(heading) = layout.pages[number as usize - 1]
            .lines
            .iter()
            .find(|line| line.flow == target.flow && line.paragraph_index == target.paragraph)
        else {
            continue;
        };
        let destination = krilla::destination::XyzDestination::new(
            first_page + written,
            krilla::geom::Point::from_xy(
                (heading.x + offset) / TWIPS_PER_POINT,
                (heading.top + offset) / TWIPS_PER_POINT,
            ),
        );
        let flows: Vec<amipro_document::FlowId> = table.cells[row * columns..(row + 1) * columns]
            .iter()
            .map(|cell| match cell.content {
                amipro_document::CellContent::Text(flow) => flow,
            })
            .collect();
        for (index, page) in layout.pages.iter().enumerate() {
            let mut bounds: Option<(f32, f32, f32, f32)> = None;
            for line in page.lines.iter().filter(|line| flows.contains(&line.flow)) {
                let (left, top, right, bottom) = (
                    line.x,
                    line.top,
                    line.x + line.slot_width.max(line.width),
                    line.top + line.height,
                );
                bounds = Some(match bounds {
                    None => (left, top, right, bottom),
                    Some((l, t, r, b)) => (l.min(left), t.min(top), r.max(right), b.max(bottom)),
                });
            }
            if let Some((left, top, right, bottom)) = bounds
                && let Some(rect) = krilla::geom::Rect::from_ltrb(
                    (left + offset) / TWIPS_PER_POINT,
                    (top + offset) / TWIPS_PER_POINT,
                    (right + offset) / TWIPS_PER_POINT,
                    (bottom + offset) / TWIPS_PER_POINT,
                )
            {
                links
                    .entry(index)
                    .or_default()
                    .push((rect, destination.clone()));
            }
        }
    }
    links
}

/// krilla's settings: PDF/A-2b when archiving.
fn serialize_settings(archival: bool) -> Result<krilla::SerializeSettings, String> {
    let mut settings = krilla::SerializeSettings::default();
    if archival {
        settings.configuration = krilla::configure::ConfigurationBuilder::new()
            .with_archival_validator(krilla::configure::Archival::A2_B)
            .finish()
            .map_err(|error| format!("{error:?}"))?;
    }
    Ok(settings)
}

/// Doc Info as the PDF's properties: the description as its title (else
/// the document's), the keywords, the date last saved (or created, or
/// now), and the program.
fn metadata(document: &Document) -> krilla::metadata::Metadata {
    use amipro_document::WallClock;
    let info = document.info();
    let title = if info.description.trim().is_empty() {
        document.title.clone()
    } else {
        info.description.clone()
    };
    let date = |seconds: u32| {
        let clock = WallClock::from_ami_pro(seconds);
        krilla::metadata::DateTime::new(clock.year.clamp(0, 9_999) as u16)
            .month(clock.month)
            .day(clock.day)
            .hour(clock.hour)
            .minute(clock.minute)
            .second(clock.second)
    };
    let mut metadata = krilla::metadata::Metadata::new()
        .title(title)
        .creator("AmiPro-Linux".to_owned());
    let keywords: Vec<String> = info
        .keywords
        .split([',', ';'])
        .map(str::trim)
        .filter(|keyword| !keyword.is_empty())
        .map(str::to_owned)
        .collect();
    if !keywords.is_empty() {
        metadata = metadata.keywords(keywords);
    }
    // krilla records only a creation date; Ami Pro's last save is the
    // nearer one to what the reader opens, and a document never saved is
    // dated now (PDF/A requires a date).
    let dated = info
        .saved
        .or(info.created)
        .unwrap_or_else(crate::clock::ami_pro_now);
    metadata.creation_date(date(dated))
}

/// Each written page's label: the number Ami Pro shows on it, in the style
/// and with the leading text of the document's page number field; `None`
/// where no field shows one (the viewer counts on from the label before).
fn page_labels(
    document: &Document,
    layout: &DocumentLayout,
    pages: &[usize],
) -> Vec<Option<krilla::page::PageLabel>> {
    use amipro_document::InlineObject;
    use krilla::page::NumberingStyle;
    let field = document.flows().find_map(|(_, flow)| {
        flow.paragraphs.iter().find_map(|paragraph| {
            paragraph.objects.iter().find_map(|object| match object {
                InlineObject::PageNumber(field) => Some(field.clone()),
                _ => None,
            })
        })
    });
    let Some(field) = field else {
        return vec![None; pages.len()];
    };
    let style = match field.style {
        'I' => NumberingStyle::UpperRoman,
        'i' => NumberingStyle::LowerRoman,
        'A' => NumberingStyle::UpperAlpha,
        'a' => NumberingStyle::LowerAlpha,
        _ => NumberingStyle::Arabic,
    };
    let prefix = (!field.leading_text.is_empty()).then(|| field.leading_text.clone());
    pages
        .iter()
        .map(|&page| {
            let number = layout.pages.get(page)?.number;
            let shown = field.number_on(number)?;
            Some(krilla::page::PageLabel::new(
                Some(style),
                prefix.clone(),
                std::num::NonZeroU32::new(shown),
            ))
        })
        .collect()
}

/// Bookmarks from the outline levels (`UI_SPEC.md` §22) of the main text's
/// styles: each levelled paragraph on a written page, under the nearest
/// lower level before it, titled with its number as shown and its text.
fn bookmark_nodes(
    document: &Document,
    layout: &DocumentLayout,
    pages: &[usize],
    first_page: usize,
    offset: f32,
) -> Vec<krilla::outline::OutlineNode> {
    use krilla::outline::OutlineNode;
    let main = document.main_flow_id();
    let mut entries: Vec<(u8, OutlineNode)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (written, &page_index) in pages.iter().enumerate() {
        let Some(page) = layout.pages.get(page_index) else {
            continue;
        };
        for line in &page.lines {
            if line.flow != main || !seen.insert(line.paragraph_index) {
                continue;
            }
            let Some(paragraph) = document.paragraphs().get(line.paragraph_index) else {
                continue;
            };
            let level = document
                .styles()
                .get(&paragraph.style)
                .map_or(0, |style| style.bullets.level);
            let text: String = paragraph
                .text
                .chars()
                .filter(|character| !character.is_control() && *character != '\u{FFFC}')
                .collect();
            if level == 0 || text.trim().is_empty() {
                continue;
            }
            let number: String = line
                .bullet
                .iter()
                .flat_map(|bullet| &bullet.pieces)
                .filter_map(|piece| match piece {
                    amipro_layout::BulletPiece::Text { text, .. } => Some(text.as_str()),
                    amipro_layout::BulletPiece::Shape { .. } => None,
                })
                .collect();
            let title = format!("{} {}", number.trim(), text.trim())
                .trim()
                .to_owned();
            let destination = krilla::destination::XyzDestination::new(
                first_page + written,
                krilla::geom::Point::from_xy(
                    (line.x + offset) / TWIPS_PER_POINT,
                    (line.top + offset) / TWIPS_PER_POINT,
                ),
            );
            entries.push((level, OutlineNode::new(title, destination)));
        }
    }
    nested(entries, |parent: &mut OutlineNode, child| {
        parent.push_child(child)
    })
}

/// Entries with levels nested as an outline: each under the nearest one
/// before it with a lower level (`push` adds a child to its parent).
fn nested<T>(entries: Vec<(u8, T)>, mut push: impl FnMut(&mut T, T)) -> Vec<T> {
    type Entries<T> = std::iter::Peekable<std::vec::IntoIter<(u8, T)>>;
    fn below<T>(entries: &mut Entries<T>, level: u8, push: &mut impl FnMut(&mut T, T)) -> Vec<T> {
        let mut nodes = Vec::new();
        while let Some((next, _)) = entries.peek() {
            if *next <= level {
                break;
            }
            let (own, mut node) = entries.next().expect("peeked");
            for child in below(entries, own, push) {
                push(&mut node, child);
            }
            nodes.push(node);
        }
        nodes
    }
    below(&mut entries.into_iter().peekable(), 0, &mut push)
}

/// Draws `pages` (positions in the document) one at a time through the page
/// surface, in `mode` (the screen's drawing when `None`), handing each
/// page's rectangle and what was drawn to `each`.
/// Page `page` (0-based) of `document` as drawn in `mode` for paper, for
/// tests: the quads' colours and the text that would print.
#[cfg(test)]
pub(crate) fn drawn_page(
    document: &Document,
    layout: &DocumentLayout,
    mode: Option<PrintMode>,
    page: usize,
) -> (Vec<cosmic::iced::Color>, Vec<String>) {
    let mut drawn = (Vec::new(), Vec::new());
    draw_pages(
        document,
        layout,
        None,
        mode,
        &[page],
        |_, renderer: &mut PdfRenderer| {
            drawn = (
                renderer
                    .quads()
                    .into_iter()
                    .filter_map(|(_, background)| match background {
                        cosmic::iced::Background::Color(colour) => Some(colour),
                        _ => None,
                    })
                    .collect(),
                renderer.printed_texts(),
            );
        },
    );
    drawn
}

fn draw_pages(
    document: &Document,
    layout: &DocumentLayout,
    pictures: Option<(&Pictures, Option<&Path>)>,
    mode: Option<PrintMode>,
    pages: &[usize],
    mut each: impl FnMut(Rectangle, &mut PdfRenderer),
) {
    let view = print_view(layout);
    let selection = PageSelection::start(document);
    let mut surface =
        page_surface(document, layout, view.clone(), 0, selection, |_| (), |_| ()).printing(mode);
    if let Some((pictures, folder)) = pictures {
        surface = surface.pictures(pictures, folder);
    }
    let mut renderer = PdfRenderer::new();
    let mut tree = widget::Tree::new(&surface as &dyn Widget<(), cosmic::Theme, PdfRenderer>);
    let node = Widget::<(), cosmic::Theme, PdfRenderer>::layout(
        &mut surface,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::INFINITE),
    );
    let theme = cosmic::Theme::light();
    for &page_index in pages {
        let Some(&paper) = view.pages.get(page_index) else {
            continue;
        };
        renderer.clear();
        Widget::<(), cosmic::Theme, PdfRenderer>::draw(
            &surface,
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &paper,
        );
        each(paper, &mut renderer);
    }
}

/// Merged labels on their sheets (MG7; `1550:0145`): each label is its
/// part's first page, drawn at its place on the sheet (across, then down,
/// then a new sheet), on the sheet's paper.
pub(crate) fn labels_pdf(
    parts: &[PdfPart<'_>],
    sheet: &LabelSheet,
    run: &[Option<usize>],
    pictures: Option<&Pictures>,
    options: &PdfOptions,
) -> Result<Vec<u8>, String> {
    let mut writer = PdfWriter::new(serialize_settings(options.additions.archival)?);
    let vectors = pictures.map(|pictures| pictures as &dyn VectorPictures);
    let points = 1.0 / (PRINT_SCALE * TWIPS_PER_POINT);
    // Each part's label, drawn once.
    let mut drawn: Vec<Option<(Rectangle, PdfRenderer)>> = Vec::with_capacity(parts.len());
    for part in parts {
        let mut label = None;
        draw_pages(
            part.document,
            part.layout,
            pictures.map(|pictures| (pictures, part.folder)),
            Some(options.mode),
            &[0],
            |page, renderer| label = Some((page, std::mem::take(renderer))),
        );
        drawn.push(label);
    }
    let (width, height) = sheet.paper.size();
    let per = sheet.per_sheet();
    for sheet_run in run.chunks(per) {
        let mut pieces: Vec<(&PdfRenderer, Transform)> = Vec::new();
        for (at, label) in sheet_run.iter().enumerate() {
            let Some(Some((page, renderer))) =
                label.map(|part| drawn.get(part).and_then(Option::as_ref))
            else {
                continue;
            };
            let (_, left, top) = sheet.place(at);
            pieces.push((
                renderer,
                Transform::from_row(
                    points,
                    0.0,
                    0.0,
                    points,
                    left as f32 / TWIPS_PER_POINT - page.x * points,
                    top as f32 / TWIPS_PER_POINT - page.y * points,
                ),
            ));
        }
        writer.sheet(
            (
                width as f32 / TWIPS_PER_POINT,
                height as f32 / TWIPS_PER_POINT,
            ),
            &pieces,
            vectors,
        );
    }
    writer.finish()
}

/// Every page at a pixel a twip, one under another with a gap, so that
/// drawing one page's rectangle draws that page alone.
fn print_view(layout: &DocumentLayout) -> PageView {
    let gap = 1_440.0 * PRINT_SCALE;
    let mut y = 0.0;
    let mut widest: f32 = 1.0;
    let pages = layout
        .pages
        .iter()
        .map(|page| {
            let rectangle = Rectangle::new(
                Point::new(0.0, y),
                Size::new(page.width * PRINT_SCALE, page.height * PRINT_SCALE),
            );
            y += rectangle.height + gap;
            widest = widest.max(rectangle.width);
            rectangle
        })
        .collect();
    PageView {
        scale: PRINT_SCALE,
        pages,
        size: Size::new(widest, y.max(1.0)),
    }
}

#[cfg(test)]
mod tests {
    use cosmic::iced::Color;

    use super::*;
    use crate::page_layout::SessionPages;

    /// A document laid out with the app's shaper.
    pub(crate) fn laid_out(document: &Document) -> DocumentLayout {
        let mut pages = SessionPages::default();
        pages.update(document);
        pages.layout().clone()
    }

    #[test]
    fn every_page_is_drawn_at_its_layouts_size() {
        let document = crate::page_surface::page_fixture();
        let layout = laid_out(&document);
        assert!(layout.pages.len() > 1);
        let pdf = document_pdf(&document, &layout, None, &PdfOptions::default()).unwrap();
        let read = hayro::hayro_syntax::Pdf::new(pdf).expect("a PDF hayro reads");
        assert_eq!(read.pages().len(), layout.pages.len());
        for (page, laid) in read.pages().iter().zip(&layout.pages) {
            let (width, height) = page.render_dimensions();
            assert!((width - laid.width / 20.0).abs() < 0.01, "{width}");
            assert!((height - laid.height / 20.0).abs() < 0.01, "{height}");
        }
    }

    #[test]
    fn an_export_writes_the_pdf_over_a_file_there() {
        let document = crate::page_surface::page_fixture();
        let layout = laid_out(&document);
        let folder = std::env::temp_dir().join(format!("amipro-export-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("Fixture.pdf");
        std::fs::write(&path, b"an older file").unwrap();
        let exported = ExportJob {
            job: PdfJob {
                set: Vec::new(),
                document,
                layout: layout.clone(),
                folder: None,
                options: PdfOptions::default(),
                labels: None,
            },
            path: path.clone(),
        }
        .run()
        .unwrap();
        assert_eq!(exported.pages, layout.pages.len());
        assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF-"));
        assert_eq!(
            exported.message(),
            format!("Exported {} pages to Fixture.pdf", layout.pages.len())
        );
        // Nothing but the PDF is left in the folder.
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// With notes adds pages of the notes under their pages' numbers, and
    /// every bookmark is a named destination (the notes run, NR3, NR6).
    #[test]
    fn with_notes_prints_the_notes_after_the_document_and_bookmarks_are_destinations() {
        use amipro_editing::references::NewBookmark;
        use amipro_editing::{Command, TextPosition, apply};
        let document = amipro_document::Document::from_plain_text("One two three.\nMore.");
        let at = |byte| TextPosition {
            flow: document.main_flow_id(),
            paragraph: 0,
            byte,
        };
        let document = apply(
            &document,
            &Command::InsertNote {
                at: at(3),
                note: amipro_document::NoteInfo {
                    time: 1_793_037_315,
                    initials: Some("LS".into()),
                    colour: None,
                    number: None,
                },
                text: "A note.".into(),
            },
        )
        .unwrap()
        .document;
        let document = apply(
            &document,
            &Command::AddBookmark {
                name: "place".into(),
                target: NewBookmark::Text {
                    start: at(0),
                    end: at(3),
                },
            },
        )
        .unwrap()
        .document;
        let layout = laid_out(&document);
        let plain = document_pdf(&document, &layout, None, &PdfOptions::default()).unwrap();
        let with_notes = document_pdf(
            &document,
            &layout,
            None,
            &PdfOptions {
                with_notes: true,
                ..PdfOptions::default()
            },
        )
        .unwrap();
        let count = |bytes: &[u8]| {
            hayro::hayro_syntax::Pdf::new(bytes.to_vec())
                .unwrap()
                .pages()
                .len()
        };
        assert_eq!(count(&with_notes), count(&plain) + 1);
        assert!(String::from_utf8_lossy(&plain).contains("place"));
    }

    #[test]
    fn the_pdf_is_named_after_the_documents_file() {
        assert_eq!(
            pdf_name(
                &DocumentLocation::LegacySource(PathBuf::from("/archive/README31.SAM")),
                "Readme"
            ),
            "README31.pdf"
        );
        assert_eq!(
            pdf_name(
                &DocumentLocation::Native(PathBuf::from("/notes/Plan.amipro")),
                ""
            ),
            "Plan.pdf"
        );
        assert_eq!(
            pdf_name(&DocumentLocation::Untitled, "Untitled 2"),
            "Untitled 2.pdf"
        );
        assert_eq!(pdf_name(&DocumentLocation::Untitled, " "), "Untitled.pdf");
    }

    #[test]
    fn crop_marks_are_ami_pros_seven_sixteenths_ending_a_sixteenth_short() {
        // `PRINT_NOTES.md`: the page ½ in right and down; on Letter paper
        // only the top-left mark lands on it (VM PFC1C: a line at ½ in from
        // the paper's edge to 263 dots of 600, 7/16 in).
        let marks = crop_marks(12_240.0, 15_840.0);
        assert_eq!(marks.len(), 8);
        assert!(marks.contains(&[0.0, 720.0, 630.0, 720.0]));
        assert!(marks.contains(&[720.0, 0.0, 720.0, 630.0]));
        // The bottom-right corner's: from 1/16 in past the corner.
        assert!(marks.contains(&[12_960.0 + 90.0, 16_560.0, 12_960.0 + 720.0, 16_560.0]));
        assert!(marks.contains(&[12_960.0, 16_560.0 + 90.0, 12_960.0, 16_560.0 + 720.0]));
        for [x1, y1, x2, y2] in marks {
            assert_eq!((x2 - x1).abs() + (y2 - y1).abs(), 630.0);
        }
    }

    #[test]
    fn a_pdf_with_crop_marks_has_the_page_in_a_half_inch_border_and_a_trim_box() {
        let document = crate::page_surface::page_fixture();
        let layout = laid_out(&document);
        let options = PdfOptions {
            crop_marks: Some(CropMarks::AroundPage),
            pages: Some(vec![0]),
            ..PdfOptions::default()
        };
        let pdf = document_pdf(&document, &layout, None, &options).unwrap();
        assert!(pdf.windows(8).any(|window| window == b"/TrimBox"));
        let read = hayro::hayro_syntax::Pdf::new(pdf).unwrap();
        let (width, height) = read.pages()[0].render_dimensions();
        assert!((width - (layout.pages[0].width / 20.0 + 72.0)).abs() < 0.01);
        assert!((height - (layout.pages[0].height / 20.0 + 72.0)).abs() < 0.01);
    }

    #[test]
    fn the_cover_sheet_lists_doc_info_as_ami_pro_does() {
        // VM PFC1D: labels padded to 20 characters, the fields by their
        // names, then the description and the imported files.
        let mut document = crate::page_surface::page_fixture();
        let info = document.info_mut();
        info.description = "The quarterly letter".into();
        info.fields[0] = "Acme".into();
        info.field_names[0] = "Client".into();
        info.revisions = 2;
        info.editing_minutes = 0;
        // 17 October 2026, 1:45 PM in Ami Pro's time base.
        info.created = Some(
            amipro_document::WallClock {
                year: 2026,
                month: 10,
                day: 17,
                hour: 13,
                minute: 45,
                second: 0,
            }
            .to_ami_pro(),
        );
        info.saved = info.created;
        let cover = CoverSheet::of(&document, Some(Path::new("/lab/FC1.SAM")));
        assert_eq!(
            cover.lines,
            [
                "Filename:           FC1.SAM",
                "Path:               /lab",
                "Style Sheet:        None",
                "Date Created:       10/17/2026",
                "Time Created:        1:45 PM",
                "Date Last Rev:      10/17/2026",
                "Total Editing Time: 0",
                "Total Revisions:    2",
                "Client:             Acme",
                "Field2:             ",
                "Field3:             ",
                "Field4:             ",
                "Field5:             ",
                "Field6:             ",
                "Field7:             ",
                "Field8:             ",
            ]
        );
        assert_eq!(cover.description, "The quarterly letter");
        // One sheet more, before the document.
        let layout = laid_out(&document);
        let options = PdfOptions {
            cover: Some(cover),
            ..PdfOptions::default()
        };
        let pdf = document_pdf(&document, &layout, None, &options).unwrap();
        let read = hayro::hayro_syntax::Pdf::new(pdf).unwrap();
        assert_eq!(read.pages().len(), layout.pages.len() + 1);
    }

    #[test]
    fn bookmarks_nest_as_the_outline_levels_do() {
        #[derive(Debug, PartialEq)]
        struct Node(&'static str, Vec<Node>);
        let nodes = nested(
            vec![
                (1, Node("1", vec![])),
                (2, Node("1.1", vec![])),
                (3, Node("1.1.1", vec![])),
                (2, Node("1.2", vec![])),
                (1, Node("2", vec![])),
                (3, Node("2..1", vec![])),
            ],
            |parent: &mut Node, child| parent.1.push(child),
        );
        assert_eq!(
            nodes,
            [
                Node(
                    "1",
                    vec![
                        Node("1.1", vec![Node("1.1.1", vec![])]),
                        Node("1.2", vec![])
                    ]
                ),
                Node("2", vec![Node("2..1", vec![])]),
            ]
        );
    }

    #[test]
    fn page_labels_are_ami_pros_page_numbers() {
        // A footer field in lower-case roman numerals starting at 3 on page 2
        // labels page 2 "iii"; page 1, where it shows nothing, has none.
        let document = crate::page_surface::page_fixture();
        let layout = laid_out(&document);
        let header = document.document_layout().header.right.unwrap();
        let amipro_document::FrameContent::Text(flow) = document.frame(header).unwrap().content
        else {
            unreachable!()
        };
        let document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::InsertPageNumber {
                at: amipro_editing::TextPosition {
                    flow,
                    paragraph: 0,
                    byte: 0,
                },
                field: amipro_document::PageNumberField {
                    style: 'i',
                    start_page: 2,
                    start_number: 3,
                    leading_text: String::new(),
                },
            },
        )
        .unwrap()
        .document;
        let labels = page_labels(&document, &layout, &[0, 1]);
        assert_eq!(labels[0], None);
        assert_eq!(
            labels[1],
            Some(krilla::page::PageLabel::new(
                Some(krilla::page::NumberingStyle::LowerRoman),
                None,
                std::num::NonZeroU32::new(3),
            ))
        );
    }

    #[test]
    fn an_archival_pdf_declares_pdf_a_2b() {
        let document = crate::page_surface::page_fixture();
        let layout = laid_out(&document);
        let options = PdfOptions {
            pages: Some(vec![0]),
            additions: PdfAdditions {
                archival: true,
                ..PdfAdditions::default()
            },
            ..PdfOptions::default()
        };
        let pdf = document_pdf(&document, &layout, None, &options).unwrap();
        let contains = |needle: &[u8]| pdf.windows(needle.len()).any(|window| window == needle);
        assert!(contains(b"pdfaid:part"), "the PDF/A identification");
    }

    /// The colours of what was drawn on page 1 of the fixture, in `mode`.
    fn page_one(document: &Document, mode: Option<PrintMode>) -> (Vec<Color>, Vec<String>) {
        let layout = laid_out(document);
        let mut drawn = (Vec::new(), Vec::new());
        draw_pages(
            document,
            &layout,
            None,
            mode,
            &[0],
            |_, renderer: &mut PdfRenderer| {
                drawn = (
                    renderer
                        .quads()
                        .into_iter()
                        .filter_map(|(_, background)| match background {
                            cosmic::iced::Background::Color(colour) => Some(colour),
                            _ => None,
                        })
                        .collect(),
                    renderer.texts(),
                );
            },
        );
        drawn
    }

    /// TD.5: a numeric cell prints (and exports to PDF) as its style
    /// shows it, the stored text unchanged; a text cell prints as typed.
    #[test]
    fn numbers_print_through_the_styles_number_format_td5() {
        let (mut document, _, _, flows) = crate::page_surface::cell_picture_fixture();
        let style = document.flow(flows[0]).unwrap().paragraphs[0].style;
        document
            .styles_mut()
            .get_mut(&style)
            .unwrap()
            .kept
            .number_format = amipro_document::NumberFormat {
            flags: 0x198,
            kind: amipro_document::NumberKind::Currency,
            ..Default::default()
        };
        for (flow, text) in [(flows[0], "1234.5"), (flows[1], "$1,200.00")] {
            document
                .flow_mut(flow)
                .unwrap()
                .paragraph_mut(0)
                .unwrap()
                .text = text.into();
        }
        let (_, texts) = page_one(&document, Some(PrintMode::default()));
        assert!(texts.iter().any(|text| text == "$1,234.50"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "$1,200.00"), "{texts:?}");
        assert!(!texts.iter().any(|text| text == "1234.5"), "{texts:?}");
        let layout = laid_out(&document);
        let options = PdfOptions {
            pages: Some(vec![0]),
            ..PdfOptions::default()
        };
        assert!(document_pdf(&document, &layout, None, &options).is_ok());
    }

    #[test]
    fn paper_shows_no_screen_marks_and_ami_pros_options_take_their_parts() {
        let document = crate::page_surface::page_fixture();
        let (screen, screen_text) = page_one(&document, None);
        assert!(screen.contains(&crate::page_surface::PAGE_SHADOW));
        assert!(screen.contains(&crate::page_surface::MARGIN_SHADE));
        let (paper, paper_text) = page_one(&document, Some(PrintMode::default()));
        assert!(!paper.contains(&crate::page_surface::PAGE_SHADOW));
        assert!(!paper.contains(&crate::page_surface::MARGIN_SHADE));
        assert_eq!(paper_text, screen_text, "the same text");
        assert!(paper_text.iter().any(|text| text.contains("Frame é text")));
        // The fixture's frame: its black lines on paper, none on a
        // preprinted form, where its text still prints.
        let black = |colours: &[Color]| {
            colours
                .iter()
                .filter(|colour| **colour == Color::BLACK)
                .count()
        };
        let (form, form_text) = page_one(
            &document,
            Some(PrintMode {
                preprinted_form: true,
                ..PrintMode::default()
            }),
        );
        assert!(
            black(&paper) > black(&form),
            "{} {}",
            black(&paper),
            black(&form)
        );
        assert_eq!(form_text, paper_text);
        // The Print panel's preview: the pages on white paper with their
        // shadow, no shaded margins.
        let (preview, preview_text) = page_one(
            &document,
            Some(PrintMode {
                preview: true,
                ..PrintMode::default()
            }),
        );
        assert!(preview.contains(&crate::page_surface::PAGE_SHADOW));
        assert!(!preview.contains(&crate::page_surface::MARGIN_SHADE));
        assert_eq!(preview_text, paper_text);
    }

    /// `AMIPRO_PDF_SAM=FILE AMIPRO_PDF_OUT=FILE.pdf`: any SAM file exported
    /// as the app exports it (`AMIPRO_STYLES` for its style sheet).
    #[test]
    #[ignore = "needs AMIPRO_PDF_SAM and AMIPRO_PDF_OUT"]
    fn pdf_export() {
        let (Some(path), Some(out)) = (
            std::env::var_os("AMIPRO_PDF_SAM"),
            std::env::var_os("AMIPRO_PDF_OUT"),
        ) else {
            eprintln!("AMIPRO_PDF_SAM or AMIPRO_PDF_OUT is not set; skipping");
            return;
        };
        let styles: Vec<std::path::PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let document = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            .unwrap()
            .document;
        let layout = laid_out(&document);
        let pictures = Pictures::default();
        let folder = Path::new(&path).parent();
        let started = std::time::Instant::now();
        // `AMIPRO_PDF_OPTIONS`: any of crop, cover, notes, form, nopictures, pdfa.
        let wanted = std::env::var("AMIPRO_PDF_OPTIONS").unwrap_or_default();
        let options = PrintOptions {
            crop_marks: wanted.contains("crop"),
            without_pictures: wanted.contains("nopictures"),
            doc_description: wanted.contains("cover"),
            with_notes: wanted.contains("notes"),
            preprinted_form: wanted.contains("form"),
            update_fields: false,
        }
        .pdf_options(&document, Some(Path::new(&path)), None);
        let options = PdfOptions {
            additions: PdfAdditions {
                archival: wanted.contains("pdfa"),
                ..options.additions
            },
            ..options
        };
        let pdf = document_pdf(&document, &layout, Some((&pictures, folder)), &options).unwrap();
        eprintln!(
            "{} pages, {} bytes, {:.1} ms",
            layout.pages.len(),
            pdf.len(),
            started.elapsed().as_secs_f64() * 1_000.0
        );
        std::fs::write(out, pdf).unwrap();
    }

    /// A page as the page surface draws it for paper with tiny-skia, at
    /// `dpi`: RGB rows.
    fn screen_page(
        document: &Document,
        layout: &DocumentLayout,
        pictures: (&Pictures, Option<&Path>),
        page: usize,
        dpi: f32,
    ) -> (u32, u32, Vec<u8>) {
        use cosmic::iced::advanced::Renderer as _;
        let scale = dpi / 1_440.0;
        let mut view = print_view(layout);
        view.scale = scale;
        let mut y = 0.0;
        for (rectangle, laid) in view.pages.iter_mut().zip(&layout.pages) {
            *rectangle = Rectangle::new(
                Point::new(0.0, y),
                Size::new(laid.width * scale, laid.height * scale),
            );
            y += rectangle.height + 100.0;
        }
        let selection = PageSelection::start(document);
        let mut surface =
            page_surface(document, layout, view.clone(), 0, selection, |_| (), |_| ())
                // The preview's drawing: the pictures themselves, not the
                // PDF's stand-ins.
                .printing(Some(PrintMode {
                    preview: true,
                    ..PrintMode::default()
                }))
                .pictures(pictures.0, pictures.1);
        let mut renderer = iced_tiny_skia::Renderer::new(
            cosmic::iced::Font::default(),
            cosmic::iced::Pixels(14.0),
        );
        let mut tree =
            widget::Tree::new(&surface as &dyn Widget<(), cosmic::Theme, iced_tiny_skia::Renderer>);
        let node = Widget::<(), cosmic::Theme, iced_tiny_skia::Renderer>::layout(
            &mut surface,
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::INFINITE),
        );
        let paper = view.pages[page];
        let (width, height) = (paper.width.ceil() as u32, paper.height.ceil() as u32);
        renderer.reset(Rectangle::with_size(Size::new(width as f32, height as f32)));
        renderer.with_translation(cosmic::iced::Vector::new(-paper.x, -paper.y), |renderer| {
            Widget::<(), cosmic::Theme, iced_tiny_skia::Renderer>::draw(
                &surface,
                &tree,
                renderer,
                &cosmic::Theme::light(),
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &paper,
            );
        });
        let viewport = cosmic::iced::advanced::graphics::Viewport::with_physical_size(
            Size::new(width, height),
            1.0,
        );
        let rgba = iced_tiny_skia::window::compositor::screenshot(
            &mut renderer,
            &viewport,
            cosmic::iced::Color::WHITE,
        );
        let rgb = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| pixel[..3].to_vec())
            .collect();
        (width, height, rgb)
    }

    /// Line numbering (CA.13) is in the PDF: ink in the left margin beside
    /// each number of page 1, none there with numbering off.
    #[test]
    fn line_numbers_are_drawn_in_the_pdf() {
        use amipro_document::{LineNumberingEvery, LineNumberingMode};
        let plain = Document::from_plain_text(
            &(1..=80)
                .map(|index| format!("Line {index} of the numbered document."))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let mut numbered = plain.clone();
        numbered.set_line_numbering(numbered.line_numbering().with(
            LineNumberingMode::TextLines,
            LineNumberingEvery::Line,
            false,
        ));
        let dpi = 72.0;
        let first_page = |document: &Document| {
            let layout = laid_out(document);
            let pdf = document_pdf(document, &layout, None, &PdfOptions::default()).unwrap();
            let read = hayro::hayro_syntax::Pdf::new(pdf).expect("a PDF");
            let image = pdf_page(&read.pages()[0], dpi);
            (layout, image)
        };
        let ink = |image: &(u32, u32, Vec<u8>),
                   columns: std::ops::Range<u32>,
                   rows: std::ops::Range<u32>| {
            let mut count = 0;
            for y in rows {
                for x in columns.clone() {
                    let at = (y as usize * image.0 as usize + x as usize) * 3;
                    let pixel = &image.2[at..at + 3];
                    if u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]) < 3 * 160 {
                        count += 1;
                    }
                }
            }
            count
        };
        let (layout, image) = first_page(&numbered);
        let numbers = &layout.line_numbers[0];
        assert!(numbers.len() > 30, "page 1 is numbered");
        let margin = layout.pages[0].columns[0].x;
        let px = |twips: f32| (twips / 20.0 * dpi / 72.0) as u32;
        for number in numbers {
            let rows = px(number.baseline - 200.0)..px(number.baseline) + 1;
            assert!(
                ink(&image, px(number.x)..px(margin), rows) > 0,
                "number {} drawn",
                number.text
            );
        }
        let band = px(margin / 4.0)..px(margin);
        let rows = 0..image.1;
        let (_, unnumbered) = first_page(&plain);
        assert_eq!(
            ink(&unnumbered, band.clone(), rows.clone()),
            0,
            "nothing in the margin when off"
        );
        assert!(ink(&image, band, rows) > 100);
    }

    /// A PDF page rendered by hayro at `dpi`, on white: RGB rows.
    fn pdf_page(page: &hayro::hayro_syntax::page::Page<'_>, dpi: f32) -> (u32, u32, Vec<u8>) {
        let cache = hayro::RenderCache::new();
        let pixmap = hayro::render(
            page,
            &cache,
            &hayro::hayro_interpret::InterpreterSettings::default(),
            &hayro::RenderSettings::default(),
            &hayro::PixmapSettings {
                x_scale: dpi / 72.0,
                y_scale: dpi / 72.0,
                bg_color: hayro::vello_cpu::color::palette::css::WHITE,
            },
        );
        let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        let rgb = pixmap
            .data_as_u8_slice()
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| pixel[..3].to_vec())
            .collect();
        (width, height, rgb)
    }

    /// Ink (darker than mid grey) in `a` with no ink of `b` within a pixel,
    /// over all ink in `a`.
    fn stray_ink(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> (usize, usize) {
        let ink = |image: &(u32, u32, Vec<u8>), x: i64, y: i64| {
            if x < 0 || y < 0 || x >= i64::from(image.0) || y >= i64::from(image.1) {
                return false;
            }
            let at = (y as usize * image.0 as usize + x as usize) * 3;
            let pixel = &image.2[at..at + 3];
            (u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2])) < 3 * 160
        };
        let (mut total, mut stray) = (0, 0);
        // The preview's paper edge is not on paper: two pixels all round
        // are left out.
        for y in 2..i64::from(a.1) - 2 {
            for x in 2..i64::from(a.0) - 2 {
                if !ink(a, x, y) {
                    continue;
                }
                total += 1;
                let near = (-1..=1).any(|dy| (-1..=1).any(|dx| ink(b, x + dx, y + dy)));
                if !near {
                    stray += 1;
                }
            }
        }
        (total, stray)
    }

    /// `AMIPRO_COMPARE_PDF=DIR` (and `AMIPRO_STYLES`, `AMIPRO_COMPARE_LIMIT`
    /// files, default 30, `AMIPRO_COMPARE_OUT=DIR` for the worst pages'
    /// images): each document's first pages drawn by the page surface and
    /// by a PDF renderer from the app's PDF, at 100 dpi, and the ink that
    /// does not coincide counted (aggregate).
    #[test]
    #[ignore = "needs AMIPRO_COMPARE_PDF"]
    fn pdf_against_screen() {
        let Some(directory) = std::env::var_os("AMIPRO_COMPARE_PDF") else {
            eprintln!("AMIPRO_COMPARE_PDF is not set; skipping");
            return;
        };
        let limit: usize = std::env::var("AMIPRO_COMPARE_LIMIT")
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or(30);
        let out = std::env::var_os("AMIPRO_COMPARE_OUT").map(PathBuf::from);
        let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let mut files = Vec::new();
        let mut stack = vec![PathBuf::from(directory)];
        while let Some(folder) = stack.pop() {
            for entry in std::fs::read_dir(folder).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
                {
                    files.push(path);
                }
            }
        }
        files.sort();
        let dpi = 100.0;
        let mut results: Vec<(f64, f64, String, usize, String)> = Vec::new();
        for path in files.into_iter().take(limit) {
            let Ok(import) = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            else {
                continue;
            };
            let document = import.document;
            let layout = laid_out(&document);
            let pictures = Pictures::default();
            let folder = path.parent();
            let options = PdfOptions {
                pages: Some((0..layout.pages.len().min(3)).collect()),
                ..PdfOptions::default()
            };
            let Ok(pdf) = document_pdf(&document, &layout, Some((&pictures, folder)), &options)
            else {
                eprintln!("no PDF: {}", path.display());
                continue;
            };
            let read = hayro::hayro_syntax::Pdf::new(pdf).expect("a PDF hayro reads");
            for (index, page) in read.pages().iter().enumerate() {
                let printed = pdf_page(page, dpi);
                let screen = screen_page(&document, &layout, (&pictures, folder), index, dpi);
                let (screen_ink, screen_stray) = stray_ink(&screen, &printed);
                let (printed_ink, printed_stray) = stray_ink(&printed, &screen);
                let share = |stray: usize, total: usize| {
                    if total == 0 {
                        0.0
                    } else {
                        stray as f64 / total as f64
                    }
                };
                let stem = format!("{:03}-{index}", results.len() + 1);
                results.push((
                    share(screen_stray, screen_ink),
                    share(printed_stray, printed_ink),
                    path.display().to_string(),
                    index,
                    stem.clone(),
                ));
                if let Some(out) = &out {
                    let write = |name: String, image: &(u32, u32, Vec<u8>)| {
                        let mut ppm = format!("P6\n{} {}\n255\n", image.0, image.1).into_bytes();
                        ppm.extend(&image.2);
                        std::fs::write(out.join(name), ppm).unwrap();
                    };
                    write(format!("{stem}-screen.ppm"), &screen);
                    write(format!("{stem}-pdf.ppm"), &printed);
                }
            }
        }
        let pages = results.len().max(1) as f64;
        let mean = |pick: fn(&(f64, f64, String, usize, String)) -> f64| {
            results.iter().map(pick).sum::<f64>() / pages
        };
        let within = |limit: f64| {
            results
                .iter()
                .filter(|result| result.0 <= limit && result.1 <= limit)
                .count()
        };
        eprintln!(
            "{} pages: screen ink missing from the PDF {:.2}% on average, PDF ink missing from the screen {:.2}%; pages within 1%: {}, within 5%: {}",
            results.len(),
            100.0 * mean(|result| result.0),
            100.0 * mean(|result| result.1),
            within(0.01),
            within(0.05),
        );
        results.sort_by(|a, b| (b.0 + b.1).total_cmp(&(a.0 + a.1)));
        for (screen, printed, file, page, stem) in results.iter().take(8) {
            eprintln!(
                "  worst: {:.1}% / {:.1}%  page {} of {file} ({stem})",
                100.0 * screen,
                100.0 * printed,
                page + 1
            );
        }
    }

    /// `AMIPRO_PERF_SAM=FILE` (release): the time to draw a document's PDF
    /// (pictures decoded, fonts embedded), five times; and for one page.
    #[test]
    #[ignore = "release-mode performance baseline"]
    fn pdf_performance_baseline() {
        let document = std::env::var_os("AMIPRO_PERF_SAM")
            .map_or_else(crate::page_surface::page_fixture, |path| {
                amipro_formats::import_sam_path(path).unwrap().document
            });
        let layout = laid_out(&document);
        let mut whole = Vec::new();
        let mut one = Vec::new();
        for _ in 0..5 {
            let pictures = Pictures::default();
            let started = std::time::Instant::now();
            let pdf = document_pdf(
                &document,
                &layout,
                Some((&pictures, None)),
                &PdfOptions::default(),
            )
            .unwrap();
            whole.push(started.elapsed());
            assert!(!pdf.is_empty());
            let started = std::time::Instant::now();
            let options = PdfOptions {
                pages: Some(vec![0]),
                ..PdfOptions::default()
            };
            document_pdf(&document, &layout, Some((&pictures, None)), &options).unwrap();
            one.push(started.elapsed());
        }
        whole.sort();
        one.sort();
        eprintln!(
            "{} pages: the PDF {:.1} ms median ({:.1} worst); one page {:.1} ms",
            layout.pages.len(),
            whole[2].as_secs_f64() * 1_000.0,
            whole[4].as_secs_f64() * 1_000.0,
            one[2].as_secs_f64() * 1_000.0,
        );
    }
}
