use eframe::egui::{
    self,
    FontData, FontDefinitions, FontFamily,
};

pub(crate) const COURIER_PRIME_FAMILY: &str = "Courier Prime";
const REGULAR_KEY: &str = "scriptcraft-courier-prime-regular";
const BOLD_KEY: &str = "scriptcraft-courier-prime-bold";
const ITALIC_KEY: &str = "scriptcraft-courier-prime-italic";
const BOLD_ITALIC_KEY: &str = "scriptcraft-courier-prime-bold-italic";

pub(super) fn install(context: &egui::Context) {
    let mut definitions = FontDefinitions::default();
    let faces = courier_prime_faces();
    let families = [
        COURIER_PRIME_FAMILY,
        "Courier Prime Bold",
        "Courier Prime Italic",
        "Courier Prime Bold Italic",
    ];
    let keys = [REGULAR_KEY, BOLD_KEY, ITALIC_KEY, BOLD_ITALIC_KEY];
    for ((family, key), bytes) in families.into_iter().zip(keys).zip(faces) {
        definitions.font_data.insert(
            key.to_owned(),
            std::sync::Arc::new(FontData::from_static(bytes)),
        );
        definitions.families.insert(
            FontFamily::Name(family.into()),
            vec![key.to_owned()],
        );
    }
    context.set_fonts(definitions);
}

pub(super) fn courier_prime_faces() -> [&'static [u8]; 4] {
    [
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
        include_bytes!("../../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
    ]
}
