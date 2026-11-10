//! Menu presentation policy and the shared ordered desktop-command handoff.

use super::app_commands::{
    command_for_id, AppCommandAvailability, CommandInvocation,
};
use super::menu_items::{
    LogicalShortcut, MenuEntrySpec, MenuItemBehavior, MenuItemSpec, PlaceholderKind, SemanticIcon,
    ShortcutKey, MENU_CATALOG,
};
use super::shortcuts::{shortcut_matches, ShortcutPlatform};
use eframe::egui;
use scriptcraft_commands::{CommandId, CommandOrigin};
use std::collections::VecDeque;

/// Initial presentation values are agent-defaults pending user review against
/// the supplied reference. They affect only the menu strip and its popups.
#[derive(Debug, Clone, Copy)]
struct MenuPresentationMetrics {
    bar_height: f32,
    bar_text_size: f32,
    item_text_size: f32,
    shortcut_text_size: f32,
    icon_size: f32,
    row_height: f32,
    icon_slot_width: f32,
    bar_item_gap_x: f32,
    bar_item_gap_y: f32,
    bar_button_padding_x: f32,
    bar_button_padding_y: f32,
    row_button_padding_x: f32,
    row_button_padding_y: f32,
    row_item_gap_x: f32,
    row_item_gap_y: f32,
    popup_padding_x: f32,
    popup_padding_y: f32,
    separator_spacing: f32,
    separator_stroke_width: f32,
    bar_corner_radius: u8,
    row_corner_radius: u8,
    popup_corner_radius: u8,
}

const MENU_PRESENTATION: MenuPresentationMetrics = MenuPresentationMetrics {
    bar_height: 24.0,
    bar_text_size: 12.0,
    item_text_size: 12.0,
    shortcut_text_size: 12.0,
    icon_size: 14.0,
    row_height: 22.0,
    icon_slot_width: 28.0,
    bar_item_gap_x: 8.0,
    bar_item_gap_y: 0.0,
    bar_button_padding_x: 8.0,
    bar_button_padding_y: 0.0,
    row_button_padding_x: 4.0,
    row_button_padding_y: 0.0,
    row_item_gap_x: 6.0,
    row_item_gap_y: 2.0,
    popup_padding_x: 10.0,
    popup_padding_y: 8.0,
    separator_spacing: 9.0,
    separator_stroke_width: 1.0,
    bar_corner_radius: 7,
    row_corner_radius: 4,
    popup_corner_radius: 8,
};

/// Fixed height of the in-window menu strip, in logical points.
pub(super) const MENU_PANEL_HEIGHT: f32 = MENU_PRESENTATION.bar_height;

/// Injectable startup choice for the runtime menu policy. The CLI adapter is a
/// later work order; production startup currently supplies `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuPlatformChoice {
    Auto,
    MacOS,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuHostPlatform {
    MacOS,
    Windows,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuBackend {
    NativeMacOS,
    InWindow,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAcceleratorOwner {
    NativeMacOS,
    InWindow,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeMenuPolicy {
    backend: MenuBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UnsupportedMenuPlatformChoice {
    pub(super) host: MenuHostPlatform,
    pub(super) requested: MenuPlatformChoice,
}

impl RuntimeMenuPolicy {
    /// Resolves a requested menu presentation against the actual compile target.
    /// A menu-platform choice never changes unrelated host integrations.
    pub(super) const fn resolve(
        host: MenuHostPlatform,
        requested: MenuPlatformChoice,
    ) -> Result<Self, UnsupportedMenuPlatformChoice> {
        let backend = match (host, requested) {
            (MenuHostPlatform::MacOS, MenuPlatformChoice::Auto | MenuPlatformChoice::MacOS) => {
                MenuBackend::NativeMacOS
            }
            (MenuHostPlatform::Windows, MenuPlatformChoice::Auto | MenuPlatformChoice::Windows) => {
                MenuBackend::InWindow
            }
            (MenuHostPlatform::Other, MenuPlatformChoice::Auto) => MenuBackend::None,
            (_, MenuPlatformChoice::Windows) => MenuBackend::InWindow,
            _ => return Err(UnsupportedMenuPlatformChoice { host, requested }),
        };
        Ok(Self { backend })
    }

    pub(super) const fn for_current_target() -> Self {
        let host = if cfg!(target_os = "macos") {
            MenuHostPlatform::MacOS
        } else if cfg!(windows) {
            MenuHostPlatform::Windows
        } else {
            MenuHostPlatform::Other
        };
        match Self::resolve(host, MenuPlatformChoice::Auto) {
            Ok(policy) => policy,
            Err(_) => Self { backend: MenuBackend::None },
        }
    }

    pub(super) const fn renders_in_window(self) -> bool {
        matches!(self.backend, MenuBackend::InWindow)
    }

    pub(super) const fn installs_native_macos(self) -> bool {
        matches!(self.backend, MenuBackend::NativeMacOS)
    }

    pub(super) const fn in_window_shortcuts_owned(self) -> bool {
        matches!(self.accelerator_owner(), MenuAcceleratorOwner::InWindow)
    }

    pub(super) const fn accelerator_owner(self) -> MenuAcceleratorOwner {
        match self.backend {
            MenuBackend::NativeMacOS => MenuAcceleratorOwner::NativeMacOS,
            MenuBackend::InWindow => MenuAcceleratorOwner::InWindow,
            MenuBackend::None => MenuAcceleratorOwner::None,
        }
    }
}

#[cfg(test)]
mod runtime_menu_policy_tests {
    use super::{
        MenuAcceleratorOwner, MenuHostPlatform, MenuPlatformChoice, RuntimeMenuPolicy,
    };

    #[test]
    fn automatic_policy_selects_only_the_host_default_backend() {
        let macos = RuntimeMenuPolicy::resolve(
            MenuHostPlatform::MacOS,
            MenuPlatformChoice::Auto,
        );
        let windows = RuntimeMenuPolicy::resolve(
            MenuHostPlatform::Windows,
            MenuPlatformChoice::Auto,
        );
        let other = RuntimeMenuPolicy::resolve(
            MenuHostPlatform::Other,
            MenuPlatformChoice::Auto,
        );

        assert!(macos.is_ok());
        assert!(windows.is_ok());
        assert!(other.is_ok());
        if let (Ok(macos), Ok(windows), Ok(other)) = (macos, windows, other) {
            assert!(macos.installs_native_macos());
            assert!(!macos.renders_in_window());
            assert!(!windows.installs_native_macos());
            assert!(windows.renders_in_window());
            assert!(!other.installs_native_macos());
            assert!(!other.renders_in_window());
        }
    }

    #[test]
    fn a_selected_backend_is_the_only_renderer_and_accelerator_owner() {
        for (host, requested, native, in_window, owner) in [
            (
                MenuHostPlatform::MacOS,
                MenuPlatformChoice::Auto,
                true,
                false,
                MenuAcceleratorOwner::NativeMacOS,
            ),
            (
                MenuHostPlatform::Windows,
                MenuPlatformChoice::Auto,
                false,
                true,
                MenuAcceleratorOwner::InWindow,
            ),
            (
                MenuHostPlatform::Windows,
                MenuPlatformChoice::Windows,
                false,
                true,
                MenuAcceleratorOwner::InWindow,
            ),
            (
                MenuHostPlatform::MacOS,
                MenuPlatformChoice::Windows,
                false,
                true,
                MenuAcceleratorOwner::InWindow,
            ),
            (
                MenuHostPlatform::Other,
                MenuPlatformChoice::Auto,
                false,
                false,
                MenuAcceleratorOwner::None,
            ),
            (
                MenuHostPlatform::Other,
                MenuPlatformChoice::Windows,
                false,
                true,
                MenuAcceleratorOwner::InWindow,
            ),
        ] {
            let policy = RuntimeMenuPolicy::resolve(host, requested);
            assert!(policy.is_ok());
            if let Ok(policy) = policy {
                assert_eq!(policy.renders_in_window(), in_window);
                assert_eq!(policy.installs_native_macos(), native);
                assert_eq!(policy.accelerator_owner(), owner);
                assert_eq!(
                    policy.in_window_shortcuts_owned(),
                    owner == MenuAcceleratorOwner::InWindow
                );
            }
        }
    }

    #[test]
    fn native_macos_choice_is_rejected_before_backend_construction_off_macos() {
        for host in [MenuHostPlatform::Windows, MenuHostPlatform::Other] {
            assert!(RuntimeMenuPolicy::resolve(host, MenuPlatformChoice::MacOS).is_err());
        }
        assert_eq!(
            RuntimeMenuPolicy::resolve(MenuHostPlatform::MacOS, MenuPlatformChoice::MacOS)
                .map(RuntimeMenuPolicy::installs_native_macos),
            Ok(true)
        );
    }
}

/// Resolves catalog shortcuts and the Windows-only Redo alias from the logical
/// key and modifiers carried by the pressed key event.
pub(super) fn command_for_key_event(
    key: egui::Key,
    modifiers: egui::Modifiers,
) -> Option<CommandId> {
    command_for_key_event_on_platform(key, modifiers, ShortcutPlatform::current())
}

fn command_for_key_event_on_platform(
    key: egui::Key,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> Option<CommandId> {
    for menu in MENU_CATALOG {
        for entry in menu.entries {
            let MenuEntrySpec::Item(item) = entry else {
                continue;
            };
            let MenuItemBehavior::Command(command) = item.behavior else {
                continue;
            };
            let is_dual_dialogue = command == CommandId::EditorDualDialogueToggle;
            if is_dual_dialogue && platform == ShortcutPlatform::Other {
                continue;
            }
            let advertised_match = !(is_dual_dialogue && platform == ShortcutPlatform::Windows)
                && item
                    .shortcut
                    .is_some_and(|shortcut| shortcut_matches(shortcut, key, modifiers, platform));
            let windows_alias_match = platform == ShortcutPlatform::Windows
                && item.windows_alias.is_some_and(|shortcut| {
                    shortcut_matches(shortcut, key, modifiers, ShortcutPlatform::Windows)
                });
            if advertised_match || windows_alias_match {
                return Some(command);
            }
        }
    }
    None
}

pub(super) fn command_for_pressed_key_event(
    event: &egui::Event,
    platform: ShortcutPlatform,
) -> Option<CommandId> {
    match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => command_for_key_event_on_platform(*key, *modifiers, platform),
        _ => None,
    }
}

/// One FIFO owned by the application receives deferred typed invocations from
/// egui menus, native menus, and owned menu accelerators. The dispatcher
/// rechecks availability after detaching each activation-ordered batch.
#[derive(Debug, Default)]
pub(super) struct PendingDesktopCommands {
    pending: VecDeque<CommandInvocation>,
}

impl PendingDesktopCommands {
    pub(super) fn enqueue(&mut self, invocation: CommandInvocation) {
        self.pending.push_back(invocation);
    }

    pub(super) fn enqueue_id(&mut self, id: CommandId, origin: CommandOrigin) {
        if let Some(command) = command_for_id(id) {
            self.enqueue(CommandInvocation::new(command, origin));
        }
    }

    /// Detaches queued commands in activation order for the application to
    /// dispatch; commands enqueued afterward remain for the next handoff.
    pub(super) fn take_pending(
        &mut self,
    ) -> std::collections::vec_deque::IntoIter<CommandInvocation> {
        std::mem::take(&mut self.pending).into_iter()
    }
}

/// Renders the catalog through egui's menu-button popup behavior. Activation is
/// only enqueued here; application commands are drained at the update boundary.
pub(super) fn show_menu_bar(
    ui: &mut egui::Ui,
    availability: AppCommandAvailability,
    pending: &mut PendingDesktopCommands,
) {
    let metrics = MENU_PRESENTATION;
    let viewport_height = ui.ctx().content_rect().height();
    let popup_padding_y = metrics.popup_padding_y.min((viewport_height / 2.0).max(0.0));
    let popup_max_height = (viewport_height - 2.0 * popup_padding_y - 2.0).max(0.0);
    let popup_style = move |style: &mut egui::Style| {
        let separator_color = style.visuals.widgets.noninteractive.bg_stroke.color;
        egui::containers::menu::menu_style(style);
        style.spacing.menu_margin = egui::Margin::symmetric(
            metrics.popup_padding_x as i8,
            popup_padding_y as i8,
        );
        style.spacing.button_padding =
            egui::vec2(metrics.row_button_padding_x, metrics.row_button_padding_y);
        style.spacing.item_spacing = egui::vec2(metrics.row_item_gap_x, metrics.row_item_gap_y);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(metrics.popup_corner_radius);
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
            metrics.separator_stroke_width,
            separator_color,
        );
        style.visuals.widgets.inactive.corner_radius =
            egui::CornerRadius::same(metrics.row_corner_radius);
        style.visuals.widgets.hovered.corner_radius =
            egui::CornerRadius::same(metrics.row_corner_radius);
        style.visuals.widgets.active.corner_radius =
            egui::CornerRadius::same(metrics.row_corner_radius);
        style.visuals.widgets.open.corner_radius =
            egui::CornerRadius::same(metrics.row_corner_radius);
    };
    let active_top_level_popup_key = egui::Id::new("scriptcraft-active-top-level-menu-popup");
    let mut active_top_level_popup_id = ui
        .ctx()
        .data(|data| data.get_temp::<egui::Id>(active_top_level_popup_key))
        .filter(|popup_id| egui::Popup::is_id_open(ui.ctx(), *popup_id));
    egui::MenuBar::new()
        .style(move |style: &mut egui::Style| {
            egui::containers::menu::menu_style(style);
            style.spacing.interact_size.y = metrics.bar_height;
            style.spacing.item_spacing =
                egui::vec2(metrics.bar_item_gap_x, metrics.bar_item_gap_y);
            style.spacing.button_padding =
                egui::vec2(metrics.bar_button_padding_x, metrics.bar_button_padding_y);
            style.visuals.widgets.inactive.corner_radius =
                egui::CornerRadius::same(metrics.bar_corner_radius);
            style.visuals.widgets.hovered.corner_radius =
                egui::CornerRadius::same(metrics.bar_corner_radius);
            style.visuals.widgets.active.corner_radius =
                egui::CornerRadius::same(metrics.bar_corner_radius);
            style.visuals.widgets.open.corner_radius =
                egui::CornerRadius::same(metrics.bar_corner_radius);
        })
        .config(egui::containers::menu::MenuConfig::new().style(popup_style))
        .ui(ui, |ui| {
        for menu in MENU_CATALOG {
            let menu_button = ui.menu_button(
                egui::RichText::new(menu.label).size(metrics.bar_text_size),
                |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(popup_max_height)
                        .min_scrolled_height(0.0)
                        .auto_shrink([true, true])
                        .show(ui, |ui| {
                            for entry in menu.entries {
                                match entry {
                                    MenuEntrySpec::Separator => {
                                        ui.add(
                                            egui::Separator::default()
                                                .spacing(metrics.separator_spacing)
                                                .horizontal(),
                                        );
                                    }
                                    MenuEntrySpec::Item(item) => {
                                        let enabled = match item.behavior {
                                            MenuItemBehavior::Command(command) => {
                                                command_enabled(command, availability)
                                            }
                                            MenuItemBehavior::DisabledPlaceholder(_) => false,
                                        };
                                        let shortcut = menu_shortcut_label(item);
                                        let icon_atom = menu_icon_atom(
                                            item,
                                            matches!(
                                                item.behavior,
                                                MenuItemBehavior::DisabledPlaceholder(
                                                    PlaceholderKind::Checked
                                                )
                                            ),
                                            metrics,
                                        );
                                        let mut label_atom = egui::Atom::from(
                                            egui::RichText::new(item.label)
                                                .size(metrics.item_text_size),
                                        );
                                        label_atom.grow = true;
                                        label_atom.align = egui::Align2::LEFT_CENTER;
                                        let button = egui::Button::new((icon_atom, label_atom))
                                        .shortcut_text(
                                            egui::RichText::new(shortcut)
                                                .size(metrics.shortcut_text_size),
                                        )
                                        .min_size(egui::vec2(0.0, metrics.row_height))
                                        .corner_radius(egui::CornerRadius::same(
                                            metrics.row_corner_radius,
                                        ));
                                        let response = ui.add_enabled(enabled, button);
                                        response.widget_info(|| {
                                            egui::WidgetInfo::labeled(
                                                egui::WidgetType::Button,
                                                enabled,
                                                item.accessibility_name.to_owned(),
                                            )
                                        });
                                        if response.has_focus() {
                                            ui.painter().rect_stroke(
                                                response.rect.shrink(1.0),
                                                egui::CornerRadius::same(
                                                    metrics.row_corner_radius,
                                                ),
                                                ui.visuals().selection.stroke,
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        if response.clicked() {
                                            if let MenuItemBehavior::Command(command) = item.behavior {
                                                pending.enqueue_id(command, CommandOrigin::Menu);
                                                ui.close();
                                                ui.ctx().request_repaint();
                                            }
                                        }
                                    }
                                }
                            }
                        });
                },
            );
            menu_button.response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    menu.accessibility_name.to_owned(),
                )
            });
            let popup_id = egui::Popup::default_response_id(&menu_button.response);
            if menu_button.response.hovered()
                && active_top_level_popup_id.is_some_and(|active_popup_id| {
                    active_popup_id != popup_id
                        && egui::Popup::is_id_open(ui.ctx(), active_popup_id)
                })
            {
                // MenuBar/MenuButton only open on click in egui 0.34.3. Switch
                // only when another catalog popup is already open; Popup
                // memory closes that popup and preserves normal menu handling.
                egui::Popup::open_id(ui.ctx(), popup_id);
                active_top_level_popup_id = Some(popup_id);
                ui.ctx().request_repaint();
            }
            if egui::Popup::is_id_open(ui.ctx(), popup_id) {
                active_top_level_popup_id = Some(popup_id);
            }
        }
        if let Some(popup_id) = active_top_level_popup_id {
            ui.ctx()
                .data_mut(|data| data.insert_temp(active_top_level_popup_key, popup_id));
        }
    });
}

fn menu_icon_atom(
    item: &MenuItemSpec,
    checked: bool,
    metrics: MenuPresentationMetrics,
) -> egui::Atom<'static> {
    let slot_size = egui::vec2(metrics.icon_slot_width, metrics.row_height);
    if checked {
        let mut atom = egui::Atom::from(
            egui::RichText::new("✓").size(metrics.icon_size),
        );
        atom.size = Some(slot_size);
        return atom;
    }
    match item.icon.map(material_icon) {
        Some(icon) => {
            let mut atom = egui::Atom::from(
                crate::material_font::rich_text(icon, None).size(metrics.icon_size),
            );
            atom.size = Some(slot_size);
            atom
        }
        None => egui::Atom::custom(egui::Id::new(("menu-icon-slot", item.id)), slot_size),
    }
}

fn material_icon(icon: SemanticIcon) -> egui_material_icons::MaterialIcon {
    use egui_material_icons::icons;

    match icon {
        SemanticIcon::DocumentNew => icons::ICON_ADD,
        SemanticIcon::FolderOpen => icons::ICON_FOLDER_OPEN,
        SemanticIcon::WindowClose => icons::ICON_CLOSE,
        SemanticIcon::Save => icons::ICON_SAVE,
        SemanticIcon::SaveAs => icons::ICON_SAVE_AS,
        SemanticIcon::PdfExport => icons::ICON_PICTURE_AS_PDF,
        SemanticIcon::Undo => icons::ICON_UNDO,
        SemanticIcon::Redo => icons::ICON_REDO,
        SemanticIcon::Cut => icons::ICON_CONTENT_CUT,
        SemanticIcon::Copy => icons::ICON_CONTENT_COPY,
        SemanticIcon::Paste => icons::ICON_CONTENT_PASTE,
        SemanticIcon::Delete => icons::ICON_DELETE,
        SemanticIcon::SelectAll => icons::ICON_SELECT_ALL,
    }
}

fn command_enabled(command: CommandId, availability: AppCommandAvailability) -> bool {
    availability.is_enabled(command)
}

fn menu_shortcut_label(item: &MenuItemSpec) -> String {
    if item.behavior == MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle) {
        return match ShortcutPlatform::current() {
            ShortcutPlatform::MacOS => "Command+D".to_owned(),
            ShortcutPlatform::Windows => "Control+Alt+D".to_owned(),
            ShortcutPlatform::Other => String::new(),
        };
    }
    item.shortcut.map(shortcut_label).unwrap_or_else(|| {
        if matches!(
            item.behavior,
            MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::Disclosure)
        ) {
            "›".to_owned()
        } else {
            String::new()
        }
    })
}

/// The in-window catalog preserves its Windows-style shortcut annotations;
/// actual shortcut matching continues to resolve against the host platform.
fn shortcut_label(shortcut: LogicalShortcut) -> String {
    let key = match shortcut.key {
        ShortcutKey::A => "A",
        ShortcutKey::B => "B",
        ShortcutKey::C => "C",
        ShortcutKey::D => "D",
        ShortcutKey::E => "E",
        ShortcutKey::F => "F",
        ShortcutKey::G => "G",
        ShortcutKey::H => "H",
        ShortcutKey::I => "I",
        ShortcutKey::J => "J",
        ShortcutKey::K => "K",
        ShortcutKey::L => "L",
        ShortcutKey::M => "M",
        ShortcutKey::N => "N",
        ShortcutKey::O => "O",
        ShortcutKey::P => "P",
        ShortcutKey::Q => "Q",
        ShortcutKey::R => "R",
        ShortcutKey::S => "S",
        ShortcutKey::T => "T",
        ShortcutKey::U => "U",
        ShortcutKey::V => "V",
        ShortcutKey::W => "W",
        ShortcutKey::X => "X",
        ShortcutKey::Y => "Y",
        ShortcutKey::Z => "Z",
        ShortcutKey::Comma => ",",
        ShortcutKey::Semicolon => ";",
        ShortcutKey::Period => ".",
        ShortcutKey::Enter => "Enter",
        ShortcutKey::Tab => "Tab",
        ShortcutKey::Backspace => "Backspace",
        ShortcutKey::Delete => "Delete",
        ShortcutKey::Digit0 => "0",
        ShortcutKey::Digit1 => "1",
        ShortcutKey::Digit2 => "2",
        ShortcutKey::Digit3 => "3",
        ShortcutKey::Digit4 => "4",
        ShortcutKey::Digit5 => "5",
        ShortcutKey::Digit6 => "6",
        ShortcutKey::Digit7 => "7",
        ShortcutKey::Digit8 => "8",
        ShortcutKey::Digit9 => "9",
        ShortcutKey::Tab => "Tab",
        ShortcutKey::Backspace => "Backspace",
        ShortcutKey::Delete => "Delete",
        ShortcutKey::Digit0 => "0",
        ShortcutKey::Digit1 => "1",
        ShortcutKey::Digit2 => "2",
        ShortcutKey::Digit3 => "3",
        ShortcutKey::Digit4 => "4",
        ShortcutKey::Digit5 => "5",
        ShortcutKey::Digit6 => "6",
        ShortcutKey::Digit7 => "7",
        ShortcutKey::Digit8 => "8",
        ShortcutKey::Digit9 => "9",
        ShortcutKey::BracketLeft => "[",
        ShortcutKey::BracketRight => "]",
        ShortcutKey::Backslash => "\\",
        ShortcutKey::Pipe => "|",
        ShortcutKey::ArrowRight => "→",
    };
    let mut parts = Vec::with_capacity(4);
    if shortcut.control {
        parts.push("Control");
    }
    if shortcut.primary {
        parts.push("Ctrl");
    }
    if shortcut.option {
        parts.push("Alt");
    }
    if shortcut.shift {
        parts.push("Shift");
    }
    parts.push(key);
    parts.join("+")
}

impl super::ScriptCraftApp {
    /// Dispatches the detached queue batch in order. Each shared command
    /// performs its normal current-availability check before it can mutate.
    pub(super) fn dispatch_pending_commands(&mut self, ctx: &eframe::egui::Context) {
        for invocation in self.pending_commands.take_pending() {
            self.dispatch_command(invocation, ctx);
        }
    }
}

#[cfg(test)]
mod dual_dialogue_menu_tests {
    use super::*;

    fn dual_dialogue_item() -> MenuItemSpec {
        MENU_CATALOG
            .iter()
            .flat_map(|menu| menu.entries)
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item)
                    if item.behavior
                        == MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle) =>
                {
                    Some(*item)
                }
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("dual-dialogue menu command is catalogued"))
    }

    fn modifiers(
        platform: ShortcutPlatform,
        primary: bool,
        option: bool,
        extra_control: bool,
        shift: bool,
    ) -> egui::Modifiers {
        egui::Modifiers {
            alt: option,
            ctrl: (platform == ShortcutPlatform::MacOS && extra_control)
                || (platform == ShortcutPlatform::Windows && primary),
            command: primary,
            mac_cmd: platform == ShortcutPlatform::Windows && extra_control,
            shift,
        }
    }

    #[test]
    fn dual_dialogue_remains_the_single_format_menu_command() {
        let entries = MENU_CATALOG
            .iter()
            .flat_map(|menu| menu.entries.iter())
            .filter_map(|entry| match entry {
                MenuEntrySpec::Item(item) => Some(item),
                MenuEntrySpec::Separator => None,
            })
            .collect::<Vec<_>>();
        let active = entries
            .iter()
            .filter(|item| {
                item.behavior == MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle)
            })
            .collect::<Vec<_>>();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "format.dual-dialogue");

        let edit = entries
            .iter()
            .find(|item| item.id == "format.edit-dual-dialogue")
            .copied();
        assert!(edit.is_some_and(|item| {
            matches!(item.behavior, MenuItemBehavior::DisabledPlaceholder(_))
                && item.shortcut.is_some_and(|shortcut| shortcut.shift)
        }));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_print_uses_unshifted_primary_p_and_print_preview_stays_disabled() {
        let print = MENU_CATALOG
            .iter()
            .flat_map(|menu| menu.entries)
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item) if item.id == "file.print" => Some(*item),
                _ => None,
            });
        assert!(print.is_some_and(|item| {
            item.label == "Print..."
                && item.behavior == MenuItemBehavior::Command(CommandId::FilePrint)
                && item.shortcut.is_some_and(|shortcut| {
                    shortcut.primary
                        && shortcut.key == ShortcutKey::P
                        && !shortcut.shift
                        && !shortcut.control
                        && !shortcut.option
                })
        }));

        let preview = MENU_CATALOG
            .iter()
            .flat_map(|menu| menu.entries)
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item) if item.id == "file.print-preview" => Some(*item),
                _ => None,
            });
        assert!(preview.is_some_and(|item| matches!(
            item.behavior,
            MenuItemBehavior::DisabledPlaceholder(_)
        )));
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::MacOS, true, false, false, false),
                ShortcutPlatform::MacOS,
            ),
            Some(CommandId::FilePrint),
        );
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::MacOS, true, false, false, true),
                ShortcutPlatform::MacOS,
            ),
            Some(CommandId::FileSaveAsPdf),
        );
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::MacOS, false, false, false, false),
                ShortcutPlatform::MacOS,
            ),
            None,
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_print_uses_unshifted_primary_p_and_print_preview_stays_disabled() {
        let file_entries = MENU_CATALOG
            .iter()
            .find(|menu| menu.id == "file")
            .map(|menu| menu.entries)
            .unwrap_or(&[]);
        let print_count = file_entries
            .iter()
            .filter(|entry| matches!(entry, MenuEntrySpec::Item(item) if item.id == "file.print"))
            .count();
        assert_eq!(print_count, 1);
        let print = file_entries
            .iter()
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item) if item.id == "file.print" => Some(*item),
                _ => None,
            });
        assert!(print.is_some_and(|item| {
            item.label == "Print..."
                && item.accessibility_name == "Print screenplay"
                && item.behavior == MenuItemBehavior::Command(CommandId::FilePrint)
                && item.shortcut.is_some_and(|shortcut| {
                    shortcut.primary
                        && shortcut.key == ShortcutKey::P
                        && !shortcut.shift
                        && !shortcut.control
                        && !shortcut.option
                })
        }));

        let preview_index = file_entries.iter().position(|entry| {
            matches!(entry, MenuEntrySpec::Item(item) if item.id == "file.print-preview")
        });
        let print_index = file_entries
            .iter()
            .position(|entry| matches!(entry, MenuEntrySpec::Item(item) if item.id == "file.print"));
        assert_eq!(print_index, preview_index.map(|index| index + 1));
        let preview = file_entries
            .iter()
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item) if item.id == "file.print-preview" => Some(*item),
                _ => None,
            });
        assert!(preview.is_some_and(|item| matches!(
            item.behavior,
            MenuItemBehavior::DisabledPlaceholder(_)
        )));
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::Windows, true, false, false, false),
                ShortcutPlatform::Windows,
            ),
            Some(CommandId::FilePrint),
        );
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::Windows, true, false, false, true),
                ShortcutPlatform::Windows,
            ),
            Some(CommandId::FileSaveAsPdf),
        );
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::Windows, false, false, false, false),
                ShortcutPlatform::Windows,
            ),
            None,
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn print_remains_a_disabled_placeholder_off_macos() {
        let print = MENU_CATALOG
            .iter()
            .flat_map(|menu| menu.entries)
            .find_map(|entry| match entry {
                MenuEntrySpec::Item(item) if item.id == "file.print" => Some(*item),
                _ => None,
            });
        assert!(print.is_some_and(|item| matches!(
            item.behavior,
            MenuItemBehavior::DisabledPlaceholder(_)
        )));
        assert_eq!(
            command_for_key_event_on_platform(
                egui::Key::P,
                modifiers(ShortcutPlatform::Windows, true, false, false, false),
                ShortcutPlatform::Windows,
            ),
            None,
        );
    }

    #[test]
    fn menu_accelerator_recognition_requires_exact_mac_and_windows_chords() {
        let key = egui::Key::D;
        assert_eq!(
            command_for_key_event_on_platform(
                key,
                modifiers(ShortcutPlatform::MacOS, true, false, false, false),
                ShortcutPlatform::MacOS,
            ),
            Some(CommandId::EditorDualDialogueToggle),
        );
        assert_eq!(
            command_for_key_event_on_platform(
                key,
                modifiers(ShortcutPlatform::Windows, true, true, false, false),
                ShortcutPlatform::Windows,
            ),
            Some(CommandId::EditorDualDialogueToggle),
        );
        for (platform, cases) in [
            (
                ShortcutPlatform::MacOS,
                [
                    modifiers(ShortcutPlatform::MacOS, false, false, false, false),
                    modifiers(ShortcutPlatform::MacOS, true, true, false, false),
                    modifiers(ShortcutPlatform::MacOS, true, false, true, false),
                    modifiers(ShortcutPlatform::MacOS, true, false, false, true),
                ],
            ),
            (
                ShortcutPlatform::Windows,
                [
                    modifiers(ShortcutPlatform::Windows, false, true, false, false),
                    modifiers(ShortcutPlatform::Windows, true, false, false, false),
                    modifiers(ShortcutPlatform::Windows, true, true, true, false),
                    modifiers(ShortcutPlatform::Windows, true, true, false, true),
                ],
            ),
        ] {
            for modifiers in cases {
                assert_eq!(command_for_key_event_on_platform(key, modifiers, platform), None);
            }
        }
        assert_eq!(
            command_for_key_event_on_platform(
                key,
                modifiers(ShortcutPlatform::Other, true, false, false, false),
                ShortcutPlatform::Other,
            ),
            None,
        );
        assert_eq!(
            menu_shortcut_label(&dual_dialogue_item()),
            match ShortcutPlatform::current() {
                ShortcutPlatform::MacOS => "Command+D",
                ShortcutPlatform::Windows => "Control+Alt+D",
                ShortcutPlatform::Other => "",
            },
        );
    }

    #[test]
    fn in_window_accelerator_dispatch_only_recognizes_key_press_not_release() {
        let press = egui::Event::Key {
            key: egui::Key::D,
            physical_key: Some(egui::Key::D),
            pressed: true,
            repeat: false,
            modifiers: modifiers(ShortcutPlatform::Windows, true, true, false, false),
        };
        let release = egui::Event::Key {
            key: egui::Key::D,
            physical_key: Some(egui::Key::D),
            pressed: false,
            repeat: false,
            modifiers: modifiers(ShortcutPlatform::Windows, true, true, false, false),
        };
        assert_eq!(
            command_for_pressed_key_event(&press, ShortcutPlatform::Windows),
            Some(CommandId::EditorDualDialogueToggle),
        );
        assert_eq!(
            command_for_pressed_key_event(&release, ShortcutPlatform::Windows),
            None,
        );
    }
}
