pub(crate) const MAX_LAYOUT_NAME_BYTES: usize = 262_144;
const POINTS_PER_INCH: u32 = 72;

const fn points_from_eighths_of_an_inch(eighths: u32) -> u32 {
    eighths * (POINTS_PER_INCH / 8)
}

/// Global paper, margin, and fixed-cell inputs for the current layout path.
/// Element-specific geometry continues to come from paragraph settings rather
/// than a parallel role-indexed box table.
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
    pub chars_per_line: u32,
    pub lines_per_page: u32,
    pub line_advance: u32,
    pub boundary_gap: u32,
    pub font_family: String,
    pub font_size: u32,
    pub char_width: u32,
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
            chars_per_line: 55,
            lines_per_page: 55,
            line_advance: 12,
            boundary_gap: 12,
            font_family: "Courier Prime".into(),
            font_size: 12,
            char_width: 7,
        }
    }
}
