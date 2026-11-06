//! Main-thread-only AppKit SF Symbols images for selected native menu items.

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{NSSize, NSString};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use muda::{ContextMenu, Menu};

use super::menu_items::{MenuEntrySpec, MENU_CATALOG};

#[derive(Clone, Copy)]
enum SfSymbolItem {
    FileNew,
    FileOpen,
    FileClose,
    FileSave,
    FileSaveAs,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
}

impl SfSymbolItem {
    fn catalog_mapping(self) -> (&'static str, &'static str) {
        match self {
            Self::FileNew => ("file.new", "plus"),
            Self::FileOpen => ("file.open", "arrow.up.right"),
            Self::FileClose => ("file.close", "xmark"),
            Self::FileSave => ("file.save", "square.and.arrow.down"),
            Self::FileSaveAs => ("file.save-as", "square.and.arrow.down.on.square"),
            Self::EditUndo => ("edit.undo", "arrow.uturn.backward"),
            Self::EditRedo => ("edit.redo", "arrow.uturn.forward"),
            Self::EditCut => ("edit.cut", "scissors"),
            Self::EditCopy => ("edit.copy", "doc.on.doc"),
            Self::EditPaste => ("edit.paste", "doc.on.clipboard"),
        }
    }
}

const ITEM_SYMBOLS: &[SfSymbolItem] = &[
    SfSymbolItem::FileNew,
    SfSymbolItem::FileOpen,
    SfSymbolItem::FileClose,
    SfSymbolItem::FileSave,
    SfSymbolItem::FileSaveAs,
    SfSymbolItem::EditUndo,
    SfSymbolItem::EditRedo,
    SfSymbolItem::EditCut,
    SfSymbolItem::EditCopy,
    SfSymbolItem::EditPaste,
];

/// Retains each catalog item resolved from the installed muda hierarchy.
/// The `Rc` marker keeps these AppKit references confined to the UI thread.
pub(super) struct SfSymbolsAdapter {
    _root_menu: Option<Retained<NSMenu>>,
    _items: HashMap<&'static str, Retained<NSMenuItem>>,
    _main_thread_only: PhantomData<Rc<()>>,
}

impl SfSymbolsAdapter {
    /// Resolves requested menu items once after muda has installed the root menu,
    /// then applies only images for which both item and symbol are available.
    pub(super) fn install(root: &Menu) -> Self {
        let requests = requested_symbols();
        let Some(_main_thread) = MainThreadMarker::new() else {
            for (_, item_id, symbol_name, _) in &requests {
                warn_unavailable(item_id, symbol_name);
            }
            return Self::empty();
        };
        let Some(root_menu) = retain_native_menu(root) else {
            for (_, item_id, symbol_name, _) in &requests {
                warn_unavailable(item_id, symbol_name);
            }
            return Self::empty();
        };

        let mut menus = HashMap::new();
        for (menu_label, _, _, _) in &requests {
            menus.entry(*menu_label).or_insert_with(|| {
                find_unique_item_by_title(&root_menu, menu_label).and_then(|item| item.submenu())
            });
        }

        let mut items = HashMap::with_capacity(requests.len());
        let mut images = HashMap::with_capacity(requests.len());
        for (menu_label, item_id, symbol_name, label) in &requests {
            if let Some(item) = menus
                .get(menu_label)
                .and_then(Option::as_ref)
                .and_then(|menu| find_unique_item_by_title(menu, label))
            {
                items.insert(*item_id, item);
            }
            if let Some(image) = template_symbol_image(symbol_name) {
                images.insert(*item_id, image);
            }
        }

        for (_, item_id, symbol_name, _) in &requests {
            match (items.get(item_id), images.get(item_id)) {
                (Some(item), Some(image)) => item.setImage(Some(image)),
                _ => warn_unavailable(item_id, symbol_name),
            }
        }

        Self {
            _root_menu: Some(root_menu),
            _items: items,
            _main_thread_only: PhantomData,
        }
    }

    fn empty() -> Self {
        Self {
            _root_menu: None,
            _items: HashMap::new(),
            _main_thread_only: PhantomData,
        }
    }
}

fn requested_symbols() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    MENU_CATALOG
        .iter()
        .flat_map(|menu| {
            menu.entries.iter().filter_map(move |entry| {
                let MenuEntrySpec::Item(item) = entry else {
                    return None;
                };
                ITEM_SYMBOLS
                    .iter()
                    .find(|symbol_item| symbol_item.catalog_mapping().0 == item.id)
                    .map(|symbol_item| {
                        (
                            menu.label,
                            item.id,
                            symbol_item.catalog_mapping().1,
                            item.label,
                        )
                    })
            })
        })
        .collect()
}

#[allow(unsafe_code)]
fn retain_native_menu(muda_root: &Menu) -> Option<Retained<NSMenu>> {
    // SAFETY: `ContextMenu::ns_menu` guarantees a valid underlying NSMenu pointer
    // while `muda_root` is alive. The caller verified the AppKit main thread,
    // and `Retained::retain` creates the owned reference kept by this adapter.
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

fn template_symbol_image(symbol_name: &str) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(symbol_name);
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, None)?;
    image.setSize(NSSize::new(18.0, 18.0));
    image.setTemplate(true);
    Some(image)
}

fn warn_unavailable(item_id: &str, symbol_name: &str) {
    eprintln!(
        "ScriptCraft: optional native image unavailable (catalog={item_id}, symbol={symbol_name})"
    );
}
