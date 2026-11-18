use crate::Role;

pub(crate) const LEGACY_ROLE_BOX_COUNT: usize = 7;
pub(crate) const ADDITIONAL_ROLE_BOX_COUNT: usize = 10;
const POINTS_PER_INCH: u32 = 72;

const fn points_from_eighths_of_an_inch(eighths: u32) -> u32 {
    eighths * (POINTS_PER_INCH / 8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Integer-point geometry retained at the transitional legacy-JSON boundary.
/// Shared FDX layout converts these values explicitly to signed hundredth-point
/// geometry; this type is not part of native FDX persistence.
pub struct RoleBox {
    pub x: u32,
    pub width: u32,
}

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
    /// Transitional integer role boxes retain their legacy JSON indices/values.
    pub role_boxes: [RoleBox; LEGACY_ROLE_BOX_COUNT],
    /// Legacy JSON-only custom boxes for roles outside the original seven.
    pub additional_role_boxes: [Option<RoleBox>; ADDITIONAL_ROLE_BOX_COUNT],
}

impl LayoutInputs {
    pub fn role_box(&self, role: Role) -> Option<&RoleBox> {
        if let Some(index) = role.legacy_profile_index() {
            self.role_boxes.get(index)
        } else {
            role.additional_profile_index()
                .and_then(|index| self.additional_role_boxes.get(index))
                .and_then(Option::as_ref)
        }
    }
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
            role_boxes: [
                RoleBox { x: 108, width: 432 },
                RoleBox { x: 108, width: 432 },
                RoleBox { x: 259, width: 175 },
                RoleBox { x: 180, width: 216 },
                RoleBox { x: 216, width: 180 },
                RoleBox { x: 432, width: 108 },
                RoleBox { x: 108, width: 432 },
            ],
            additional_role_boxes: [None; ADDITIONAL_ROLE_BOX_COUNT],
        }
    }
}
