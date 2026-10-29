use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

pub(crate) const COURIER_PRIME_FAMILY: &str = "Courier Prime";
const REGULAR_KEY: &str = "scriptcraft-courier-prime-regular";
const BOLD_KEY: &str = "scriptcraft-courier-prime-bold";
const ITALIC_KEY: &str = "scriptcraft-courier-prime-italic";
const BOLD_ITALIC_KEY: &str = "scriptcraft-courier-prime-bold-italic";

pub(crate) fn font_definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::default();
    add_face(
        &mut definitions,
        COURIER_PRIME_FAMILY,
        REGULAR_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold",
        BOLD_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Italic",
        ITALIC_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
    );
    add_face(
        &mut definitions,
        "Courier Prime Bold Italic",
        BOLD_ITALIC_KEY,
        include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
    );
    definitions
}

fn add_face(
    definitions: &mut FontDefinitions,
    family_name: &str,
    data_key: &str,
    bytes: &'static [u8],
) {
    definitions
        .font_data
        .insert(data_key.to_owned(), FontData::from_static(bytes));
    definitions.families.insert(
        FontFamily::Name(family_name.into()),
        vec![data_key.to_owned()],
    );
}

pub(crate) fn install(context: &egui::Context) {
    context.set_fonts(font_definitions());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_faces_register_the_supplied_bytes_and_family_names() {
        let definitions = font_definitions();
        let faces = [
            (
                REGULAR_KEY,
                COURIER_PRIME_FAMILY,
                include_bytes!("../../../fonts/CourierPrime/Courier Prime.ttf").as_slice(),
            ),
            (
                BOLD_KEY,
                "Courier Prime Bold",
                include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold.ttf").as_slice(),
            ),
            (
                ITALIC_KEY,
                "Courier Prime Italic",
                include_bytes!("../../../fonts/CourierPrime/Courier Prime Italic.ttf").as_slice(),
            ),
            (
                BOLD_ITALIC_KEY,
                "Courier Prime Bold Italic",
                include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf")
                    .as_slice(),
            ),
        ];

        for (data_key, family_name, expected_bytes) in faces {
            let data = definitions
                .font_data
                .get(data_key)
                .unwrap_or_else(|| unreachable!());
            assert_eq!(data.font.as_ref(), expected_bytes);
            assert_eq!(
                definitions
                    .families
                    .get(&FontFamily::Name(family_name.into()))
                    .unwrap_or_else(|| unreachable!()),
                &[data_key.to_owned()]
            );
        }
    }
}
