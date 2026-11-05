use eframe::egui::{
    self,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily, VariationCoords},
    FontData, FontFamily, RichText,
};

const MATERIAL_SYMBOLS_TTF: &[u8] = include_bytes!(
    "../../../fonts/MaterialSymbols/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf"
);

const STANDARD_WEIGHTS: [MaterialIconWeight; 7] = [
    MaterialIconWeight::W100,
    MaterialIconWeight::W200,
    MaterialIconWeight::W300,
    MaterialIconWeight::W400,
    MaterialIconWeight::W500,
    MaterialIconWeight::W600,
    MaterialIconWeight::W700,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum MaterialIconWeight {
    W100,
    W200,
    W300,
    W400,
    W500,
    W600,
    W700,
}

impl MaterialIconWeight {
    fn value(self) -> f32 {
        match self {
            Self::W100 => 100.0,
            Self::W200 => 200.0,
            Self::W300 => 300.0,
            Self::W400 => 400.0,
            Self::W500 => 500.0,
            Self::W600 => 600.0,
            Self::W700 => 700.0,
        }
    }

    fn family_value(self) -> u16 {
        match self {
            Self::W100 => 100,
            Self::W200 => 200,
            Self::W300 => 300,
            Self::W400 => 400,
            Self::W500 => 500,
            Self::W600 => 600,
            Self::W700 => 700,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum MaterialIconOpticalSize {
    Size20,
    Size24,
    Size48,
}

impl MaterialIconOpticalSize {
    fn value(self) -> f32 {
        match self {
            Self::Size20 => 20.0,
            Self::Size24 => 24.0,
            Self::Size48 => 48.0,
        }
    }

    fn family_value(self) -> u16 {
        match self {
            Self::Size20 => 20,
            Self::Size24 => 24,
            Self::Size48 => 48,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum MaterialIconGrade {
    Minus50,
    Zero,
    Plus200,
}

impl MaterialIconGrade {
    fn value(self) -> f32 {
        match self {
            Self::Minus50 => -50.0,
            Self::Zero => 0.0,
            Self::Plus200 => 200.0,
        }
    }

    fn family_value(self) -> i16 {
        match self {
            Self::Minus50 => -50,
            Self::Zero => 0,
            Self::Plus200 => 200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MaterialIconAxes {
    pub(crate) weight: MaterialIconWeight,
    pub(crate) optical_size: MaterialIconOpticalSize,
    pub(crate) grade: MaterialIconGrade,
}

impl MaterialIconAxes {
    pub(crate) const DEFAULT: Self = Self {
        weight: MaterialIconWeight::W200,
        optical_size: MaterialIconOpticalSize::Size24,
        grade: MaterialIconGrade::Zero,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaterialIconAxisOverrides {
    pub(crate) weight: Option<MaterialIconWeight>,
    pub(crate) optical_size: Option<MaterialIconOpticalSize>,
    pub(crate) grade: Option<MaterialIconGrade>,
}

impl MaterialIconAxisOverrides {
    pub(crate) const NONE: Self = Self {
        weight: None,
        optical_size: None,
        grade: None,
    };

    pub(crate) fn resolve(self, defaults: MaterialIconAxes) -> MaterialIconAxes {
        MaterialIconAxes {
            weight: self.weight.unwrap_or(defaults.weight),
            optical_size: self.optical_size.unwrap_or(defaults.optical_size),
            grade: self.grade.unwrap_or(defaults.grade),
        }
    }
}

/// Adds a bounded set of native faces without replacing other app fonts.
///
/// All standard weights are registered at the application default optical size
/// and grade. Additional optical-size/grade pairs are registered only when a
/// toolbar icon explicitly requests that complete axis combination.
pub(crate) fn install(context: &egui::Context, custom_axes: &[MaterialIconAxes]) {
    let mut registered_axes = Vec::with_capacity(STANDARD_WEIGHTS.len() + custom_axes.len());
    for weight in STANDARD_WEIGHTS {
        registered_axes.push(MaterialIconAxes {
            weight,
            ..MaterialIconAxes::DEFAULT
        });
    }
    for axes in custom_axes.iter().copied() {
        if !registered_axes.contains(&axes) {
            registered_axes.push(axes);
        }
    }

    for axes in registered_axes {
        add_face(
            context,
            egui_material_icons::IconStyle::Filled,
            axes,
            1.0,
        );
        add_face(
            context,
            egui_material_icons::IconStyle::Outlined,
            axes,
            0.0,
        );
    }
}

/// Builds icon text using the registered face matching the resolved axes.
pub(crate) fn rich_text(
    icon: egui_material_icons::MaterialIcon,
    overrides: Option<MaterialIconAxisOverrides>,
) -> RichText {
    let axes = overrides
        .unwrap_or(MaterialIconAxisOverrides::NONE)
        .resolve(MaterialIconAxes::DEFAULT);
    RichText::new(icon.codepoint).family(FontFamily::Name(family_name(icon.style, axes).into()))
}

fn add_face(
    context: &egui::Context,
    style: egui_material_icons::IconStyle,
    axes: MaterialIconAxes,
    fill: f32,
) {
    let family_name = family_name(style, axes);
    let data_key = format!("{family_name}-font-data");
    let mut data = FontData::from_static(MATERIAL_SYMBOLS_TTF);
    data.tweak.y_offset_factor = 0.05;
    data.tweak.coords = VariationCoords::new([
        (b"FILL", fill),
        (b"GRAD", axes.grade.value()),
        (b"opsz", axes.optical_size.value()),
        (b"wght", axes.weight.value()),
    ]);

    context.add_font(FontInsert::new(
        data_key.as_str(),
        data,
        vec![
            InsertFontFamily {
                family: FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: FontFamily::Name(family_name.into()),
                priority: FontPriority::Highest,
            },
        ],
    ));
}

fn family_name(style: egui_material_icons::IconStyle, axes: MaterialIconAxes) -> String {
    if axes == MaterialIconAxes::DEFAULT {
        return match style {
            egui_material_icons::IconStyle::Filled => {
                egui_material_icons::FONT_FAMILY.to_owned()
            }
            egui_material_icons::IconStyle::Outlined => {
                egui_material_icons::FONT_FAMILY_OUTLINED.to_owned()
            }
            _ => egui_material_icons::FONT_FAMILY.to_owned(),
        };
    }

    let (style, fill_name) = match style {
        egui_material_icons::IconStyle::Filled => ("filled", "fill-1"),
        egui_material_icons::IconStyle::Outlined => ("outlined", "fill-0"),
        _ => ("filled", "fill-1"),
    };
    format!(
        "scriptcraft-material-symbols-{style}-{fill_name}-wght-{}-opsz-{}-grad-{}",
        axes.weight.family_value(),
        axes.optical_size.family_value(),
        axes.grade.family_value()
    )
}
