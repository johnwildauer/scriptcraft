//! Menu presentation policy and the single handoff queue for menu commands.

use super::app_commands::{AppCommand, AppCommandAvailability};
use super::menu_items::{
    LogicalShortcut, MenuEntrySpec, MenuItemBehavior, MenuItemSpec, PlaceholderKind, SemanticIcon,
    ShortcutKey, MENU_CATALOG,
};
use eframe::egui;
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

/// Keeps the Windows-style in-window menu preview on while developing both
/// presentations on macOS. This is a source-only development switch, not a
/// user setting. Do not turn it off in this work-order batch.
pub(super) const IN_WINDOW_MENU_PREVIEW_ENABLED: bool = true;

/// Native macOS menu installation is target policy, independent of whether
/// the in-window preview is enabled.
pub(super) const NATIVE_MACOS_MENU_ENABLED: bool = cfg!(target_os = "macos");

/// Windows always uses the in-window menu. macOS uses it while the explicit
/// development preview switch is enabled. Other targets have no menu backend.
pub(super) const fn in_window_menu_enabled() -> bool {
    cfg!(windows) || (cfg!(target_os = "macos") && IN_WINDOW_MENU_PREVIEW_ENABLED)
}

/// Native macOS owns accelerators whenever its backend is active, including
/// the dual-preview configuration. Windows keeps in-window ownership.
pub(super) const fn in_window_menu_shortcuts_owned() -> bool {
    cfg!(windows) || (in_window_menu_enabled() && !NATIVE_MACOS_MENU_ENABLED)
}

/// Resolves catalog shortcuts and the Windows-only Redo alias from the logical
/// key and modifiers carried by the pressed key event.
pub(super) fn command_for_key_event(
    key: egui::Key,
    modifiers: egui::Modifiers,
) -> Option<AppCommand> {
    for menu in MENU_CATALOG {
        for entry in menu.entries {
            let MenuEntrySpec::Item(item) = entry else {
                continue;
            };
            let MenuItemBehavior::Command(command) = item.behavior else {
                continue;
            };
            let advertised_match = item.shortcut.is_some_and(|shortcut| {
                shortcut_key(shortcut.key)
                    .is_some_and(|shortcut_key| shortcut_key == key)
                    && shortcut_modifiers_match(shortcut, modifiers)
            });
            let windows_alias_match = cfg!(windows)
                && item.windows_alias.is_some_and(|shortcut| {
                    shortcut_key(shortcut.key).is_some_and(|shortcut_key| shortcut_key == key)
                        && shortcut_modifiers_match(shortcut, modifiers)
                });
            if advertised_match || windows_alias_match {
                return Some(command);
            }
        }
    }
    None
}

fn shortcut_key(key: ShortcutKey) -> Option<egui::Key> {
    match key {
        ShortcutKey::A => Some(egui::Key::A),
        ShortcutKey::B
        | ShortcutKey::D
        | ShortcutKey::E
        | ShortcutKey::F
        | ShortcutKey::G
        | ShortcutKey::H
        | ShortcutKey::I
        | ShortcutKey::J
        | ShortcutKey::K
        | ShortcutKey::L
        | ShortcutKey::M
        | ShortcutKey::Q
        | ShortcutKey::R
        | ShortcutKey::T
        | ShortcutKey::U => None,
        ShortcutKey::C => Some(egui::Key::C),
        ShortcutKey::N => Some(egui::Key::N),
        ShortcutKey::O => Some(egui::Key::O),
        ShortcutKey::P => Some(egui::Key::P),
        ShortcutKey::S => Some(egui::Key::S),
        ShortcutKey::V => Some(egui::Key::V),
        ShortcutKey::W => Some(egui::Key::W),
        ShortcutKey::X => Some(egui::Key::X),
        ShortcutKey::Y => Some(egui::Key::Y),
        ShortcutKey::Z => Some(egui::Key::Z),
        ShortcutKey::Comma
        | ShortcutKey::Semicolon
        | ShortcutKey::Period
        | ShortcutKey::Enter
        | ShortcutKey::BracketLeft
        | ShortcutKey::BracketRight
        | ShortcutKey::Backslash
        | ShortcutKey::Pipe
        | ShortcutKey::ArrowRight => None,
    }
}

fn shortcut_modifiers_match(shortcut: LogicalShortcut, modifiers: egui::Modifiers) -> bool {
    let unexpected_platform_modifier = if cfg!(target_os = "macos") {
        modifiers.ctrl
    } else {
        modifiers.mac_cmd
    };
    modifiers.alt == shortcut.option
        && unexpected_platform_modifier == shortcut.control
        && modifiers.command == shortcut.primary
        && modifiers.shift == shortcut.shift
}

/// One FIFO owned by the application receives activations from both the egui
/// and native adapters. Producers enqueue shared `AppCommand` identities;
/// the application takes the pending batch at its command-dispatch handoff,
/// where the shared dispatcher rechecks current availability.
#[derive(Debug, Default)]
pub(super) struct PendingMenuCommands {
    pending: VecDeque<AppCommand>,
}

impl PendingMenuCommands {
    pub(super) fn enqueue(&mut self, command: AppCommand) {
        self.pending.push_back(command);
    }

    /// Detaches queued commands in activation order for the application to
    /// dispatch; commands enqueued afterward remain for the next handoff.
    pub(super) fn take_pending(&mut self) -> std::collections::vec_deque::IntoIter<AppCommand> {
        std::mem::take(&mut self.pending).into_iter()
    }
}

/// Renders the catalog through egui's menu-button popup behavior. Activation is
/// only enqueued here; application commands are drained at the update boundary.
pub(super) fn show_menu_bar(
    ui: &mut egui::Ui,
    availability: AppCommandAvailability,
    pending: &mut PendingMenuCommands,
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
                                        let shortcut = item
                                            .shortcut
                                            .map(shortcut_label)
                                            .unwrap_or_else(|| {
                                                if matches!(
                                                    item.behavior,
                                                    MenuItemBehavior::DisabledPlaceholder(
                                                        PlaceholderKind::Disclosure
                                                    )
                                                ) {
                                                    "›".to_owned()
                                                } else {
                                                    String::new()
                                                }
                                            });
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
                                                pending.enqueue(command);
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

fn command_enabled(command: AppCommand, availability: AppCommandAvailability) -> bool {
    availability.is_enabled(command)
}

/// The in-window preview intentionally presents Windows-style shortcut labels
/// on every development target, including macOS dual-preview sessions.
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
    pub(super) fn dispatch_pending_menu_commands(&mut self, ctx: &eframe::egui::Context) {
        for command in self.pending_menu_commands.take_pending() {
            self.dispatch_app_command(command, ctx);
            if matches!(
                command,
                AppCommand::Undo
                    | AppCommand::Redo
                    | AppCommand::Cut
                    | AppCommand::Copy
                    | AppCommand::Paste
                    | AppCommand::Delete
                    | AppCommand::SelectAll
            ) {
                self.restore_editor_focus(ctx);
            }
        }
    }
}
