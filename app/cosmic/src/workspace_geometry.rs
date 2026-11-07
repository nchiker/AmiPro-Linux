use amipro_document::PageSetup;

pub const SCREEN_PIXELS_PER_POINT: f32 = 96.0 / 72.0;
const WORKSPACE_GUTTER: f32 = 24.0;
const MIN_DISPLAY_SCALE: f32 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageViewGeometry {
    pub display_scale: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerSegment {
    pub width: f32,
    pub inch: usize,
}

impl PageViewGeometry {
    pub fn fit_width(page: &PageSetup, available_width: f32) -> Self {
        let natural_width = page.width_points * SCREEN_PIXELS_PER_POINT;
        let usable_width = (available_width - WORKSPACE_GUTTER * 2.0).max(1.0);
        let display_scale = (usable_width / natural_width).clamp(MIN_DISPLAY_SCALE, 1.0);
        Self::at_scale(page, display_scale)
    }

    pub fn at_scale(page: &PageSetup, display_scale: f32) -> Self {
        let pixels_per_point = SCREEN_PIXELS_PER_POINT * display_scale;
        let [top, right, bottom, left] = page.margin_points;
        Self {
            display_scale,
            page_width: page.width_points * pixels_per_point,
            page_height: page.height_points * pixels_per_point,
            margin_top: top * pixels_per_point,
            margin_right: right * pixels_per_point,
            margin_bottom: bottom * pixels_per_point,
            margin_left: left * pixels_per_point,
        }
    }

    pub fn workspace_padding(self) -> f32 {
        WORKSPACE_GUTTER * self.display_scale.max(0.75)
    }

    pub fn workspace_width(self, available_width: f32) -> f32 {
        available_width.max(self.page_width + self.workspace_padding() * 2.0)
    }

    pub fn text_width(self) -> f32 {
        (self.page_width - self.margin_left - self.margin_right).max(0.0)
    }

    pub fn ruler_segments(self) -> Vec<RulerSegment> {
        let inch_width = 72.0 * SCREEN_PIXELS_PER_POINT * self.display_scale;
        let mut segments = Vec::new();
        let mut covered = 0.0;
        while covered < self.page_width {
            segments.push(RulerSegment {
                width: inch_width.min(self.page_width - covered),
                inch: segments.len(),
            });
            covered += inch_width;
        }
        segments
    }
}

#[cfg(test)]
mod tests {
    use amipro_document::Document;

    use super::*;

    #[test]
    fn default_letter_page_uses_physical_point_geometry() {
        let geometry = PageViewGeometry::at_scale(&Document::default().page_setup(), 1.0);

        assert_eq!(geometry.page_width, 816.0);
        assert_eq!(geometry.page_height, 1056.0);
        assert_eq!(geometry.margin_top, 96.0);
        assert_eq!(geometry.margin_right, 96.0);
        assert_eq!(geometry.margin_bottom, 96.0);
        assert_eq!(geometry.margin_left, 96.0);
        assert_eq!(geometry.text_width(), 624.0);
        assert_eq!(geometry.ruler_segments().len(), 9);
        assert!(
            (geometry
                .ruler_segments()
                .iter()
                .map(|segment| segment.width)
                .sum::<f32>()
                - geometry.page_width)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn page_shrinks_to_available_width_without_changing_document_geometry() {
        let document = Document::default();
        let geometry = PageViewGeometry::fit_width(&document.page_setup(), 700.0);

        assert!(geometry.display_scale < 1.0);
        assert!((geometry.page_width - (700.0 - WORKSPACE_GUTTER * 2.0)).abs() < 0.01);
        assert_eq!(document.page_setup().width_points, 612.0);
        assert_eq!(document.page_setup().margin_points, [72.0; 4]);
    }

    #[test]
    fn very_narrow_view_retains_a_usable_page_scale() {
        let geometry = PageViewGeometry::fit_width(&Document::default().page_setup(), 200.0);

        assert_eq!(geometry.display_scale, MIN_DISPLAY_SCALE);
        assert!(geometry.page_width > 200.0);
        assert!(geometry.workspace_width(200.0) > geometry.page_width);
    }
}
