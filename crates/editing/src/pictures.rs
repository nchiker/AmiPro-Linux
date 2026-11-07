//! Picture commands (`docs/Design/UI_SPEC.md` §25, decisions P1–P8;
//! `docs/research/PICTURE_NOTES.md`): a picture put into an empty frame or
//! an empty table cell, as Ami Pro's Import Picture does (`10d8:126c`,
//! `14d0:05ec`); Graphics Scaling (`1168:0000`); moving a picture in its
//! frame (picture mode); Replace Picture and a description, which Ami Pro
//! does not have. A picture in a new frame at the caret is `CreateFrame`
//! with picture content.

use std::sync::Arc;

use amipro_document::{
    CellContent, Document, FlowId, Frame, FrameContent, FrameId, FrameStyle, LegacyData, Offset,
    PICTURE_MAX, Picture, PictureScaling, PictureSource, Placement, Size,
};

use crate::{Applied, ChangeReport, EditError};

/// A picture's bytes or the file it links to.
#[derive(Clone, Debug, PartialEq)]
pub enum NewPictureSource {
    /// Bytes copied into the document (Ami Pro's Copy image).
    Embedded(Arc<[u8]>),
    /// A file the picture is drawn from (decision P2).
    Linked(String),
}

/// A picture to put somewhere, as Import Picture leaves it: Fit in frame
/// with the aspect ratio kept.
#[derive(Clone, Debug, PartialEq)]
pub struct NewPicture {
    pub source: NewPictureSource,
    /// The file's extension, such as `.png`.
    pub extension: String,
    /// The size the picture is drawn at, kept as Ami Pro keeps its last
    /// rectangle.
    pub size: Size,
}

/// Where Insert › Image… puts a picture other than a new frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PictureTarget {
    /// An empty text frame, which becomes the picture's frame (P02).
    Frame(FrameId),
    /// The empty cell at `cell` of the table in `table` (`14d0:05ec`, T3).
    Cell { table: FrameId, cell: usize },
}

impl NewPicture {
    fn into_picture(self, document: &mut Document) -> Picture {
        let source = match self.source {
            NewPictureSource::Embedded(bytes) => {
                PictureSource::Embedded(document.add_object(bytes))
            }
            NewPictureSource::Linked(path) => PictureSource::Linked(path),
        };
        fresh(source, self.extension, self.size)
    }

    fn check(&self) -> Result<(), EditError> {
        match &self.source {
            NewPictureSource::Embedded(bytes) if bytes.is_empty() => Err("a picture has no bytes"),
            NewPictureSource::Linked(path) if path.is_empty() => {
                Err("a linked picture has no path")
            }
            _ => Ok(()),
        }
    }
}

/// A picture as Ami Pro leaves a new one: Fit in frame with the aspect
/// kept (Import Picture, PI1), but a drawing at its own size (Tools ›
/// Drawing, DR1).
pub(crate) fn fresh(source: PictureSource, extension: impl Into<String>, size: Size) -> Picture {
    let mut picture = Picture::new(source, extension, size);
    // Ami Pro's own drawings and charts are Original, with no aspect ratio
    // (CHV2: flags 1 on a chart frame).
    if matches!(
        picture.format(),
        amipro_document::PictureFormat::Drawing | amipro_document::PictureFormat::Chart
    ) {
        picture.scaling.sizing = amipro_document::PictureSizing::Original;
        picture.scaling.keep_aspect = false;
    }
    picture
}

fn unchanged(document: &Document) -> Applied {
    Applied {
        document: document.clone(),
        caret: None,
        report: ChangeReport::default(),
    }
}

pub(crate) fn changed(document: Document, frame: FrameId, mut report: ChangeReport) -> Applied {
    report.frames.insert(frame);
    Applied {
        document,
        caret: None,
        report,
    }
}

pub(crate) fn picture_of(document: &Document, frame: FrameId) -> Result<&Picture, EditError> {
    match &document.frame(frame).ok_or("frame does not exist")?.content {
        FrameContent::Picture(picture) => Ok(picture),
        _ => Err("the frame holds no picture"),
    }
}

/// Changes `frame`'s picture with `change`, as one command.
fn change_picture(
    document: &Document,
    frame: FrameId,
    change: impl FnOnce(&mut Picture),
) -> Result<Applied, EditError> {
    let mut picture = picture_of(document, frame)?.clone();
    change(&mut picture);
    if picture == *picture_of(document, frame)? {
        return Ok(unchanged(document));
    }
    let mut next = document.clone();
    let Some(target) = next.frame_mut(frame) else {
        return Err("frame does not exist");
    };
    target.content = FrameContent::Picture(picture);
    Ok(changed(next, frame, ChangeReport::default()))
}

/// Graphics Scaling: how the picture is sized and rotated in its frame.
pub(crate) fn set_scaling(
    document: &Document,
    frame: FrameId,
    scaling: PictureScaling,
) -> Result<Applied, EditError> {
    if scaling.percent == 0 || scaling.rotation >= 360 {
        return Err("a picture's percentage is at least 1 and its rotation below 360");
    }
    if scaling.sizing == amipro_document::PictureSizing::Custom
        && !((1..=PICTURE_MAX).contains(&scaling.size.width)
            && (1..=PICTURE_MAX).contains(&scaling.size.height))
    {
        return Err("a picture's size is 1 to 31,680 twips");
    }
    change_picture(document, frame, |picture| picture.scaling = scaling)
}

/// Picture mode: how far the picture is moved in its frame.
pub(crate) fn move_in_frame(
    document: &Document,
    frame: FrameId,
    pan: Offset,
) -> Result<Applied, EditError> {
    if pan.x.abs() > 4 * PICTURE_MAX || pan.y.abs() > 4 * PICTURE_MAX {
        return Err("a picture is moved too far in its frame");
    }
    change_picture(document, frame, |picture| picture.pan = pan)
}

pub(crate) fn set_description(
    document: &Document,
    frame: FrameId,
    description: &str,
) -> Result<Applied, EditError> {
    change_picture(document, frame, |picture| {
        picture.description = description.to_owned();
    })
}

/// Image Processing: the picture's adjustments, non-destructive. A
/// gray-scale TIFF changes to Ami Pro's `.tgf` form (the same bytes; the
/// kept record, which described the old driver, goes and the writer makes a
/// new one with the state).
pub(crate) fn set_adjust(
    document: &Document,
    frame: FrameId,
    adjust: amipro_document::ImageAdjust,
    gray_tiff: bool,
) -> Result<Applied, EditError> {
    if adjust != adjust.clamped() {
        return Err("image adjustments are from 0 to 100");
    }
    change_picture(document, frame, |picture| {
        if gray_tiff && !picture.is_gray_tiff() {
            picture.extension = ".tgf".into();
            picture.legacy.values.retain(|value| value.key != "isd");
            picture.rendering = None;
        }
        picture.adjust = adjust;
    })
}

/// Replace Picture (P7): the frame, its scaling, and the picture's place in
/// it stay; the picture, its kind, and Ami Pro's kept record and screen
/// image (which described the old one) go, and so does its description.
pub(crate) fn replace(
    document: &Document,
    frame: FrameId,
    picture: &NewPicture,
) -> Result<Applied, EditError> {
    picture.check()?;
    let old = picture_of(document, frame)?.clone();
    let mut next = document.clone();
    let mut new = picture.clone().into_picture(&mut next);
    new.scaling = old.scaling;
    new.pan = old.pan;
    let Some(target) = next.frame_mut(frame) else {
        return Err("frame does not exist");
    };
    target.content = FrameContent::Picture(new);
    next.drop_unreferenced_objects();
    Ok(changed(next, frame, ChangeReport::default()))
}

/// Sets an equation frame's stored text (DO.9): the picture is the new
/// two-line object, drawn at `natural` twips with `baseline` as its ascent
/// (Ami Pro keeps the width and height in the scaling and the baseline in
/// the pan, `OBJECTS_NOTES.md` §12.2); with `fit_frame` the frame takes
/// that size too. Ami Pro's kept record and screen image described the old
/// text, so they go. One step.
pub(crate) fn set_equation(
    document: &Document,
    frame: FrameId,
    bytes: &Arc<[u8]>,
    natural: Size,
    baseline: i32,
    fit_frame: bool,
) -> Result<Applied, EditError> {
    if bytes.is_empty() {
        return Err("an equation has no bytes");
    }
    if natural.width <= 0 || natural.height <= 0 {
        return Err("an equation has no size");
    }
    let old = picture_of(document, frame)?.clone();
    if !old.extension.eq_ignore_ascii_case(".tex") {
        return Err("the frame holds no equation");
    }
    let mut next = document.clone();
    let mut picture = fresh(
        PictureSource::Embedded(next.add_object(Arc::clone(bytes))),
        ".tex",
        natural,
    );
    picture.scaling = PictureScaling::imported(natural);
    picture.scaling.sizing = amipro_document::PictureSizing::Original;
    picture.scaling.keep_aspect = false;
    picture.pan = Offset { x: 0, y: baseline };
    picture.description = old.description;
    let target = next.frame_mut(frame).ok_or("frame does not exist")?;
    target.content = FrameContent::Picture(picture);
    next.drop_unreferenced_objects();
    if !fit_frame {
        return Ok(changed(next, frame, ChangeReport::default()));
    }
    let current = next.frame(frame).ok_or("frame does not exist")?.placement;
    let rect = match current {
        Placement::Fixed { rect, .. } | Placement::Repeating { rect, .. } => {
            amipro_document::Rect::new(
                rect.left,
                rect.top,
                rect.left + natural.width,
                rect.top + natural.height,
            )
        }
        Placement::BelowParagraph { .. } | Placement::Inline { .. } => {
            amipro_document::Rect::new(0, 0, natural.width, natural.height)
        }
        _ => return Ok(changed(next, frame, ChangeReport::default())),
    };
    let resized = crate::frames::resize_frame(&next, frame, rect)?;
    Ok(resized)
}

/// Whether a flow is one empty paragraph, as a new frame's or cell's text.
fn empty_flow(document: &Document, flow: FlowId) -> bool {
    document.flow(flow).is_some_and(|flow| {
        matches!(&flow.paragraphs[..], [paragraph]
            if paragraph.text.is_empty() && paragraph.marks.is_empty())
    })
}

/// Import Picture with an empty frame selected or the caret in an empty
/// table cell: the picture goes into it. A frame must hold empty text (Ami
/// Pro disables Import Picture with a picture frame selected, PI9); a cell
/// must be empty and hold no frame (TF12), and its picture is a frame of
/// `cell_style` at the front of the frame stack, as `14d0:0000` puts it at
/// the front of Ami Pro's list of frames in cells.
pub(crate) fn insert(
    document: &Document,
    target: PictureTarget,
    picture: &NewPicture,
    cell_style: &FrameStyle,
) -> Result<Applied, EditError> {
    picture.check()?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    match target {
        PictureTarget::Frame(frame) => {
            let current = document.frame(frame).ok_or("frame does not exist")?;
            if matches!(
                current.placement,
                Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell
            ) {
                return Err("a header, footer, or cell frame cannot take a picture");
            }
            let FrameContent::Text(flow) = current.content else {
                return Err("only an empty text frame takes a picture");
            };
            if !empty_flow(document, flow) {
                return Err("only an empty text frame takes a picture");
            }
            let new = picture.clone().into_picture(&mut next);
            let Some(target) = next.frame_mut(frame) else {
                return Err("frame does not exist");
            };
            target.content = FrameContent::Picture(new);
            // A picture frame keeps no `[txt]` marks.
            target.legacy.values.retain(|value| value.key != "txt");
            next.remove_flow(flow);
            report.removed_flows.push(flow);
            Ok(changed(next, frame, report))
        }
        PictureTarget::Cell { table, cell } => {
            let FrameContent::Table(grid) =
                &document.frame(table).ok_or("frame does not exist")?.content
            else {
                return Err("the frame holds no table");
            };
            let target_cell = grid.cells.get(cell).ok_or("cell does not exist")?;
            if target_cell.span.is_covered() {
                return Err("a joined cell covers this cell");
            }
            if target_cell.frame.is_some() {
                return Err("the cell holds a picture already");
            }
            let CellContent::Text(flow) = target_cell.content;
            if !empty_flow(document, flow) {
                return Err("only an empty cell takes a picture");
            }
            let new = picture.clone().into_picture(&mut next);
            let id = next.allocate_frame_id();
            next.insert_frame(Frame {
                id,
                placement: Placement::InCell,
                style: cell_style.clone(),
                group: None,
                content: FrameContent::Picture(new),
                legacy: LegacyData::default(),
            });
            next.frame_stack_mut().insert(0, id);
            let Some(FrameContent::Table(grid)) =
                next.frame_mut(table).map(|frame| &mut frame.content)
            else {
                return Err("frame does not exist");
            };
            grid.cells[cell].frame = Some(id);
            report.frames.insert(table);
            Ok(changed(next, id, report))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, History, NewFrameContent, apply};
    use amipro_document::{PictureSizing, Rect};

    fn new_picture(bytes: &[u8], extension: &str) -> NewPicture {
        NewPicture {
            source: NewPictureSource::Embedded(Arc::from(bytes)),
            extension: extension.into(),
            size: Size {
                width: 600,
                height: 400,
            },
        }
    }

    /// A document with an empty text frame and a 2 × 1 table frame on page
    /// one; returns them.
    fn frames() -> (Document, FrameId, FrameId) {
        let mut document = Document::from_plain_text("Text");
        let mut create = |content| {
            let applied = apply(
                &document,
                &Command::CreateFrame {
                    placement: Placement::Fixed {
                        page: 1,
                        rect: Rect::new(1_440, 1_440, 2_880, 2_880),
                    },
                    style: FrameStyle::default(),
                    content,
                    anchor: None,
                    layout: None,
                },
            )
            .unwrap();
            document = applied.document;
            applied.report.frames.into_iter().next().unwrap()
        };
        let text = create(NewFrameContent::Text);
        let table = create(NewFrameContent::Table {
            columns: vec![700, 700],
            rows: vec![300],
        });
        (document, text, table)
    }

    /// Import Picture with an empty frame selected (P02): the frame keeps
    /// its place and style and holds the picture, Fit in frame with the
    /// aspect ratio kept; its text goes. One undo step.
    #[test]
    fn a_picture_goes_into_an_empty_frame() {
        let (document, text, _) = frames();
        let FrameContent::Text(flow) = document.frame(text).unwrap().content else {
            unreachable!();
        };
        let mut current = document.clone();
        let mut history = History::new();
        history
            .execute(
                &mut current,
                (),
                &Command::InsertPicture {
                    target: PictureTarget::Frame(text),
                    picture: new_picture(b"PNG", ".png"),
                    cell_style: FrameStyle::default(),
                },
            )
            .unwrap();
        current.validate().unwrap();
        let frame = current.frame(text).unwrap();
        let FrameContent::Picture(picture) = &frame.content else {
            panic!("no picture");
        };
        assert_eq!(picture.scaling.sizing, PictureSizing::FitInFrame);
        assert!(picture.scaling.keep_aspect);
        assert_eq!(picture.format(), amipro_document::PictureFormat::Png);
        assert!(current.flow(flow).is_none(), "the frame's text goes");
        // Not into a picture frame (PI9), nor into a frame with text.
        assert!(
            apply(
                &current,
                &Command::InsertPicture {
                    target: PictureTarget::Frame(text),
                    picture: new_picture(b"PNG", ".png"),
                    cell_style: FrameStyle::default(),
                },
            )
            .is_err()
        );
        history.undo(&mut current, ()).unwrap();
        assert_eq!(current, document);
    }

    /// Image Processing (DO.5): the adjustments are a picture setting, one
    /// undo step each; a gray-scale TIFF takes the `.tgf` form with the
    /// same bytes; values past 100 are refused.
    #[test]
    fn image_adjustments_are_one_step_and_a_gray_tiff_becomes_tgf() {
        use amipro_document::{Halftone, ImageAdjust};
        let (document, text, _) = frames();
        let mut current = document.clone();
        let mut history = History::new();
        let insert = Command::InsertPicture {
            target: PictureTarget::Frame(text),
            picture: new_picture(b"II*\0tiff", ".tif"),
            cell_style: FrameStyle::default(),
        };
        history.execute(&mut current, (), &insert).unwrap();
        let before = current.clone();
        let adjust = ImageAdjust {
            brightness: 70,
            contrast: 40,
            edge: 10,
            smoothing: 20,
            invert: true,
            halftone: Halftone::BestPictureQuality,
        };
        let set = |gray_tiff| Command::SetImageAdjust {
            frame: text,
            adjust,
            gray_tiff,
        };
        history.execute(&mut current, (), &set(false)).unwrap();
        let picture = picture_of(&current, text).unwrap().clone();
        assert_eq!(picture.adjust, adjust);
        assert_eq!(picture.extension, ".tif");
        history.execute(&mut current, (), &set(true)).unwrap();
        let picture = picture_of(&current, text).unwrap().clone();
        assert!(picture.is_gray_tiff() && picture.extension == ".tgf");
        let PictureSource::Embedded(object) = picture.source else {
            unreachable!();
        };
        assert_eq!(
            &*current.objects().get(&object).unwrap().bytes,
            b"II*\0tiff"
        );
        history.undo(&mut current, ()).unwrap();
        history.undo(&mut current, ()).unwrap();
        assert_eq!(current, before);
        assert!(
            apply(
                &current,
                &Command::SetImageAdjust {
                    frame: text,
                    adjust: ImageAdjust {
                        edge: 101,
                        ..adjust
                    },
                    gray_tiff: false,
                },
            )
            .is_err()
        );
    }

    /// Import Picture with the caret in an empty cell (`14d0:05ec`, T3):
    /// a frame in the cell at the front of the frame stack, of the given
    /// style; not into a cell with text or a picture (TF12). Deleting the
    /// table deletes it.
    #[test]
    fn a_picture_goes_into_an_empty_cell() {
        let (document, _, table) = frames();
        let style = FrameStyle {
            border: amipro_document::Border {
                sides: amipro_document::Sides::default(),
                ..FrameStyle::default().border
            },
            ..FrameStyle::default()
        };
        let insert = |document: &Document, cell| {
            apply(
                document,
                &Command::InsertPicture {
                    target: PictureTarget::Cell { table, cell },
                    picture: new_picture(b"BM", ".bmp"),
                    cell_style: style.clone(),
                },
            )
        };
        let applied = insert(&document, 1).unwrap();
        let next = applied.document;
        next.validate().unwrap();
        let picture = next.frame_stack()[0];
        assert_eq!(next.frame(picture).unwrap().placement, Placement::InCell);
        assert_eq!(next.frame(picture).unwrap().style, style);
        assert_eq!(next.cell_holding(picture).map(|(_, cell)| cell), Some(1));
        assert!(insert(&next, 1).is_err(), "the cell holds a picture");
        // A cell with text takes none.
        let FrameContent::Table(grid) = &next.frame(table).unwrap().content else {
            unreachable!();
        };
        let CellContent::Text(flow) = grid.cells[0].content;
        let typed = apply(
            &next,
            &Command::insert(
                crate::TextPosition {
                    flow,
                    paragraph: 0,
                    byte: 0,
                },
                "x",
            ),
        )
        .unwrap()
        .document;
        assert!(insert(&typed, 0).is_err());
        let deleted = apply(&next, &Command::DeleteFrame { frame: table })
            .unwrap()
            .document;
        assert!(deleted.frame(picture).is_none());
        deleted.validate().unwrap();
    }

    /// Replace Picture keeps the frame, the scaling, and the place in the
    /// frame; the kind, Ami Pro's kept record and screen image, and the
    /// description go with the old picture. Scaling, moving, and the
    /// description are one step each, refused out of Ami Pro's limits.
    #[test]
    fn replace_scale_move_and_describe_a_picture() {
        let (document, text, _) = frames();
        let mut current = apply(
            &document,
            &Command::InsertPicture {
                target: PictureTarget::Frame(text),
                picture: new_picture(b"BM", ".bmp"),
                cell_style: FrameStyle::default(),
            },
        )
        .unwrap()
        .document;
        let mut history = History::new();
        let scaling = PictureScaling {
            sizing: PictureSizing::Custom,
            keep_aspect: false,
            percent: 100,
            size: Size {
                width: 1_440,
                height: 360,
            },
            rotation: 0,
        };
        let commands = [
            Command::SetPictureScaling {
                frame: text,
                scaling,
            },
            Command::MovePictureInFrame {
                frame: text,
                pan: Offset { x: 936, y: 144 },
            },
            Command::SetPictureDescription {
                frame: text,
                description: "Logo".into(),
            },
            Command::ReplacePicture {
                frame: text,
                picture: new_picture(b"<svg/>", ".svg"),
            },
        ];
        let before = current.clone();
        for command in &commands {
            history.execute(&mut current, (), command).unwrap();
            current.validate().unwrap();
        }
        let FrameContent::Picture(picture) = &current.frame(text).unwrap().content else {
            unreachable!();
        };
        assert_eq!(picture.scaling, scaling);
        assert_eq!(picture.pan, Offset { x: 936, y: 144 });
        assert_eq!(picture.format(), amipro_document::PictureFormat::Svg);
        assert!(picture.description.is_empty());
        assert!(picture.legacy.values.is_empty() && picture.rendering.is_none());
        assert_eq!(current.objects().len(), 1, "the old bytes go");
        for _ in &commands {
            history.undo(&mut current, ()).unwrap();
        }
        assert_eq!(current, before);
        for refused in [
            PictureScaling {
                percent: 0,
                ..scaling
            },
            PictureScaling {
                rotation: 360,
                ..scaling
            },
            PictureScaling {
                size: Size {
                    width: 31_681,
                    height: 1,
                },
                ..scaling
            },
        ] {
            assert!(
                apply(
                    &current,
                    &Command::SetPictureScaling {
                        frame: text,
                        scaling: refused
                    }
                )
                .is_err()
            );
        }
    }
}
