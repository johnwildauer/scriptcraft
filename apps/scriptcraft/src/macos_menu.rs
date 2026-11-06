//! Main-thread-only native macOS application-menu construction.
//!
//! Native menu construction, event wakeup/routing, and state synchronization.

use super::app_commands::AppCommandAvailability;
use super::menu_items::{
    LogicalShortcut, MenuEntrySpec, MenuItemBehavior, MenuItemSpec, PlaceholderKind, ShortcutKey,
    MENU_CATALOG,
};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use scriptcraft_commands::{CommandId, CommandOrigin};
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

type NativeEventInbox = Arc<Mutex<VecDeque<MenuId>>>;

/// Retains the complete installed native tree and each item handle for the
/// lifetime of `ScriptCraftApp`. The `Rc` marker makes main-thread confinement
/// explicit in addition to the non-Send muda handles.
pub(super) struct NativeMenuController {
    _root: Menu,
    _submenus: Vec<Submenu>,
    command_items: HashMap<MenuId, (CommandId, MenuItem)>,
    _placeholder_items: Vec<MenuItem>,
    _checked_placeholder_items: Vec<CheckMenuItem>,
    _predefined_items: Vec<PredefinedMenuItem>,
    _sf_symbols: super::macos_sf_symbols::SfSymbolsAdapter,
    event_inbox: NativeEventInbox,
    _main_thread_only: PhantomData<Rc<()>>,
}

impl NativeMenuController {
    /// Constructs and installs the root menu from the shared catalog. Called
    /// by eframe's app-creation callback, which runs on the macOS main thread.
    pub(super) fn install(
        availability: AppCommandAvailability,
        repaint_context: eframe::egui::Context,
    ) -> Result<Self, String> {
        let root = Menu::new();
        let mut retained_submenus = Vec::with_capacity(MENU_CATALOG.len() + 1);
        let mut command_items = HashMap::new();
        let mut placeholder_items = Vec::new();
        let mut checked_placeholder_items = Vec::new();
        let mut retained_predefined_items = Vec::new();

        let application_menu = Submenu::with_id("application", "ScriptCraft", true);
        append_predefined(
            &application_menu,
            PredefinedMenuItem::about(
                Some("About ScriptCraft"),
                Some(AboutMetadata {
                    name: Some("ScriptCraft".to_owned()),
                    version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                    ..AboutMetadata::default()
                }),
            ),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::separator(),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::hide(None),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::hide_others(None),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::show_all(None),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::separator(),
            &mut retained_predefined_items,
        )?;
        append_predefined(
            &application_menu,
            PredefinedMenuItem::quit(None),
            &mut retained_predefined_items,
        )?;
        root.append(&application_menu)
            .map_err(|error| format!("could not append native Application menu: {error}"))?;
        retained_submenus.push(application_menu);

        for menu_spec in MENU_CATALOG {
            let submenu = Submenu::with_id(menu_spec.id, menu_spec.label, true);
            for entry in menu_spec.entries {
                match entry {
                    MenuEntrySpec::Separator => append_predefined(
                        &submenu,
                        PredefinedMenuItem::separator(),
                        &mut retained_predefined_items,
                    )?,
                    MenuEntrySpec::Item(item) => append_catalog_item(
                        &submenu,
                        item,
                        availability,
                        &mut command_items,
                        &mut retained_submenus,
                        &mut placeholder_items,
                        &mut checked_placeholder_items,
                    )?,
                }
            }
            root.append(&submenu).map_err(|error| {
                format!("could not append native {} menu: {error}", menu_spec.label)
            })?;
            retained_submenus.push(submenu);
        }

        // Safe muda API; all construction and global installation are on the
        // app-creation/main thread.
        root.init_for_nsapp();
        let sf_symbols = super::macos_sf_symbols::SfSymbolsAdapter::install(&root);

        // In muda 0.17.2 an installed handler replaces delivery to
        // MenuEvent::receiver(). Forward IDs into an app-drained inbox and wake
        // egui here; the UI thread maps IDs to commands and queues dispatch.
        let event_inbox = Arc::new(Mutex::new(VecDeque::new()));
        let handler_inbox = Arc::clone(&event_inbox);
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            handler_inbox
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push_back(event.id);
            repaint_context.request_repaint();
        }));

        Ok(Self {
            _root: root,
            _submenus: retained_submenus,
            command_items,
            _placeholder_items: placeholder_items,
            _checked_placeholder_items: checked_placeholder_items,
            _predefined_items: retained_predefined_items,
            _sf_symbols: sf_symbols,
            event_inbox,
            _main_thread_only: PhantomData,
        })
    }

    /// Transfers native activation IDs into the shared application FIFO.
    pub(super) fn drain_events(&self, pending: &mut super::menu::PendingDesktopCommands) {
        let events = {
            let mut inbox = self
                .event_inbox
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::mem::take(&mut *inbox)
        };
        for id in events {
            if let Some((command, _)) = self.command_items.get(&id) {
                pending.enqueue_id(*command, CommandOrigin::NativeMenu);
            }
        }
    }

    /// Reprojects native labels and enabled state from the shared catalog and
    /// current application availability after each application update.
    pub(super) fn synchronize(&self, availability: AppCommandAvailability) {
        for menu in MENU_CATALOG {
            for entry in menu.entries {
                let MenuEntrySpec::Item(spec) = entry else {
                    continue;
                };
                let id = MenuId::new(spec.id);
                if let MenuItemBehavior::Command(command) = spec.behavior {
                    if let Some((_, item)) = self.command_items.get(&id) {
                        item.set_text(spec.label);
                        item.set_enabled(availability.is_enabled(command));
                    }
                }
            }
        }
    }
}

fn append_predefined(
    submenu: &Submenu,
    item: PredefinedMenuItem,
    retained_items: &mut Vec<PredefinedMenuItem>,
) -> Result<(), String> {
    submenu
        .append(&item)
        .map_err(|error| format!("could not append native predefined menu item: {error}"))?;
    retained_items.push(item);
    Ok(())
}

fn append_catalog_item(
    submenu: &Submenu,
    spec: &MenuItemSpec,
    availability: AppCommandAvailability,
    command_items: &mut HashMap<MenuId, (CommandId, MenuItem)>,
    retained_submenus: &mut Vec<Submenu>,
    placeholder_items: &mut Vec<MenuItem>,
    checked_placeholder_items: &mut Vec<CheckMenuItem>,
) -> Result<(), String> {
    let accelerator = spec.shortcut.map(native_accelerator);
    match spec.behavior {
        MenuItemBehavior::Command(command) => {
            let item = MenuItem::with_id(
                spec.id,
                spec.label,
                availability.is_enabled(command),
                accelerator,
            );
            submenu.append(&item).map_err(|error| {
                format!("could not append native menu item {}: {error}", spec.id)
            })?;
            command_items.insert(MenuId::new(spec.id), (command, item));
        }
        MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::Disclosure) => {
            let child_menu = Submenu::with_id(spec.id, spec.label, false);
            submenu.append(&child_menu).map_err(|error| {
                format!("could not append native disclosure placeholder {}: {error}", spec.id)
            })?;
            retained_submenus.push(child_menu);
        }
        MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::Checked) => {
            let item = CheckMenuItem::with_id(spec.id, spec.label, false, true, accelerator);
            submenu.append(&item).map_err(|error| {
                format!("could not append native checked placeholder {}: {error}", spec.id)
            })?;
            checked_placeholder_items.push(item);
        }
        MenuItemBehavior::DisabledPlaceholder(
            PlaceholderKind::Item | PlaceholderKind::Search,
        ) => {
            let item = MenuItem::with_id(spec.id, spec.label, false, accelerator);
            submenu.append(&item).map_err(|error| {
                format!("could not append native placeholder {}: {error}", spec.id)
            })?;
            placeholder_items.push(item);
        }
    }
    Ok(())
}

fn native_accelerator(shortcut: LogicalShortcut) -> Accelerator {
    let mut modifiers = if shortcut.primary {
        Modifiers::SUPER
    } else {
        Modifiers::empty()
    };
    if shortcut.shift {
        modifiers |= Modifiers::SHIFT;
    }
    if shortcut.control {
        modifiers |= Modifiers::CONTROL;
    }
    if shortcut.option {
        modifiers |= Modifiers::ALT;
    }
    Accelerator::new(Some(modifiers), native_key(shortcut.key))
}

fn native_key(key: ShortcutKey) -> Code {
    match key {
        ShortcutKey::A => Code::KeyA,
        ShortcutKey::B => Code::KeyB,
        ShortcutKey::C => Code::KeyC,
        ShortcutKey::D => Code::KeyD,
        ShortcutKey::E => Code::KeyE,
        ShortcutKey::F => Code::KeyF,
        ShortcutKey::G => Code::KeyG,
        ShortcutKey::H => Code::KeyH,
        ShortcutKey::I => Code::KeyI,
        ShortcutKey::J => Code::KeyJ,
        ShortcutKey::K => Code::KeyK,
        ShortcutKey::L => Code::KeyL,
        ShortcutKey::M => Code::KeyM,
        ShortcutKey::N => Code::KeyN,
        ShortcutKey::O => Code::KeyO,
        ShortcutKey::P => Code::KeyP,
        ShortcutKey::Q => Code::KeyQ,
        ShortcutKey::R => Code::KeyR,
        ShortcutKey::S => Code::KeyS,
        ShortcutKey::T => Code::KeyT,
        ShortcutKey::U => Code::KeyU,
        ShortcutKey::V => Code::KeyV,
        ShortcutKey::W => Code::KeyW,
        ShortcutKey::X => Code::KeyX,
        ShortcutKey::Y => Code::KeyY,
        ShortcutKey::Z => Code::KeyZ,
        ShortcutKey::Comma => Code::Comma,
        ShortcutKey::Semicolon => Code::Semicolon,
        ShortcutKey::Period => Code::Period,
        ShortcutKey::Enter => Code::Enter,
        ShortcutKey::Tab => Code::Tab,
        ShortcutKey::Backspace => Code::Backspace,
        ShortcutKey::Delete => Code::Delete,
        ShortcutKey::Digit0 => Code::Digit0,
        ShortcutKey::Digit1 => Code::Digit1,
        ShortcutKey::Digit2 => Code::Digit2,
        ShortcutKey::Digit3 => Code::Digit3,
        ShortcutKey::Digit4 => Code::Digit4,
        ShortcutKey::Digit5 => Code::Digit5,
        ShortcutKey::Digit6 => Code::Digit6,
        ShortcutKey::Digit7 => Code::Digit7,
        ShortcutKey::Digit8 => Code::Digit8,
        ShortcutKey::Digit9 => Code::Digit9,
        ShortcutKey::BracketLeft => Code::BracketLeft,
        ShortcutKey::BracketRight => Code::BracketRight,
        ShortcutKey::Backslash | ShortcutKey::Pipe => Code::Backslash,
        ShortcutKey::ArrowRight => Code::ArrowRight,
    }
}
