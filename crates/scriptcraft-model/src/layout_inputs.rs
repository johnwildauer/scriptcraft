pub(crate) const MAX_LAYOUT_NAME_BYTES: usize = 262_144;
const POINTS_PER_INCH: u32 = 72;

const fn points_from_eighths_of_an_inch(eighths: u32) -> u32 {
    eighths * (POINTS_PER_INCH / 8)
}

/// Session layout settings. Page geometry is projected to [`PageLayoutInputs`]
/// for canonical layout; `font_size` is retained only as the desktop zoom
/// baseline, while text styles and metrics come from the resolved environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutInputs {
    pub name: String,
    pub format_version: u32,
    pub paper_width: u32,
    pub paper_height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub boundary_gap: u32,
    pub font_size: u32,
}

impl Default for LayoutInputs {
    fn default() -> Self {
        Self {
            name: "us_letter_screenplay".into(),
            format_version: 1,
            paper_width: points_from_eighths_of_an_inch(68),
            paper_height: points_from_eighths_of_an_inch(88),
            margin_top: points_from_eighths_of_an_inch(8),
            margin_bottom: points_from_eighths_of_an_inch(8),
            margin_left: points_from_eighths_of_an_inch(12),
            margin_right: points_from_eighths_of_an_inch(8),
            boundary_gap: 12,
            font_size: 12,
        }
    }
}

/// Page geometry without font, character-cell, or nominal-line measurements.
/// Resolved font identities and metrics are a separate transient input and are
/// never stored on Project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageLayoutInputs {
    pub name: String,
    pub format_version: u32,
    pub paper_width: u32,
    pub paper_height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub boundary_gap: u32,
}

impl LayoutInputs {
    pub fn page_only(&self) -> PageLayoutInputs {
        PageLayoutInputs {
            name: self.name.clone(),
            format_version: self.format_version,
            paper_width: self.paper_width,
            paper_height: self.paper_height,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            boundary_gap: self.boundary_gap,
        }
    }
}

impl Default for PageLayoutInputs {
    fn default() -> Self {
        Self {
            name: "us_letter_screenplay".into(),
            format_version: 1,
            paper_width: points_from_eighths_of_an_inch(68),
            paper_height: points_from_eighths_of_an_inch(88),
            margin_top: points_from_eighths_of_an_inch(8),
            margin_bottom: points_from_eighths_of_an_inch(8),
            margin_left: points_from_eighths_of_an_inch(12),
            margin_right: points_from_eighths_of_an_inch(8),
            boundary_gap: 12,
        }
    }
}
