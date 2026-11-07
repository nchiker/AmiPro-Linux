//! Drawings (`docs/Design/UI_SPEC.md` §33, decisions DW1–DW9;
//! `docs/research/DRAWING_NOTES.md`): a drawing is a picture whose bytes
//! are Ami Pro's object stream (`amipro_pictures::sdw`). Draw mode changes
//! its objects; [`Command::SetDrawing`](crate::Command::SetDrawing) puts the
//! changed objects back as one step, written as Ami Pro writes them, and
//! drops what described the old ones (Ami Pro's screen image and kept
//! driver state, which Ami Pro rebuilds; DRW0). The operations Draw mode
//! offers are pure functions on a drawing's objects, here.

use std::sync::Arc;

use amipro_document::{Document, FrameContent, FrameId, PictureFormat, PictureSource};
use amipro_pictures::sdw::{self, Affine, Drawing, Fill, Object, Pen, Rect, Shape, Text};

use crate::{Applied, ChangeReport, EditError};

/// The most objects a drawing may hold, as a bound on runaway edits.
pub const MAX_OBJECTS: usize = 10_000;

/// Puts `drawing` into `frame`'s drawing as one step.
pub(crate) fn set_drawing(
    document: &Document,
    frame: FrameId,
    drawing: &Drawing,
) -> Result<Applied, EditError> {
    let picture = crate::pictures::picture_of(document, frame)?;
    // A chart is a drawing whose objects the Charting panel generates
    // (DO.8); Draw mode may also edit them.
    if !matches!(
        picture.format(),
        PictureFormat::Drawing | PictureFormat::Chart
    ) {
        return Err("the frame holds no drawing");
    }
    if count(&drawing.objects) > MAX_OBJECTS {
        return Err("a drawing holds at most 10,000 objects");
    }
    let is_chart = picture.format() == PictureFormat::Chart;
    let bytes: Arc<[u8]> = sdw::write(drawing).into();
    if let PictureSource::Embedded(object) = &picture.source
        && document
            .objects()
            .get(object)
            .is_some_and(|object| object.bytes == bytes)
    {
        return Ok(Applied {
            document: document.clone(),
            caret: None,
            report: ChangeReport::default(),
        });
    }
    let mut next = document.clone();
    let source = PictureSource::Embedded(next.add_object(bytes));
    let Some(FrameContent::Picture(target)) = next.frame_mut(frame).map(|frame| &mut frame.content)
    else {
        return Err("frame does not exist");
    };
    target.source = source;
    // Ami Pro's screen image and driver state described the old objects.
    target.rendering = None;
    // A chart's `[isd]` names its driver (`.sct`), which is what makes the
    // frame a chart in a file Ami Pro wrote: it stays (DO.8).
    if !is_chart {
        target.legacy.values.retain(|value| value.key != "isd");
    }
    next.drop_unreferenced_objects();
    Ok(crate::pictures::changed(
        next,
        frame,
        ChangeReport::default(),
    ))
}

/// The objects of `frame`'s drawing, or `None` when it holds none that
/// reads.
pub fn drawing_of(document: &Document, frame: FrameId) -> Option<Drawing> {
    let FrameContent::Picture(picture) = &document.frame(frame)?.content else {
        return None;
    };
    if !matches!(
        picture.format(),
        PictureFormat::Drawing | PictureFormat::Chart
    ) {
        return None;
    }
    let PictureSource::Embedded(object) = &picture.source else {
        return None;
    };
    sdw::read(&document.objects().get(object)?.bytes)
}

/// What a new, empty drawing measures each way (3 in, `LIBSIZE`).
pub fn empty_drawing_size() -> i32 {
    sdw::EMPTY_SIZE
}

/// The bytes of a new, empty drawing.
pub fn empty_drawing() -> Arc<[u8]> {
    sdw::write(&Drawing::new()).into()
}

fn count(objects: &[Object]) -> usize {
    objects
        .iter()
        .map(|object| match &object.shape {
            Shape::Group { objects, .. } => 1 + count(objects),
            _ => 1,
        })
        .sum()
}

/// The rectangle holding the objects at `indices` (top-level objects).
pub fn bounds(drawing: &Drawing, indices: &[usize]) -> Option<Rect> {
    indices
        .iter()
        .filter_map(|&index| drawing.objects.get(index))
        .map(|object| object.rect)
        .reduce(|a, b| a.union(&b))
}

fn centre(rect: Rect) -> (f64, f64) {
    (
        (f64::from(rect.left) + f64::from(rect.right)) / 2.0,
        (f64::from(rect.top) + f64::from(rect.bottom)) / 2.0,
    )
}

/// The indices in range, each once, in order.
fn valid(drawing: &Drawing, indices: &[usize]) -> Vec<usize> {
    let mut indices: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&index| index < drawing.objects.len())
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

/// Applies `map` to the objects at `indices`.
pub fn transform(drawing: &mut Drawing, indices: &[usize], map: &Affine) {
    for index in valid(drawing, indices) {
        drawing.objects[index].transform(map);
    }
}

/// Moves the objects by `(dx, dy)` twips.
pub fn move_by(drawing: &mut Drawing, indices: &[usize], dx: i32, dy: i32) {
    transform(
        drawing,
        indices,
        &Affine::translate(f64::from(dx), f64::from(dy)),
    );
}

/// Sizes the objects together from their joint rectangle onto `to` (left,
/// top, right, bottom; inverted to flip, as a handle dragged past the
/// opposite side flips in Ami Pro).
pub fn resize(drawing: &mut Drawing, indices: &[usize], to: (f64, f64, f64, f64)) {
    if let Some(from) = bounds(drawing, indices) {
        transform(drawing, indices, &Affine::rect_to_rect(from, to));
    }
}

/// Flip ▸ Horizontal or Vertical: each flippable object about the
/// selection's centre (Ami Pro flips no text or bitmap).
pub fn flip(drawing: &mut Drawing, indices: &[usize], horizontal: bool) {
    let Some(rect) = bounds(drawing, indices) else {
        return;
    };
    let (cx, cy) = centre(rect);
    let map = if horizontal {
        Affine {
            xx: -1.0,
            dx: 2.0 * cx,
            ..Affine::IDENTITY
        }
    } else {
        Affine {
            yy: -1.0,
            dy: 2.0 * cy,
            ..Affine::IDENTITY
        }
    };
    for index in valid(drawing, indices) {
        if drawing.objects[index].flippable() {
            drawing.objects[index].transform(&map);
        }
    }
}

/// Rotate: `tenths` of a degree clockwise about `centre`, or the
/// selection's centre (`ROTATEPROC`).
pub fn rotate(
    drawing: &mut Drawing,
    indices: &[usize],
    tenths: i32,
    centre_at: Option<(f64, f64)>,
) {
    let Some(rect) = bounds(drawing, indices) else {
        return;
    };
    let about = centre_at.unwrap_or_else(|| centre(rect));
    for index in valid(drawing, indices) {
        drawing.objects[index].rotate(tenths, about);
    }
}

/// Deletes the objects; the indices of what remains change.
pub fn delete(drawing: &mut Drawing, indices: &[usize]) {
    for index in valid(drawing, indices).into_iter().rev() {
        drawing.objects.remove(index);
    }
}

/// Adds objects in front of the others; returns their indices.
pub fn add(drawing: &mut Drawing, objects: Vec<Object>) -> Vec<usize> {
    let first = drawing.objects.len();
    drawing.objects.extend(objects);
    (first..drawing.objects.len()).collect()
}

/// Copies of the objects, moved by `(dx, dy)`, in front (Ctrl+D, and
/// Ami Pro's Shift at the end of a move); returns their indices.
pub fn duplicate(drawing: &mut Drawing, indices: &[usize], dx: i32, dy: i32) -> Vec<usize> {
    let mut copies: Vec<Object> = valid(drawing, indices)
        .into_iter()
        .map(|index| drawing.objects[index].clone())
        .collect();
    for copy in &mut copies {
        copy.transform(&Affine::translate(f64::from(dx), f64::from(dy)));
    }
    add(drawing, copies)
}

/// Where Bring to Front and the others move objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Order {
    Front,
    Back,
    /// One place nearer the front (DW5).
    Forward,
    /// One place nearer the back (DW5).
    Backward,
}

/// Restacks the objects, keeping their own order; returns their new
/// indices.
pub fn order(drawing: &mut Drawing, indices: &[usize], to: Order) -> Vec<usize> {
    let chosen = valid(drawing, indices);
    if chosen.is_empty() {
        return chosen;
    }
    let count = drawing.objects.len();
    match to {
        Order::Front | Order::Back => {
            let mut taken = Vec::new();
            let mut rest = Vec::new();
            for (index, object) in std::mem::take(&mut drawing.objects).into_iter().enumerate() {
                if chosen.contains(&index) {
                    taken.push(object);
                } else {
                    rest.push(object);
                }
            }
            let moved = taken.len();
            if to == Order::Front {
                rest.extend(taken);
                drawing.objects = rest;
                (count - moved..count).collect()
            } else {
                taken.extend(rest);
                drawing.objects = taken;
                (0..moved).collect()
            }
        }
        Order::Forward => {
            let mut at = chosen.clone();
            for slot in (0..at.len()).rev() {
                let index = at[slot];
                let limit = at.get(slot + 1).copied().unwrap_or(count);
                if index + 1 < limit {
                    drawing.objects.swap(index, index + 1);
                    at[slot] = index + 1;
                }
            }
            at
        }
        Order::Backward => {
            let mut at = chosen.clone();
            for slot in 0..at.len() {
                let index = at[slot];
                let limit = if slot == 0 { 0 } else { at[slot - 1] + 1 };
                if index > limit {
                    drawing.objects.swap(index, index - 1);
                    at[slot] = index - 1;
                }
            }
            at
        }
    }
}

/// Group: the objects, in their order, become one group where the front
/// one was; returns its index.
pub fn group(drawing: &mut Drawing, indices: &[usize]) -> Option<usize> {
    let chosen = valid(drawing, indices);
    if chosen.len() < 2 {
        return None;
    }
    let front = *chosen.last()?;
    let members: Vec<Object> = chosen
        .iter()
        .map(|&index| drawing.objects[index].clone())
        .collect();
    let at = front + 1 - chosen.len();
    for &index in chosen.iter().rev() {
        drawing.objects.remove(index);
    }
    drawing.objects.insert(at, Object::group(members));
    Some(at)
}

/// Ungroup: a group's members take its place; returns their indices.
pub fn ungroup(drawing: &mut Drawing, index: usize) -> Vec<usize> {
    let Some(Object {
        shape: Shape::Group { objects, .. },
        ..
    }) = drawing.objects.get(index).cloned()
    else {
        return Vec::new();
    };
    let count = objects.len();
    drawing.objects.splice(index..=index, objects);
    (index..index + count).collect()
}

/// How Align lines objects up (DW5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Align {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// Aligns the objects with the selection's edge or centre.
pub fn align(drawing: &mut Drawing, indices: &[usize], how: Align) {
    let Some(all) = bounds(drawing, indices) else {
        return;
    };
    for index in valid(drawing, indices) {
        let rect = drawing.objects[index].rect;
        let (dx, dy) = match how {
            Align::Left => (i32::from(all.left) - i32::from(rect.left), 0),
            Align::Right => (i32::from(all.right) - i32::from(rect.right), 0),
            Align::Centre => (
                (i32::from(all.left) + i32::from(all.right)
                    - i32::from(rect.left)
                    - i32::from(rect.right))
                    / 2,
                0,
            ),
            Align::Top => (0, i32::from(all.top) - i32::from(rect.top)),
            Align::Bottom => (0, i32::from(all.bottom) - i32::from(rect.bottom)),
            Align::Middle => (
                0,
                (i32::from(all.top) + i32::from(all.bottom)
                    - i32::from(rect.top)
                    - i32::from(rect.bottom))
                    / 2,
            ),
        };
        drawing.objects[index].transform(&Affine::translate(f64::from(dx), f64::from(dy)));
    }
}

/// Distributes three or more objects evenly across (or down) the
/// selection: equal gaps between them, the outer two staying.
pub fn distribute(drawing: &mut Drawing, indices: &[usize], across: bool) {
    let mut chosen = valid(drawing, indices);
    if chosen.len() < 3 {
        return;
    }
    let start = |rect: &Rect| {
        if across {
            i32::from(rect.left)
        } else {
            i32::from(rect.top)
        }
    };
    let extent = |rect: &Rect| {
        if across { rect.width() } else { rect.height() }
    };
    chosen.sort_by_key(|&index| start(&drawing.objects[index].rect));
    let first = drawing.objects[chosen[0]].rect;
    let last = drawing.objects[*chosen.last().unwrap_or(&0)].rect;
    let total: i32 = chosen
        .iter()
        .map(|&index| extent(&drawing.objects[index].rect))
        .sum();
    let span = start(&last) + extent(&last) - start(&first);
    let gap = f64::from(span - total) / (chosen.len() - 1) as f64;
    let mut at = f64::from(start(&first));
    for &index in &chosen {
        let rect = drawing.objects[index].rect;
        let shift = (at - f64::from(start(&rect))).round();
        let map = if across {
            Affine::translate(shift, 0.0)
        } else {
            Affine::translate(0.0, shift)
        };
        drawing.objects[index].transform(&map);
        at += f64::from(extent(&rect)) + gap;
    }
}

/// A change to the objects' look (Line Style, Fill Pattern, the text
/// objects' font; each field `None` keeps what each object has).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleChange {
    pub width: Option<u8>,
    pub style: Option<u8>,
    pub colour: Option<sdw::Colour>,
    pub start: Option<u8>,
    pub end: Option<u8>,
    pub pattern: Option<u16>,
    pub foreground: Option<sdw::Colour>,
    pub background: Option<sdw::Colour>,
    pub face: Option<String>,
    /// Twips, as a text's height (negative: the character height).
    pub height: Option<i16>,
    pub text_colour: Option<sdw::Colour>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}

impl StyleChange {
    /// Apply Line & Fill: the whole of `pen` and `fill`.
    pub fn line_and_fill(pen: &Pen, fill: &Fill) -> Self {
        Self {
            width: Some(pen.width),
            style: Some(pen.style),
            colour: Some(pen.colour),
            start: Some(pen.start),
            end: Some(pen.end),
            pattern: Some(fill.pattern),
            foreground: Some(fill.foreground),
            background: Some(fill.background),
            ..Self::default()
        }
    }
}

fn restyle(object: &mut Object, change: &StyleChange) {
    let endings = object.has_endings();
    if let Some(pen) = object.pen_mut() {
        if let Some(width) = change.width {
            pen.width = width;
        }
        if let Some(style) = change.style {
            pen.style = style;
        }
        if let Some(colour) = change.colour {
            pen.colour = colour;
        }
        if endings {
            if let Some(start) = change.start {
                pen.start = start;
            }
            if let Some(end) = change.end {
                pen.end = end;
            }
        }
    }
    if let Some(fill) = object.fill_mut() {
        if let Some(foreground) = change.foreground {
            fill.foreground = foreground;
            // A colour chosen with no fill gives a solid one (FILLPROC).
            if fill.pattern == Fill::NONE && change.pattern.is_none() {
                fill.pattern = Fill::SOLID;
            }
        }
        if let Some(pattern) = change.pattern {
            fill.pattern = pattern;
        }
        if let Some(background) = change.background {
            fill.background = background;
        }
    }
    match &mut object.shape {
        Shape::Text(text) => {
            let before = text.height;
            restyle_text(text, change);
            // A new size scales the box with the text (Ami Pro keeps the
            // box to the text's extent).
            if before != 0 && text.height != before {
                let ratio = f64::from(text.height) / f64::from(before);
                text.size = sdw::Point::new(
                    (f64::from(text.size.x) * ratio).round() as i16,
                    (f64::from(text.size.y) * ratio).round() as i16,
                );
            }
            object.refresh_rect();
        }
        Shape::Group { objects, .. } => {
            for member in objects.iter_mut() {
                restyle(member, change);
            }
            object.refresh_rect();
        }
        _ => {}
    }
}

fn restyle_text(text: &mut Text, change: &StyleChange) {
    if let Some(face) = &change.face {
        text.face = face.clone();
    }
    if let Some(height) = change.height {
        text.height = height;
    }
    if let Some(colour) = change.text_colour {
        text.colour = colour;
    }
    let mut set = |on: Option<bool>, bit: u8| {
        if let Some(on) = on {
            if on {
                text.attributes |= bit;
            } else {
                text.attributes &= !bit;
            }
        }
    };
    set(change.bold, Text::BOLD);
    set(change.italic, Text::ITALIC);
    set(change.underline, Text::UNDERLINE);
}

/// Applies a look to the objects.
pub fn set_style(drawing: &mut Drawing, indices: &[usize], change: &StyleChange) {
    for index in valid(drawing, indices) {
        restyle(&mut drawing.objects[index], change);
    }
}
