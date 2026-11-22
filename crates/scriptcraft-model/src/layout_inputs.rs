pub(crate) const MAX_LAYOUT_NAME_BYTES: usize = 262_144;
const POINTS_PER_INCH: u32 = 72;

/// Supported document-level point adjustment applied to each candidate style
/// leading. `Unsupported` preserves an unrecognized present source value until
/// layout can return a bounded error; it has no numeric interpretation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DocumentLeading {
    #[default]
    Normal,
    Loose,
    Tight,
    VeryTight,
    Unsupported,
}

/// Effective document-owned FDX page layout. Header/footer margins are retained
/// as typed source values for future behavior but never affect body geometry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentPageLayout {
    pub paper_width: u32,
    pub paper_height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub document_leading: DocumentLeading,
    pub header_margin: Option<crate::formatting::FixedPoint>,
    pub footer_margin: Option<crate::formatting::FixedPoint>,
}

impl Default for DocumentPageLayout {
    fn default() -> Self {
        let inputs = LayoutInputs::default();
        Self {
            paper_width: inputs.paper_width,
            paper_height: inputs.paper_height,
            margin_top: inputs.margin_top,
            margin_bottom: inputs.margin_bottom,
            document_leading: inputs.document_leading,
            header_margin: Some(crate::formatting::FixedPoint::from_points(36)),
            footer_margin: Some(crate::formatting::FixedPoint::from_points(36)),
        }
    }
}

impl DocumentPageLayout {
    /// Produces the complete layout snapshot used by existing consumers,
    /// replacing only the five effective document page fields.
    pub fn session_layout_inputs(&self) -> LayoutInputs {
        let mut inputs = LayoutInputs::default();
        inputs.paper_width = self.paper_width;
        inputs.paper_height = self.paper_height;
        inputs.margin_top = self.margin_top;
        inputs.margin_bottom = self.margin_bottom;
        inputs.document_leading = self.document_leading;
        inputs
    }
}

const fn points_from_eighths_of_an_inch(eighths: u32) -> u32 {
    eighths * (POINTS_PER_INCH / 8)
}

/// Session layout settings. Page geometry is projected to [`PageLayoutInputs`]
/// for canonical layout; `font_size` is retained only as the desktop zoom
/// baseline, while text styles and metrics come from the resolved environment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    pub document_leading: DocumentLeading,
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
            document_leading: DocumentLeading::Normal,
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
    pub document_leading: DocumentLeading,
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
            document_leading: self.document_leading,
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
            document_leading: DocumentLeading::Normal,
        }
    }
}
