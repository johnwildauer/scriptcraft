//! Keep the packaged macOS bundle icon owned by Launch Services. On other
//! native targets, supply the embedded PNG as the runtime window icon.

#[cfg(not(target_os = "macos"))]
const APP_ICON_PNG: &[u8] = include_bytes!("../../../packaging/macos/AppIcon.png");

pub(super) fn egui_icon() -> Result<eframe::egui::IconData, String> {
    #[cfg(target_os = "macos")]
    {
        // eframe treats IconData::default() as an explicit request not to set
        // NSApplication's icon. An omitted icon instead selects eframe's logo.
        // Leaving the icon unset at the OS level preserves the AppIcon.icns
        // referenced by CFBundleIconFile and rendered by Launch Services.
        Ok(eframe::egui::IconData::default())
    }

    #[cfg(not(target_os = "macos"))]
    {
        embedded_source_icon()
    }
}

#[cfg(not(target_os = "macos"))]
fn embedded_source_icon() -> Result<eframe::egui::IconData, String> {
    let image = image::load_from_memory_with_format(APP_ICON_PNG, image::ImageFormat::Png)
        .map_err(|error| format!("could not decode the embedded ScriptCraft app icon: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();

    Ok(eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
