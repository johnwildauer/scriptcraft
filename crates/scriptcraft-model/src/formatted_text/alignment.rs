//! Alignment conversions shared by screenplay and title-page model values.

use crate::{formatting::ParagraphAlignment, title_page::TitlePageAlignment};

impl ParagraphAlignment {
    pub const fn fdx_value(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Center => "Center",
            Self::Justified => "Full",
        }
    }

    pub fn from_fdx(value: &str) -> Option<Self> {
        match value {
            "Left" => Some(Self::Left),
            "Right" => Some(Self::Right),
            "Center" => Some(Self::Center),
            "Full" => Some(Self::Justified),
            _ => None,
        }
    }
}

impl From<ParagraphAlignment> for TitlePageAlignment {
    fn from(value: ParagraphAlignment) -> Self {
        match value {
            ParagraphAlignment::Left => Self::Left,
            ParagraphAlignment::Right => Self::Right,
            ParagraphAlignment::Center => Self::Center,
            ParagraphAlignment::Justified => Self::Justified,
        }
    }
}

impl From<TitlePageAlignment> for ParagraphAlignment {
    fn from(value: TitlePageAlignment) -> Self {
        match value {
            TitlePageAlignment::Left => Self::Left,
            TitlePageAlignment::Right => Self::Right,
            TitlePageAlignment::Center => Self::Center,
            TitlePageAlignment::Justified => Self::Justified,
        }
    }
}
