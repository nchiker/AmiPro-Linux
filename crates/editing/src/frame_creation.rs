//! Ami Pro's rules for creating frames (`docs/research/FRAME_CREATION_NOTES.md`;
//! `docs/Design/UI_SPEC.md` §20): the default frame a new frame takes and
//! Make Default, the Create Frame dialog's values and limits, and the
//! rectangle a drag on the page draws. The frame itself is made by
//! `Command::CreateFrame`.

use amipro_document::{
    Border, Colour, FrameStyle, LayoutBox, LinePosition, Margins, Rect, Shadow, Sides, Twips, Wrap,
};

/// What every new frame takes: Ami Pro's default frame (`1018:0eeb`), which
/// Make Default replaces (`12b0:0465`). Placement, size, columns, and tabs
/// are never part of it: a new frame is Where placed, with one column and
/// no tabs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameDefaults {
    pub wrap: Wrap,
    pub opaque: bool,
    pub border: Border,
    pub shadow: Shadow,
    /// Rounded-corner percentage.
    pub corners: u8,
    pub background: Colour,
    pub margins: Margins,
}

const OPAQUE: i64 = 0x40;
const WRAP_AROUND: i64 = 0x80;
const LINES_ON: i64 = 0x1_0000;
const NO_WRAP_BESIDE: i64 = 0x2_0000;

impl FrameDefaults {
    /// Ami Pro's built-in default frame, when AMIPRO.INI has no `FrDefs`
    /// (`1018:0eeb`; every default fixture since F01, and FC1): opaque, Wrap
    /// around, a thin black line on all sides in the middle, a Normal shadow
    /// falling right and down, 18 % rounded corners, white, 0.10 in margins.
    pub const AMI_PRO: Self = Self {
        wrap: Wrap::Around,
        opaque: true,
        border: Border {
            sides: Sides::ALL,
            style: 1,
            colour: 0,
            position: LinePosition::Middle,
        },
        shadow: Shadow {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
            colour: 0,
        },
        corners: 18,
        background: 0x00FF_FFFF,
        margins: Margins::uniform(144),
    };

    /// What Make Default takes from a frame (`12b0:0465`): its wrap, display,
    /// lines, colours, shadow, corners, and interior margins; nothing of
    /// its placement, size, columns, or tabs (FC6, FC6N).
    pub fn from_style(style: &FrameStyle) -> Self {
        Self {
            wrap: style.wrap,
            opaque: style.opaque,
            border: style.border,
            shadow: style.shadow,
            corners: style.corners,
            background: style.background,
            margins: style.interior.margins,
        }
    }

    /// A new frame's appearance and interior over `rect` (`1260:0827`): the
    /// default's settings, one column, no tabs. Margins that do not fit are
    /// dropped as Ami Pro drops them: the left and right ones when left +
    /// bottom ≥ the width (Ami Pro adds the bottom margin, not the right),
    /// the top and bottom ones when their sum ≥ the frame's bottom edge
    /// from the paper's top (not its height), so in practice those stay
    /// (FC3: a 50 × 33-twip frame kept 144 above and below).
    pub fn frame_style(&self, rect: Rect) -> FrameStyle {
        let given = self.margins;
        let mut margins = given;
        if given.bottom + given.top >= rect.bottom {
            margins.top = 0;
            margins.bottom = 0;
        }
        if given.left + given.bottom >= rect.width() {
            margins.left = 0;
            margins.right = 0;
        }
        FrameStyle {
            wrap: self.wrap,
            opaque: self.opaque,
            border: self.border,
            shadow: self.shadow,
            corners: self.corners,
            background: self.background,
            interior: LayoutBox::single_column(margins),
            name: None,
            macro_name: None,
        }
    }

    /// A picture frame put into a table cell (`14d0:0000`, TF1): opaque or
    /// transparent and its wrap from the default, no lines, no shadow,
    /// square corners, no margins.
    pub fn cell_frame_style(&self) -> FrameStyle {
        FrameStyle {
            wrap: self.wrap,
            opaque: self.opaque,
            border: Border {
                sides: Sides::default(),
                ..self.border
            },
            shadow: Shadow::default(),
            corners: 0,
            background: self.background,
            interior: LayoutBox::single_column(Margins::uniform(0)),
            name: None,
            macro_name: None,
        }
    }

    /// The default as AMIPRO.INI's `FrDefs` holds it (`1018:0eeb`,
    /// `12b0:0465`), 17 numbers: line sides (1 all, else 2 left, 4 right, 8
    /// top, 16 bottom), line style, line position, unit (1, inches: the app
    /// shows inches throughout), line colour, background, flags (opaque
    /// `0x40`, Wrap around `0x80`, lines `0x10000`, No wrap beside
    /// `0x20000`), margins left, bottom, top, right, shadow extents left,
    /// top, right, bottom, shadow colour, and corners. The app keeps the
    /// default in its settings in this form.
    pub fn to_numbers(&self) -> [i64; 17] {
        let sides = self.border.sides;
        let side_code = if sides == Sides::ALL {
            1
        } else {
            [
                (sides.left, 2),
                (sides.right, 4),
                (sides.top, 8),
                (sides.bottom, 16),
            ]
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, bit)| bit)
            .sum()
        };
        let mut flags = 0;
        if self.opaque {
            flags |= OPAQUE;
        }
        match self.wrap {
            Wrap::Around => flags |= WRAP_AROUND,
            Wrap::NoWrapBeside => flags |= NO_WRAP_BESIDE,
            Wrap::NoWrapAround => {}
        }
        if !sides.is_none() {
            flags |= LINES_ON;
        }
        let margins = self.margins;
        let shadow = self.shadow;
        [
            side_code,
            i64::from(self.border.style),
            self.border.position as i64,
            1,
            i64::from(self.border.colour),
            i64::from(self.background),
            flags,
            i64::from(margins.left),
            i64::from(margins.bottom),
            i64::from(margins.top),
            i64::from(margins.right),
            i64::from(shadow.left),
            i64::from(shadow.top),
            i64::from(shadow.right),
            i64::from(shadow.bottom),
            i64::from(shadow.colour),
            i64::from(self.corners),
        ]
    }

    /// The default from `FrDefs` numbers, or `None` when they are not 17
    /// numbers in range. Of the flags only those Make Default keeps count
    /// (`12b0:0465`); without the lines flag there are no lines.
    pub fn from_numbers(numbers: &[i64]) -> Option<Self> {
        let [
            sides,
            style,
            position,
            _unit,
            line_colour,
            background,
            flags,
            left,
            bottom,
            top,
            right,
            shadow_left,
            shadow_top,
            shadow_right,
            shadow_bottom,
            shadow_colour,
            corners,
        ] = *numbers
        else {
            return None;
        };
        let twips = |value: i64| Twips::try_from(value).ok().filter(|value| *value >= 0);
        let colour = |value: i64| Colour::try_from(value).ok().filter(|c| *c <= 0x00FF_FFFF);
        let sides = if flags & LINES_ON == 0 || sides == 0 {
            Sides::NONE
        } else if sides == 1 {
            Sides::ALL
        } else {
            Sides {
                left: sides & 2 != 0,
                right: sides & 4 != 0,
                top: sides & 8 != 0,
                bottom: sides & 16 != 0,
            }
        };
        let position = match position {
            1 => LinePosition::Middle,
            2 => LinePosition::Inside,
            3 => LinePosition::Outside,
            4 => LinePosition::CloseToInside,
            5 => LinePosition::CloseToOutside,
            _ => return None,
        };
        let wrap = if flags & WRAP_AROUND != 0 {
            Wrap::Around
        } else if flags & NO_WRAP_BESIDE != 0 {
            Wrap::NoWrapBeside
        } else {
            Wrap::NoWrapAround
        };
        Some(Self {
            wrap,
            opaque: flags & OPAQUE != 0,
            border: Border {
                sides,
                style: u16::try_from(style).ok().filter(|style| *style >= 1)?,
                colour: colour(line_colour)?,
                position,
            },
            shadow: Shadow {
                left: twips(shadow_left)?,
                top: twips(shadow_top)?,
                right: twips(shadow_right)?,
                bottom: twips(shadow_bottom)?,
                colour: colour(shadow_colour)?,
            },
            corners: u8::try_from(corners)
                .ok()
                .filter(|corners| *corners <= 100)?,
            background: colour(background)?,
            margins: Margins {
                top: twips(top)?,
                right: twips(right)?,
                bottom: twips(bottom)?,
                left: twips(left)?,
            },
        })
    }
}

impl Default for FrameDefaults {
    fn default() -> Self {
        Self::AMI_PRO
    }
}

/// A frame's size and position as the Create Frame dialog shows them, from
/// the paper's top-left corner, and as AMIPRO.INI's `frmcrt` remembers the
/// last frame made: width, height, top, left.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameGeometry {
    pub width: Twips,
    pub height: Twips,
    pub top: Twips,
    pub left: Twips,
}

/// The smallest size the dialog remembers, each way (`1260:01cf`).
pub const REMEMBERED_MINIMUM: Twips = 360;

impl FrameGeometry {
    /// The dialog's values before any frame was made (`1128:0000`): 2.00 ×
    /// 2.00 in at 4.50 in down and 3.25 in in, centred on a letter page.
    pub const FIRST: Self = Self {
        width: 2880,
        height: 2880,
        top: 6480,
        left: 4680,
    };

    /// The geometry of `rect`.
    pub fn of(rect: Rect) -> Self {
        Self {
            width: rect.width(),
            height: rect.height(),
            top: rect.top,
            left: rect.left,
        }
    }

    /// The frame's rectangle.
    pub fn rect(self) -> Rect {
        Rect::new(
            self.left,
            self.top,
            self.left + self.width,
            self.top + self.height,
        )
    }

    /// What the dialog remembers after a frame was made over `rect`, drawn
    /// or not (`1260:01cf`): its position, and its size at least 0.25 in
    /// each way (FC3).
    pub fn remembered(rect: Rect) -> Self {
        let mut geometry = Self::of(rect);
        geometry.width = geometry.width.max(REMEMBERED_MINIMUM);
        geometry.height = geometry.height.max(REMEMBERED_MINIMUM);
        geometry
    }

    /// The dialog's starting values on a page `page_width` × `page_height`
    /// (`1128:0000`, `1360:01d7`): the remembered frame, or [`Self::FIRST`];
    /// a top below the page becomes the page height less 72, a left past
    /// it the page width less 144; then the width and height shrink so that
    /// the frame stays on the paper. (Ami Pro moves only a remembered
    /// position; the app also moves the first one, which only a page
    /// narrower or shorter than it can show.)
    pub fn starting(remembered: Option<Self>, page_width: Twips, page_height: Twips) -> Self {
        let mut geometry = remembered.unwrap_or(Self::FIRST);
        if geometry.top > page_height {
            geometry.top = page_height - 72;
        }
        if geometry.left > page_width {
            geometry.left = page_width - 144;
        }
        geometry.top = geometry.top.clamp(0, (page_height - 1).max(0));
        geometry.left = geometry.left.clamp(0, (page_width - 1).max(0));
        geometry.width = geometry.width.clamp(1, (page_width - geometry.left).max(1));
        geometry.height = geometry
            .height
            .clamp(1, (page_height - geometry.top).max(1));
        geometry
    }

    /// The range each value may take with the others as they are
    /// (`1360:0382`), so that the frame stays on the paper: width and height
    /// at least a twip, the positions at least 0.
    pub fn limits(self, page_width: Twips, page_height: Twips) -> GeometryLimits {
        GeometryLimits {
            width: (1, page_width - self.left),
            height: (1, page_height - self.top),
            top: (0, page_height - self.height),
            left: (0, page_width - self.width),
        }
    }
}

/// Each Create Frame value's smallest and largest allowed value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeometryLimits {
    pub width: (Twips, Twips),
    pub height: (Twips, Twips),
    pub top: (Twips, Twips),
    pub left: (Twips, Twips),
}

/// The frame a drag on a page draws (`1260:0000`, `1260:00ef`,
/// `1048:1fdb`): from the press to the release, in either direction, the
/// release kept on the paper. A release at the press point draws nothing
/// (FC4); there is no other minimum (FC3).
pub fn drawn_rect(
    press: (Twips, Twips),
    release: (Twips, Twips),
    page_width: Twips,
    page_height: Twips,
) -> Option<Rect> {
    let release = (
        release.0.clamp(0, page_width),
        release.1.clamp(0, page_height),
    );
    let rect = Rect::new(
        press.0.min(release.0),
        press.1.min(release.1),
        press.0.max(release.0),
        press.1.max(release.1),
    );
    rect.is_valid().then_some(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FC1's frame: Create Frame, 2.50 × 1.00 in at 3.00 in down and 1.25
    /// in in, with Ami Pro's default frame (record and `[frmlay]` in
    /// `FRAME_CREATION_NOTES.md`).
    #[test]
    fn the_built_in_default_frame_is_fc1s() {
        let geometry = FrameGeometry {
            width: 3600,
            height: 1440,
            top: 4320,
            left: 1800,
        };
        assert_eq!(geometry.rect(), Rect::new(1800, 4320, 5400, 5760));
        let style = FrameDefaults::AMI_PRO.frame_style(geometry.rect());
        assert_eq!(style.wrap, Wrap::Around);
        assert!(style.opaque);
        assert_eq!(style.border.sides, Sides::ALL);
        assert_eq!(style.border.style, 1);
        assert_eq!(style.border.position, LinePosition::Middle);
        assert_eq!(
            (
                style.shadow.left,
                style.shadow.top,
                style.shadow.right,
                style.shadow.bottom
            ),
            (0, 0, 100, 100)
        );
        assert_eq!(style.corners, 18);
        assert_eq!(style.background, 0x00FF_FFFF);
        assert_eq!(style.interior.margins, Margins::uniform(144));
        assert_eq!(style.interior.columns, 1);
        assert!(style.interior.tab_stops().is_empty());
        // FrDefs's flags and the text bit are FC1's 66240 (0x102C0).
        assert_eq!(FrameDefaults::AMI_PRO.to_numbers()[6] | 0x200, 66240);
    }

    /// FC3's 50 × 33-twip frame lost its left and right margins and kept
    /// its top and bottom ones; and the dialog then remembered 0.25 × 0.25.
    #[test]
    fn a_tiny_frame_drops_the_margins_ami_pro_drops_fc3() {
        let rect = Rect::new(8303, 8701, 8353, 8734);
        let margins = FrameDefaults::AMI_PRO.frame_style(rect).interior.margins;
        assert_eq!(
            margins,
            Margins {
                top: 144,
                right: 0,
                bottom: 144,
                left: 0
            }
        );
        assert_eq!(
            FrameGeometry::remembered(rect),
            FrameGeometry {
                width: 360,
                height: 360,
                top: 8701,
                left: 8303
            }
        );
        // Near the paper's top the vertical margins go too: their sum is
        // compared with the bottom edge.
        let top = FrameDefaults::AMI_PRO
            .frame_style(Rect::new(1000, 0, 2000, 200))
            .interior
            .margins;
        assert_eq!((top.top, top.bottom, top.left), (0, 0, 144));
    }

    /// FC6/FC6N: after Make Default on a With para above, No wrap around,
    /// Transparent, square frame with only a top line of style 3, no
    /// shadow, 0.20 in margins, and four columns, the next frame took all
    /// but the placement and the columns.
    #[test]
    fn make_default_keeps_what_ami_pro_keeps_fc6n() {
        let mut modified = FrameDefaults::AMI_PRO.frame_style(Rect::new(0, 0, 3600, 1440));
        modified.wrap = Wrap::NoWrapAround;
        modified.opaque = false;
        modified.corners = 0;
        modified.border.sides = Sides {
            top: true,
            ..Sides::NONE
        };
        modified.border.style = 3;
        modified.shadow = Shadow::default();
        modified.interior.margins = Margins::uniform(288);
        modified.interior.columns = 4;
        modified.interior.tabs = vec![Vec::new(); 4];
        let defaults = FrameDefaults::from_style(&modified);
        let new = defaults.frame_style(Rect::new(1440, 10800, 4320, 12960));
        assert_eq!(new.interior.columns, 1);
        assert_eq!(new.interior.margins, Margins::uniform(288));
        // FC6N's record: flags 66048 (0x10200), sides 8, style 3, no
        // shadow, square, margins 288.
        let numbers = defaults.to_numbers();
        assert_eq!(numbers[6] | 0x200, 66048);
        assert_eq!((numbers[0], numbers[1]), (8, 3));
        assert_eq!(&numbers[11..15], &[0, 0, 0, 0]);
        assert_eq!(numbers[16], 0);
        assert_eq!(&numbers[7..11], &[288; 4]);
    }

    #[test]
    fn frdefs_numbers_round_trip_and_refuse_nonsense() {
        let built_in = FrameDefaults::AMI_PRO.to_numbers();
        // 1018:0eeb's built-in values, in FrDefs order.
        assert_eq!(
            built_in,
            [
                1, 1, 1, 1, 0, 16_777_215, 0x100C0, 144, 144, 144, 144, 0, 0, 100, 100, 0, 18
            ]
        );
        assert_eq!(
            FrameDefaults::from_numbers(&built_in),
            Some(FrameDefaults::AMI_PRO)
        );
        let mut beside = FrameDefaults::AMI_PRO;
        beside.wrap = Wrap::NoWrapBeside;
        beside.border.sides = Sides {
            left: true,
            bottom: true,
            ..Sides::NONE
        };
        beside.border.position = LinePosition::CloseToOutside;
        assert_eq!(
            FrameDefaults::from_numbers(&beside.to_numbers()),
            Some(beside)
        );
        let mut none = FrameDefaults::AMI_PRO;
        none.border.sides = Sides::NONE;
        assert_eq!(FrameDefaults::from_numbers(&none.to_numbers()), Some(none));
        assert_eq!(FrameDefaults::from_numbers(&built_in[..16]), None);
        let mut bad = built_in;
        bad[16] = 101;
        assert_eq!(FrameDefaults::from_numbers(&bad), None);
        bad = built_in;
        bad[2] = 9;
        assert_eq!(FrameDefaults::from_numbers(&bad), None);
        bad = built_in;
        bad[8] = -1;
        assert_eq!(FrameDefaults::from_numbers(&bad), None);
    }

    /// The dialog's values: the first frame, a remembered one, and the
    /// clamps that keep them on a letter page (12240 × 15840).
    #[test]
    fn the_dialog_starts_from_the_last_frame_on_the_paper() {
        let (width, height) = (12240, 15840);
        assert_eq!(
            FrameGeometry::starting(None, width, height),
            FrameGeometry::FIRST
        );
        // FC2's drawn frame, as the dialog showed it next.
        let fc2 = FrameGeometry::remembered(Rect::new(4184, 5076, 6656, 7218));
        assert_eq!(FrameGeometry::starting(Some(fc2), width, height), fc2);
        // A remembered frame from a larger page.
        let big = FrameGeometry {
            width: 5000,
            height: 5000,
            top: 20000,
            left: 13000,
        };
        assert_eq!(
            FrameGeometry::starting(Some(big), width, height),
            FrameGeometry {
                width: 144,
                height: 72,
                top: 15768,
                left: 12096
            }
        );
        let limits = FrameGeometry::FIRST.limits(width, height);
        assert_eq!(limits.width, (1, 12240 - 4680));
        assert_eq!(limits.height, (1, 15840 - 6480));
        assert_eq!(limits.top, (0, 15840 - 2880));
        assert_eq!(limits.left, (0, 12240 - 2880));
        // FC5: a 9.00 in width at 1.25 in is past its limit (7.25 in).
        let fc5 = FrameGeometry {
            left: 1800,
            ..FrameGeometry::FIRST
        };
        assert!(12960 > fc5.limits(width, height).width.1);
    }

    /// FC2 was drawn from bottom right to top left; FC4's click drew
    /// nothing; a drag past the paper stops at its edge.
    #[test]
    fn a_drag_draws_from_corner_to_corner_on_the_paper() {
        assert_eq!(
            drawn_rect((6656, 7218), (4184, 5076), 12240, 15840),
            Some(Rect::new(4184, 5076, 6656, 7218))
        );
        assert_eq!(drawn_rect((700, 700), (700, 700), 12240, 15840), None);
        assert_eq!(
            drawn_rect((11000, 15000), (13000, 17000), 12240, 15840),
            Some(Rect::new(11000, 15000, 12240, 15840))
        );
        // A drag along one edge has no area.
        assert_eq!(drawn_rect((100, 100), (100, 900), 12240, 15840), None);
    }
}
