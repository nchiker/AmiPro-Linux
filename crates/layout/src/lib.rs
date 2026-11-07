//! Layout: line breaking, pagination, frame placement, and the display list
//! (`ARCHITECTURE.md`, "Layout engine"; `docs/research/DECISIONS.md`, D5).
//!
//! The crate has no dependencies beyond the document model. Shaping is behind
//! the [`Shaper`] trait: the app implements it with COSMIC Text, and tests use
//! the deterministic [`FixedShaper`]. Lengths are twips (1/1440 inch) as `f32`.

pub mod breaking;
pub mod bullets;
pub mod cells;
pub mod display;
pub mod engine;
pub mod estimate;
pub mod hyphenation;
pub mod incremental;
pub mod line_numbers;
pub mod master;
pub mod shaper;
pub mod tabs;

pub use breaking::{LineBreak, next_line};
pub use bullets::{
    BulletBox, BulletPiece, ParagraphNumber, bullet_box, bullet_metrics, end_counters,
    inline_bullet_width, number_text, paragraph_numbers, paragraph_numbers_carried,
};
pub use cells::{cell_box, cell_line_strokes};
pub use display::{DisplaySpan, display_spans, display_text};
pub use engine::*;
pub use estimate::{Page, ParagraphBox, Rect, paginate};
pub use hyphenation::hyphenation_points;
pub use incremental::{IncrementalLayout, LayoutKind, LayoutStats};
pub use line_numbers::{LineNumberBox, line_numbers};
pub use shaper::{FixedShaper, ShapeRequest, ShapedParagraph, ShapedWord, Shaper, TextExtent};
pub use tabs::{size_tabs, tab_stops};
