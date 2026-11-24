//! Main-thread-only static Color and Highlight swatches for native menus.

use crate::chrome::menu_items::{MenuEntrySpec, MenuItemSpec, MenuSwatch, MENU_CATALOG};
use super::request_visible_menu_item_image;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{runtime::Bool, MainThreadMarker};
use objc2_app_kit::{NSBezierPath, NSColor, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::marker::PhantomData;
use std::rc::Rc;
use std::{collections::HashMap, eprintln};
use muda::{ContextMenu, Menu};

/// Retains the installed menu tree, each adorned item, and its image for the
/// controller lifetime. The marker confines AppKit handles to the main thread.
pub(super) struct MenuSwatchesAdapter {
    _root_menu: Option<Retained<NSMenu>>,
    _items: HashMap<&'static str, Retained<NSMenuItem>>,
    _images: HashMap<&'static str, Retained<NSImage>>,
    _main_thread_only: PhantomData<Rc<()>>,
}

impl MenuSwatchesAdapter {
    pub(super) fn install(muda_root: &Menu) -> Self {
        let requests = swatch_requests();
        let Some(_main_thread) = MainThreadMarker::new() else {
            warn_unavailable("format.color-highlight");
            return Self::empty();
        };
        let Some(root_menu) = retain_native_menu(muda_root) else {
            warn_unavailable("format.color-highlight");
            return Self::empty();
        };
        let Some(format_menu) = find_unique_item_by_title(&root_menu, "Format")
            .and_then(|item| item.submenu())
        else {
            warn_unavailable("format.color-highlight");
            return Self::empty();
        };

        let mut items = HashMap::with_capacity(requests.len());
        let mut images = HashMap::with_capacity(requests.len());
        let mut cached_images: Vec<(MenuSwatch, Option<Retained<NSImage>>)> = Vec::new();
        for (submenu_label, item) in &requests {
            let Some(swatch) = item.swatch else {
                continue;
            };
            let resolved = find_unique_item_by_title(&format_menu, submenu_label)
                .and_then(|menu_item| menu_item.submenu())
                .and_then(|submenu| find_unique_item_by_title(&submenu, item.label));
            let image = if let Some((_, image)) = cached_images.iter().find(|(cached, _)| *cached == swatch) {
                image.clone()
            } else {
                let image = make_swatch_image(swatch);
                cached_images.push((swatch, image.clone()));
                image
            };
            match (resolved, image) {
                (Some(native_item), Some(image)) => {
                    native_item.setImage(Some(&image));
                    request_visible_menu_item_image(&native_item);
                    items.insert(item.id, native_item);
                    images.insert(item.id, image);
                }
                _ => warn_unavailable(item.id),
            }
        }

        Self {
            _root_menu: Some(root_menu),
            _items: items,
            _images: images,
            _main_thread_only: PhantomData,
        }
    }

    fn empty() -> Self {
        Self {
            _root_menu: None,
            _items: HashMap::new(),
            _images: HashMap::new(),
            _main_thread_only: PhantomData,
        }
    }
}

fn swatch_requests() -> Vec<(&'static str, &'static MenuItemSpec)> {
    MENU_CATALOG
        .iter()
        .find(|menu| menu.label == "Format")
        .into_iter()
        .flat_map(|menu| menu.entries.iter())
        .filter_map(|entry| match entry {
            MenuEntrySpec::Submenu(submenu) if matches!(submenu.id, "format.color" | "format.highlight") => {
                Some((submenu.label, submenu.entries))
            }
            _ => None,
        })
        .flat_map(|(label, entries)| {
            entries.iter().filter_map(move |entry| match entry {
                MenuEntrySpec::Item(item) if item.swatch.is_some() => Some((label, item)),
                _ => None,
            })
        })
        .collect()
}

#[allow(unsafe_code)]
fn retain_native_menu(muda_root: &Menu) -> Option<Retained<NSMenu>> {
    // SAFETY: muda exposes the valid NSMenu while `muda_root` remains alive;
    // installation has verified the AppKit main thread and retains the handle.
    unsafe { Retained::retain(muda_root.ns_menu().cast::<NSMenu>()) }
}

fn find_unique_item_by_title(menu: &NSMenu, title: &str) -> Option<Retained<NSMenuItem>> {
    let expected_title = NSString::from_str(title);
    let mut found = None;
    for index in 0..menu.numberOfItems() {
        let item = menu.itemAtIndex(index)?;
        if item.title().isEqualToString(&expected_title) {
            if found.is_some() {
                return None;
            }
            found = Some(item);
        }
    }
    found
}

fn make_swatch_image(swatch: MenuSwatch) -> Option<Retained<NSImage>> {
    let size = NSSize::new(18.0, 18.0);
    let drawing_handler: RcBlock<dyn Fn(NSRect) -> Bool> = RcBlock::new(move |_rect: NSRect| {
        let circle = NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
            NSPoint::new(4.0, 4.0),
            NSSize::new(10.0, 10.0),
        ));
        match swatch {
            MenuSwatch::Palette([red, green, blue]) => {
                NSColor::colorWithSRGBRed_green_blue_alpha(
                    f64::from(red) / 255.0,
                    f64::from(green) / 255.0,
                    f64::from(blue) / 255.0,
                    1.0,
                )
                .setFill();
                circle.fill();
            }
            MenuSwatch::NoColor => {
                NSColor::secondaryLabelColor().setStroke();
                circle.setLineWidth(1.4);
                circle.stroke();
                let slash = NSBezierPath::bezierPath();
                slash.moveToPoint(NSPoint::new(4.5, 4.5));
                slash.lineToPoint(NSPoint::new(13.5, 13.5));
                slash.setLineWidth(1.4);
                slash.stroke();
            }
        }
        Bool::YES
    });
    let image = NSImage::imageWithSize_flipped_drawingHandler(size, false, &*drawing_handler);
    let actual_size = image.size();
    (actual_size.width == size.width && actual_size.height == size.height).then_some(image)
}

fn warn_unavailable(catalog_item: &str) {
    eprintln!(
        "ScriptCraft: optional native menu swatch unavailable (catalog={catalog_item})"
    );
}
