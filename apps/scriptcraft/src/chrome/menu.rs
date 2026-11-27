//! Menu presentation policy and the shared ordered desktop-command handoff.

use super::element_choices::DynamicElementMenuProjection;
use super::element_shortcuts::ReservedElementChords;
use super::formatting_menu::FormattingMenuProjection;
use super::menu_items::{
    activation_for_action, menu_item_can_display_check, menu_item_label, DynamicSubmenuKind,
    LogicalShortcut, MenuAction, MenuEntrySpec, MenuItemBehavior, MenuItemSpec,
    MenuSwatch, PlaceholderKind, SemanticIcon, ShortcutKey, ViewMenuProjection, ZoomStepDirection,
    ZOOM_STEP_ITEMS, MENU_CATALOG,
};
pub(crate) use super::menu_items::MenuActivation;
use super::shortcuts::{shortcut_matches, ShortcutPlatform};
use crate::commands::{command_for_id, AppCommandAvailability, CommandInvocation};
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, EditorSurface, ElementTypeValue, ScriptViewMode, ZoomPercent,
};
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
    icon_slot_width: 18.0,
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
pub(crate) const MENU_PANEL_HEIGHT: f32 = MENU_PRESENTATION.bar_height;

/// Injectable startup choice for the runtime menu policy. The CLI adapter is a
/// later work order; production startup currently supplies `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuPlatformChoice {
    Auto,
    MacOS,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuHostPlatform {
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
pub(crate) enum MenuAcceleratorOwner {
    NativeMacOS,
    InWindow,
    None,
}

/// Build conservative reserved policy from every catalog accelerator, including
/// disabled placeholders and platform aliases. Chords remain reserved even if
/// the requested menu backend is unavailable.
pub(crate) fn reserved_element_chords(platform: ShortcutPlatform) -> ReservedElementChords {
    fn append(
        entries: &'static [MenuEntrySpec],
        platform: ShortcutPlatform,
        out: &mut Vec<LogicalShortcut>,
    ) {
        for entry in entries {
            match entry {
                MenuEntrySpec::Item(item) => {
                    let windows_dual_alias = platform == ShortcutPlatform::Windows
                        && item.behavior
                            == MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle);
                    if !windows_dual_alias {
                        if let Some(shortcut) = item.shortcut {
                            out.push(shortcut);
                        }
                    }
                    if platform == ShortcutPlatform::Windows {
                        if let Some(alias) = item.windows_alias {
                            out.push(alias);
                        }
                    }
                }
                MenuEntrySpec::Submenu(menu) => append(menu.entries, platform, out),
                MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::Zoom) => {
                    for item in ZOOM_STEP_ITEMS {
                        if let Some(shortcut) = item.shortcut {
                            out.push(shortcut);
                        }
                        if platform == ShortcutPlatform::Windows {
                            if let Some(alias) = item.windows_alias {
                                out.push(alias);
                            }
                        }
                    }
                }
                MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::ChangeElementTo) => {}
                MenuEntrySpec::Separator => {}
            }
        }
    }

    let mut menu_shortcuts = Vec::new();
    for menu in MENU_CATALOG {
        append(menu.entries, platform, &mut menu_shortcuts);
    }
    let mut reserved = ReservedElementChords::from_menu_shortcuts(menu_shortcuts);
    if platform == ShortcutPlatform::MacOS {
        // Native application Quit is not a shared File/Edit catalog row.
        reserved.reserve_exact(super::shortcuts::primary_shortcut(ShortcutKey::Q, false));
    }
    for key in [
        ShortcutKey::Backspace,
        ShortcutKey::Delete,
        ShortcutKey::ArrowLeft,
        ShortcutKey::ArrowRight,
        ShortcutKey::ArrowUp,
        ShortcutKey::ArrowDown,
        ShortcutKey::Home,
        ShortcutKey::End,
    ] {
        reserved.reserve_wildcard_key(key);
    }
    for key in [ShortcutKey::Enter, ShortcutKey::Tab] {
        reserved.reserve_exact(LogicalShortcut {
            key,
            primary: false,
            control: false,
            option: false,
            shift: false,
        });
    }
    reserved
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeMenuPolicy {
    backend: MenuBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedMenuPlatformChoice {
    pub(crate) host: MenuHostPlatform,
    pub(crate) requested: MenuPlatformChoice,
}

impl RuntimeMenuPolicy {
    /// Resolves a requested menu presentation against the actual compile target.
    /// A menu-platform choice never changes unrelated host integrations.
    pub(crate) const fn resolve(
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

    pub(crate) const fn for_current_target() -> Self {
        let host = if cfg!(target_os = "macos") {
            MenuHostPlatform::MacOS
        } else if cfg!(windows) {
            MenuHostPlatform::Windows
        } else {
            MenuHostPlatform::Other
        };
        match Self::resolve(host, MenuPlatformChoice::Auto) {
            Ok(policy) => policy,
            Err(_) => Self {
                backend: MenuBackend::None,
            },
        }
    }

    pub(crate) const fn renders_in_window(self) -> bool {
        matches!(self.backend, MenuBackend::InWindow)
    }

    pub(crate) const fn installs_native_macos(self) -> bool {
        matches!(self.backend, MenuBackend::NativeMacOS)
    }

    pub(crate) const fn in_window_shortcuts_owned(self) -> bool {
        matches!(self.accelerator_owner(), MenuAcceleratorOwner::InWindow)
    }

    pub(crate) const fn accelerator_owner(self) -> MenuAcceleratorOwner {
        match self.backend {
            MenuBackend::NativeMacOS => MenuAcceleratorOwner::NativeMacOS,
            MenuBackend::InWindow => MenuAcceleratorOwner::InWindow,
            MenuBackend::None => MenuAcceleratorOwner::None,
        }
    }
}

/// Whether an in-window top-level menu popup currently owns keyboard focus.
/// The menu popup identity is stored by `show_menu_bar` for cross-menu hover.
pub(crate) fn in_window_menu_popup_open(ctx: &egui::Context) -> bool {
    let key = egui::Id::new("scriptcraft-active-top-level-menu-popup");
    ctx.data(|data| data.get_temp::<egui::Id>(key))
        .is_some_and(|popup_id| egui::Popup::is_id_open(ctx, popup_id))
}

/// Resolves catalog shortcuts and the Windows-only Redo alias from the logical
/// key and modifiers carried by the pressed key event.
pub(super) fn command_for_key_event(
    key: egui::Key,
    modifiers: egui::Modifiers,
) -> Option<CommandId> {
    command_for_key_event_on_platform(key, modifiers, ShortcutPlatform::current())
}

pub(crate) fn command_for_key_event_command(
    key: egui::Key,
    modifiers: egui::Modifiers,
) -> Option<Command> {
    match command_for_shortcut_event_on_platform(key, modifiers, ShortcutPlatform::current())? {
        MenuActivation::Command(command) => Some(command),
        MenuActivation::ZoomStep(_) => None,
    }
}

fn command_for_key_event_on_platform(
    key: egui::Key,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> Option<CommandId> {
    match command_for_shortcut_event_on_platform(key, modifiers, platform)? {
        MenuActivation::Command(command) => Some(command.id()),
        MenuActivation::ZoomStep(_) => None,
    }
}

fn command_for_shortcut_event_on_platform(
    key: egui::Key,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> Option<MenuActivation> {
    fn item_activation(
        item: &MenuItemSpec,
        key: egui::Key,
        modifiers: egui::Modifiers,
        platform: ShortcutPlatform,
    ) -> Option<MenuActivation> {
        let dual = item.behavior == MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle);
        if dual && platform == ShortcutPlatform::Other {
            return None;
        }
        let advertised = !(dual && platform == ShortcutPlatform::Windows)
            && item
                .shortcut
                .is_some_and(|shortcut| shortcut_matches(shortcut, key, modifiers, platform));
        let windows_alias = platform == ShortcutPlatform::Windows
            && item
                .windows_alias
                .is_some_and(|shortcut| shortcut_matches(shortcut, key, modifiers, platform));
        if !advertised && !windows_alias {
            return None;
        }
        match item.behavior {
            MenuItemBehavior::Command(command) => crate::commands::command_for_id(command)
                .map(MenuActivation::Command),
            MenuItemBehavior::MenuCommand(action) => {
                Some(activation_for_action(action, EditorSurface::Body))
            }
            MenuItemBehavior::DisabledPlaceholder(_) => None,
        }
    }

    fn visit(
        entries: &'static [MenuEntrySpec],
        key: egui::Key,
        modifiers: egui::Modifiers,
        platform: ShortcutPlatform,
    ) -> Option<MenuActivation> {
        for entry in entries {
            match entry {
                MenuEntrySpec::Submenu(menu) => {
                    if let Some(command) = visit(menu.entries, key, modifiers, platform) {
                        return Some(command);
                    }
                }
                MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::Zoom) => {
                    for item in ZOOM_STEP_ITEMS {
                        if let Some(activation) = item_activation(&item, key, modifiers, platform) {
                            return Some(activation);
                        }
                    }
                }
                MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::ChangeElementTo) => {}
                MenuEntrySpec::Item(item) => {
                    if let Some(activation) = item_activation(item, key, modifiers, platform) {
                        return Some(activation);
                    }
                }
                MenuEntrySpec::Separator => {}
            }
        }
        None
    }
    MENU_CATALOG
        .iter()
        .find_map(|menu| visit(menu.entries, key, modifiers, platform))
}

pub(crate) fn command_for_pressed_key_event(
    event: &egui::Event,
    platform: ShortcutPlatform,
) -> Option<MenuActivation> {
    match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => command_for_shortcut_event_on_platform(*key, *modifiers, platform),
        _ => None,
    }
}

/// One FIFO owned by the application receives deferred typed invocations from
/// egui menus, native menus, and owned menu accelerators. The dispatcher
/// rechecks availability after detaching each activation-ordered batch.
#[derive(Debug, Default)]
pub(crate) struct PendingDesktopCommands {
    pending: VecDeque<PendingDesktopAction>,
}

#[derive(Debug)]
enum PendingDesktopAction {
    Invocation(CommandInvocation),
    ZoomStep {
        direction: ZoomStepDirection,
        origin: CommandOrigin,
    },
}

impl PendingDesktopCommands {
    pub(crate) fn enqueue(&mut self, invocation: CommandInvocation) {
        self.pending
            .push_back(PendingDesktopAction::Invocation(invocation));
    }

    pub(crate) fn enqueue_activation(
        &mut self,
        activation: MenuActivation,
        origin: CommandOrigin,
    ) {
        match activation {
            MenuActivation::Command(command) => {
                self.enqueue(CommandInvocation::new(command, origin));
            }
            MenuActivation::ZoomStep(direction) => {
                self.pending
                    .push_back(PendingDesktopAction::ZoomStep { direction, origin });
            }
        }
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
    ) -> std::collections::vec_deque::IntoIter<PendingDesktopAction> {
        std::mem::take(&mut self.pending).into_iter()
    }
}

#[derive(Debug, Clone, Copy)]
struct MenuContentMetrics {
    required_content_width: f32,
    desired_content_height: f32,
}

fn nonnegative_finite_dimension(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn add_finite_dimensions(left: f32, right: f32) -> f32 {
    let sum = nonnegative_finite_dimension(left) + nonnegative_finite_dimension(right);
    if sum.is_finite() {
        sum
    } else {
        f32::MAX
    }
}

fn popup_content_height(entry_heights: impl IntoIterator<Item = f32>, row_gap: f32) -> f32 {
    let row_gap = nonnegative_finite_dimension(row_gap);
    let mut height = 0.0;
    let mut entry_count = 0usize;
    for entry_height in entry_heights {
        height = add_finite_dimensions(height, entry_height);
        entry_count = entry_count.saturating_add(1);
    }
    let gaps = (0..entry_count.saturating_sub(1))
        .fold(0.0, |height, _| add_finite_dimensions(height, row_gap));
    add_finite_dimensions(height, gaps)
}

pub(crate) fn popup_rows_content_height(
    row_count: usize,
    row_height: f32,
    row_gap: f32,
) -> f32 {
    let row_height = nonnegative_finite_dimension(row_height);
    popup_content_height((0..row_count).map(|_| row_height), row_gap)
}

pub(crate) fn menu_popup_row_height() -> f32 {
    MENU_PRESENTATION.row_height
}

fn menu_font_id(ui: &egui::Ui, size: f32) -> egui::FontId {
    let mut font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Button)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(size));
    font_id.size = size;
    font_id
}

fn measured_menu_text_width(ui: &egui::Ui, text: &str, font_id: egui::FontId) -> f32 {
    let measured = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font_id, egui::Color32::TRANSPARENT)
        .size()
        .x;
    nonnegative_finite_dimension(measured)
}

fn menu_content_metrics(
    ui: &egui::Ui,
    entries: &[MenuEntrySpec],
    view: ViewMenuProjection,
    say_undual: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
    metrics: MenuPresentationMetrics,
    leading_slot: bool,
) -> MenuContentMetrics {
    let item_font = menu_font_id(ui, metrics.item_text_size);
    let shortcut_font = menu_font_id(ui, metrics.shortcut_text_size);
    let spacing = ui.spacing();
    let button_padding_x = nonnegative_finite_dimension(spacing.button_padding.x);
    let row_gap_x = nonnegative_finite_dimension(spacing.item_spacing.x);
    let mut required_content_width = 0.0_f32;
    let mut desired_content_height = 0.0_f32;

    for entry in entries {
        let row_width = match entry {
            MenuEntrySpec::Item(item) => {
                let label_width = measured_menu_text_width(
                    ui,
                    menu_item_label(
                        item,
                        say_undual,
                        invisibles_visible,
                        ruler_visible,
                        view.show_story_map,
                    ),
                    item_font.clone(),
                );
                let shortcut = menu_shortcut_label(item);
                let shortcut_width = if shortcut.is_empty() {
                    0.0
                } else {
                    measured_menu_text_width(ui, &shortcut, shortcut_font.clone())
                };
                let mut width = if leading_slot {
                    add_finite_dimensions(
                        nonnegative_finite_dimension(metrics.icon_slot_width),
                        row_gap_x,
                    )
                } else {
                    0.0
                };
                if item.swatch.is_some() {
                    width = add_finite_dimensions(width, 18.0);
                    width = add_finite_dimensions(width, row_gap_x);
                }
                width = add_finite_dimensions(width, label_width);
                if !shortcut.is_empty() {
                    width = add_finite_dimensions(width, row_gap_x);
                    width = add_finite_dimensions(width, shortcut_width);
                }
                add_finite_dimensions(
                    width,
                    add_finite_dimensions(button_padding_x, button_padding_x),
                )
            }
            MenuEntrySpec::Submenu(menu) => {
                let label_width = measured_menu_text_width(ui, menu.label, item_font.clone());
                let mut width = label_width;
                if leading_slot {
                    width = add_finite_dimensions(
                        width,
                        add_finite_dimensions(
                            nonnegative_finite_dimension(metrics.icon_slot_width),
                            row_gap_x,
                        ),
                    );
                }
                width = add_finite_dimensions(width, row_gap_x);
                width =
                    add_finite_dimensions(width, nonnegative_finite_dimension(metrics.icon_size));
                add_finite_dimensions(
                    width,
                    add_finite_dimensions(button_padding_x, button_padding_x),
                )
            }
            MenuEntrySpec::DynamicSubmenu(kind) => {
                let label_width = measured_menu_text_width(ui, kind.label(), item_font.clone());
                let mut width = label_width;
                if leading_slot {
                    width = add_finite_dimensions(
                        width,
                        add_finite_dimensions(
                            nonnegative_finite_dimension(metrics.icon_slot_width),
                            row_gap_x,
                        ),
                    );
                }
                width = add_finite_dimensions(width, row_gap_x);
                width = add_finite_dimensions(width, nonnegative_finite_dimension(metrics.icon_size));
                add_finite_dimensions(
                    width,
                    add_finite_dimensions(button_padding_x, button_padding_x),
                )
            }
            MenuEntrySpec::Separator => 0.0,
        };
        required_content_width = required_content_width.max(row_width);

        let row_height = match entry {
            MenuEntrySpec::Item(_) | MenuEntrySpec::Submenu(_) | MenuEntrySpec::DynamicSubmenu(_) => {
                nonnegative_finite_dimension(metrics.row_height)
            }
            MenuEntrySpec::Separator => nonnegative_finite_dimension(metrics.separator_spacing),
        };
        desired_content_height = add_finite_dimensions(desired_content_height, row_height);
    }

    let row_gap_y = nonnegative_finite_dimension(spacing.item_spacing.y);
    let inter_row_gaps = (0..entries.len().saturating_sub(1))
        .fold(0.0, |height, _| add_finite_dimensions(height, row_gap_y));
    desired_content_height = add_finite_dimensions(desired_content_height, inter_row_gaps);

    MenuContentMetrics {
        required_content_width: nonnegative_finite_dimension(required_content_width),
        desired_content_height: nonnegative_finite_dimension(desired_content_height),
    }
}

fn menu_entries_require_leading_slot(entries: &[MenuEntrySpec]) -> bool {
    entries.iter().any(|entry| match entry {
        MenuEntrySpec::Item(item) => {
            item.icon.is_some() || menu_item_can_display_check(item)
        }
        MenuEntrySpec::Separator
        | MenuEntrySpec::Submenu(_)
        | MenuEntrySpec::DynamicSubmenu(_) => false,
    })
}

fn render_submenu_button(
    ui: &mut egui::Ui,
    label: egui::RichText,
    leading_slot: bool,
    natural_content_width: f32,
    metrics: MenuPresentationMetrics,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    if leading_slot {
        ui.horizontal(|ui| {
            ui.add_sized(
                egui::vec2(metrics.icon_slot_width, metrics.row_height),
                egui::Label::new(""),
            );
            let leading_width = add_finite_dimensions(
                nonnegative_finite_dimension(metrics.icon_slot_width),
                nonnegative_finite_dimension(ui.spacing().item_spacing.x),
            );
            let button_width = nonnegative_finite_dimension(natural_content_width - leading_width);
            egui::containers::menu::SubMenuButton::from_button(
                egui::Button::new(label)
                    .right_text(egui::containers::menu::SubMenuButton::RIGHT_ARROW)
                    .min_size(egui::vec2(button_width, metrics.row_height)),
            )
            .ui(ui, add_contents);
        });
    } else {
        ui.menu_button(label, add_contents);
    }
}

fn popup_vertical_padding(ui: &egui::Ui, metrics: MenuPresentationMetrics) -> f32 {
    let viewport_height = nonnegative_finite_dimension(ui.ctx().content_rect().height());
    nonnegative_finite_dimension(metrics.popup_padding_y).min((viewport_height / 2.0).max(0.0))
}

pub(crate) fn available_popup_inner_viewport_height(
    ui: &egui::Ui,
    vertical_margin: f32,
) -> f32 {
    let viewport_height = nonnegative_finite_dimension(ui.ctx().content_rect().height());
    nonnegative_finite_dimension(
        viewport_height - nonnegative_finite_dimension(vertical_margin) - 2.0,
    )
}

fn available_menu_inner_viewport_height(ui: &egui::Ui, metrics: MenuPresentationMetrics) -> f32 {
    let popup_padding_y = popup_vertical_padding(ui, metrics);
    available_popup_inner_viewport_height(ui, 2.0 * popup_padding_y)
}

pub(crate) fn render_popup_content(
    ui: &mut egui::Ui,
    desired_content_height: f32,
    available_inner_viewport_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let desired_content_height = nonnegative_finite_dimension(desired_content_height);
    let available_inner_viewport_height =
        nonnegative_finite_dimension(available_inner_viewport_height);
    if desired_content_height <= available_inner_viewport_height {
        add_contents(ui);
    } else {
        egui::ScrollArea::vertical()
            .max_height(available_inner_viewport_height)
            .min_scrolled_height(0.0)
            .auto_shrink([true, true])
            .show(ui, add_contents);
    }
}

fn render_menu_popup(
    ui: &mut egui::Ui,
    entries: &'static [MenuEntrySpec],
    availability: AppCommandAvailability,
    dynamic_elements: &DynamicElementMenuProjection,
    formatting: &FormattingMenuProjection,
    view: ViewMenuProjection,
    script_view_mode: ScriptViewMode,
    say_undual: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
    pending: &mut PendingDesktopCommands,
    metrics: MenuPresentationMetrics,
    available_inner_viewport_height: f32,
) {
    let leading_slot = menu_entries_require_leading_slot(entries);
    let content = menu_content_metrics(
        ui,
        entries,
        view,
        say_undual,
        invisibles_visible,
        ruler_visible,
        metrics,
        leading_slot,
    );
    ui.set_min_width(content.required_content_width);
    render_popup_content(
        ui,
        content.desired_content_height,
        available_inner_viewport_height,
        |ui| {
            render_menu_entries(
                ui,
                entries,
                availability,
                dynamic_elements,
                formatting,
                view,
                script_view_mode,
                say_undual,
                invisibles_visible,
                ruler_visible,
                pending,
                metrics,
                leading_slot,
                content.required_content_width,
            );
        },
    );
}

/// Renders the catalog through egui's menu-button popup behavior. Activation is
/// only enqueued here; application commands are drained at the update boundary.
pub(crate) fn show_menu_bar(
    ui: &mut egui::Ui,
    availability: AppCommandAvailability,
    dynamic_elements: &DynamicElementMenuProjection,
    formatting: &FormattingMenuProjection,
    view: ViewMenuProjection,
    script_view_mode: ScriptViewMode,
    say_undual: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
    pending: &mut PendingDesktopCommands,
) {
    let metrics = MENU_PRESENTATION;
    let popup_padding_y = popup_vertical_padding(ui, metrics);
    let popup_max_height = available_menu_inner_viewport_height(ui, metrics);
    let popup_style = move |style: &mut egui::Style| {
        let separator_color = style.visuals.widgets.noninteractive.bg_stroke.color;
        egui::containers::menu::menu_style(style);
        style.spacing.menu_margin =
            egui::Margin::symmetric(metrics.popup_padding_x as i8, popup_padding_y as i8);
        style.spacing.button_padding =
            egui::vec2(metrics.row_button_padding_x, metrics.row_button_padding_y);
        style.spacing.item_spacing = egui::vec2(metrics.row_item_gap_x, metrics.row_item_gap_y);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(metrics.popup_corner_radius);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(metrics.separator_stroke_width, separator_color);
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
            style.spacing.item_spacing = egui::vec2(metrics.bar_item_gap_x, metrics.bar_item_gap_y);
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
                        render_menu_popup(
                            ui,
                            menu.entries,
                            availability,
                            dynamic_elements,
                            formatting,
                            view,
                            script_view_mode,
                            say_undual,
                            invisibles_visible,
                            ruler_visible,
                            pending,
                            metrics,
                            popup_max_height,
                        );
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

fn render_menu_entries(
    ui: &mut egui::Ui,
    entries: &'static [MenuEntrySpec],
    availability: AppCommandAvailability,
    dynamic_elements: &DynamicElementMenuProjection,
    formatting: &FormattingMenuProjection,
    view: ViewMenuProjection,
    script_view_mode: ScriptViewMode,
    say_undual: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
    pending: &mut PendingDesktopCommands,
    metrics: MenuPresentationMetrics,
    leading_slot: bool,
    natural_content_width: f32,
) {
    for entry in entries {
        match entry {
            MenuEntrySpec::Separator => {
                ui.add(
                    egui::Separator::default()
                        .spacing(metrics.separator_spacing)
                        .horizontal(),
                );
            }
            MenuEntrySpec::Submenu(menu) => {
                let available_inner_viewport_height =
                    available_menu_inner_viewport_height(ui, metrics);
                render_submenu_button(
                    ui,
                    egui::RichText::new(menu.label).size(metrics.item_text_size),
                    leading_slot,
                    natural_content_width,
                    metrics,
                    |ui| {
                        render_menu_popup(
                            ui,
                            menu.entries,
                            availability,
                            dynamic_elements,
                            formatting,
                            view,
                            script_view_mode,
                            say_undual,
                            invisibles_visible,
                            ruler_visible,
                            pending,
                            metrics,
                            available_inner_viewport_height,
                        );
                    },
                );
            }
            MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::ChangeElementTo) => {
                let available_inner_viewport_height =
                    available_menu_inner_viewport_height(ui, metrics);
                let enabled = availability.is_enabled(CommandId::EditorElementChange)
                    && !dynamic_elements.choices().is_empty();
                let response = ui.add_enabled_ui(enabled, |ui| {
                    render_submenu_button(
                        ui,
                        egui::RichText::new("Change Element To").size(metrics.item_text_size),
                        leading_slot,
                        natural_content_width,
                        metrics,
                        |ui| {
                            render_dynamic_element_menu_popup(
                                ui,
                                dynamic_elements,
                                availability,
                                pending,
                                metrics,
                                available_inner_viewport_height,
                            );
                        },
                    )
                });
                response.response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        enabled,
                        "Change Element To".to_owned(),
                    )
                });
            }
            MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::Zoom) => {
                let available_inner_viewport_height =
                    available_menu_inner_viewport_height(ui, metrics);
                let enabled = availability.is_enabled(CommandId::ViewZoomSet);
                let response = ui.add_enabled_ui(enabled, |ui| {
                    render_submenu_button(
                        ui,
                        egui::RichText::new("Zoom").size(metrics.item_text_size),
                        leading_slot,
                        natural_content_width,
                        metrics,
                        |ui| {
                            render_zoom_menu_popup(
                                ui,
                                availability,
                                view,
                                pending,
                                metrics,
                                available_inner_viewport_height,
                            );
                        },
                    )
                });
                response.response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Zoom".to_owned())
                });
            }
            MenuEntrySpec::Item(item) => {
                let activation = menu_item_activation(item);
                let enabled = match item.behavior {
                    MenuItemBehavior::Command(command) => command_enabled(command, availability),
                    MenuItemBehavior::MenuCommand(_) => activation.as_ref().is_some_and(|activation| {
                        let command_id = match activation {
                            MenuActivation::Command(command) => command.id(),
                            MenuActivation::ZoomStep(_) => CommandId::ViewZoomSet,
                        };
                        command_enabled(command_id, availability)
                    }),
                    MenuItemBehavior::DisabledPlaceholder(_) => false,
                };
                let label = menu_item_label(
                    item,
                    say_undual,
                    invisibles_visible,
                    ruler_visible,
                    view.show_story_map,
                );
                let shortcut = menu_shortcut_label(item);
                let icon_atom = menu_icon_atom(
                    item,
                    script_view_mode,
                    formatting,
                    view,
                    metrics,
                    leading_slot,
                );
                let mut label_atom =
                    egui::Atom::from(egui::RichText::new(label).size(metrics.item_text_size));
                label_atom.grow = true;
                label_atom.align = egui::Align2::LEFT_CENTER;
                let button = match (icon_atom, item.swatch.is_some()) {
                    (Some(icon_atom), true) => {
                        let swatch_slot = egui::Atom::custom(
                            egui::Id::new(("menu-swatch-slot", item.id)),
                            egui::vec2(18.0, metrics.row_height),
                        );
                        egui::Button::new((icon_atom, swatch_slot, label_atom))
                    }
                    (None, true) => {
                        let swatch_slot = egui::Atom::custom(
                            egui::Id::new(("menu-swatch-slot", item.id)),
                            egui::vec2(18.0, metrics.row_height),
                        );
                        egui::Button::new((swatch_slot, label_atom))
                    }
                    (Some(icon_atom), false) => egui::Button::new((icon_atom, label_atom)),
                    (None, false) => egui::Button::new(label_atom),
                }
                .shortcut_text(egui::RichText::new(shortcut).size(metrics.shortcut_text_size))
                .min_size(egui::vec2(0.0, metrics.row_height))
                .corner_radius(egui::CornerRadius::same(metrics.row_corner_radius));
                let response = ui.add_enabled(enabled, button);
                if let Some(swatch) = item.swatch {
                    paint_menu_swatch(ui, response.rect, swatch, metrics, leading_slot);
                }
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label.to_owned())
                });
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        response.rect.shrink(1.0),
                        egui::CornerRadius::same(metrics.row_corner_radius),
                        ui.visuals().selection.stroke,
                        egui::StrokeKind::Inside,
                    );
                }
                if response.clicked() {
                    if let Some(activation) = activation {
                        pending.enqueue_activation(activation, CommandOrigin::Menu);
                        ui.close();
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
    }
}

pub(crate) fn paint_script_note_color_swatch(ui: &egui::Ui, row: egui::Rect, rgb: [u8; 3]) {
    paint_menu_swatch(ui, row, MenuSwatch::Palette(rgb), MENU_PRESENTATION, true);
}

pub(crate) fn paint_script_note_no_color_swatch(ui: &egui::Ui, row: egui::Rect) {
    paint_menu_swatch(ui, row, MenuSwatch::NoColor, MENU_PRESENTATION, true);
}

fn paint_menu_swatch(
    ui: &egui::Ui,
    row: egui::Rect,
    swatch: MenuSwatch,
    metrics: MenuPresentationMetrics,
    leading_slot: bool,
) {
    let spacing = nonnegative_finite_dimension(ui.spacing().item_spacing.x);
    let leading_width = if leading_slot {
        nonnegative_finite_dimension(metrics.icon_slot_width) + spacing
    } else {
        0.0
    };
    let x = row.left()
        + nonnegative_finite_dimension(ui.spacing().button_padding.x)
        + leading_width
        + 9.0;
    let center = egui::pos2(x, row.center().y);
    let painter = ui.painter();
    match swatch {
        MenuSwatch::Palette([red, green, blue]) => {
            painter.circle_filled(center, 5.0, egui::Color32::from_rgb(red, green, blue));
        }
        MenuSwatch::NoColor => {
            let color = ui.visuals().weak_text_color();
            painter.circle_stroke(center, 5.0, egui::Stroke::new(1.2, color));
            painter.line_segment(
                [center + egui::vec2(-4.0, 4.0), center + egui::vec2(4.0, -4.0)],
                egui::Stroke::new(1.2, color),
            );
        }
    }
}

fn zoom_menu_metrics(ui: &egui::Ui, metrics: MenuPresentationMetrics) -> MenuContentMetrics {
    let font = menu_font_id(ui, metrics.item_text_size);
    let spacing = ui.spacing();
    let padding_x = nonnegative_finite_dimension(spacing.button_padding.x);
    let gap_x = nonnegative_finite_dimension(spacing.item_spacing.x);
    let measure_row = |width: f32, label: &str, shortcut: Option<&str>| {
        let label_width = measured_menu_text_width(ui, label, font.clone());
        let mut row_width = add_finite_dimensions(
            add_finite_dimensions(metrics.icon_slot_width, gap_x),
            label_width,
        );
        if let Some(shortcut) = shortcut {
            row_width = add_finite_dimensions(
                add_finite_dimensions(row_width, gap_x),
                measured_menu_text_width(ui, shortcut, menu_font_id(ui, metrics.shortcut_text_size)),
            );
        }
        width.max(add_finite_dimensions(row_width, padding_x + padding_x))
    };
    let mut width = 0.0;
    for item in ZOOM_STEP_ITEMS {
        width = measure_row(width, item.label, Some(&menu_shortcut_label(&item)));
    }
    for percent in crate::chrome::status_bar::ZOOM_PRESETS {
        width = measure_row(width, &format!("{percent}%"), None);
    }
    let row_count = 2 + crate::chrome::status_bar::ZOOM_PRESETS.len();
    let separator_height = nonnegative_finite_dimension(metrics.separator_spacing);
    let spacing_y = nonnegative_finite_dimension(spacing.item_spacing.y);
    let desired_content_height = popup_content_height(
        (0..row_count)
            .map(|_| nonnegative_finite_dimension(metrics.row_height))
            .chain(std::iter::once(separator_height)),
        spacing_y,
    );
    MenuContentMetrics {
        required_content_width: nonnegative_finite_dimension(width),
        desired_content_height,
    }
}

fn render_zoom_menu_popup(
    ui: &mut egui::Ui,
    availability: AppCommandAvailability,
    view: ViewMenuProjection,
    pending: &mut PendingDesktopCommands,
    metrics: MenuPresentationMetrics,
    available_inner_viewport_height: f32,
) {
    let content = zoom_menu_metrics(ui, metrics);
    ui.set_min_width(content.required_content_width);
    render_popup_content(
        ui,
        content.desired_content_height,
        available_inner_viewport_height,
        |ui| {
            for item in ZOOM_STEP_ITEMS {
                render_menu_action_row(
                    ui,
                    item.label,
                    item.accessibility_name,
                    menu_shortcut_label(&item),
                    activation_for_action(
                        match item.behavior {
                            MenuItemBehavior::MenuCommand(action) => action,
                            _ => unreachable!("zoom step entries are menu actions"),
                        },
                        EditorSurface::Body,
                    ),
                    availability,
                    metrics,
                    pending,
                    false,
                );
            }
            ui.add(
                egui::Separator::default()
                    .spacing(metrics.separator_spacing)
                    .horizontal(),
            );
            for percent in crate::chrome::status_bar::ZOOM_PRESETS {
                let label = format!("{percent}%");
                let action = MenuAction::SetZoom(percent);
                let activation = activation_for_action(action, EditorSurface::Body);
                render_menu_action_row(
                    ui,
                    &label,
                    &label,
                    String::new(),
                    activation,
                    availability,
                    metrics,
                    pending,
                    view.zoom_percent == percent,
                );
            }
        },
    );
}

fn render_menu_action_row(
    ui: &mut egui::Ui,
    label: &str,
    accessibility_name: &str,
    shortcut: String,
    activation: MenuActivation,
    availability: AppCommandAvailability,
    metrics: MenuPresentationMetrics,
    pending: &mut PendingDesktopCommands,
    checked: bool,
) {
    let command_id = match &activation {
        MenuActivation::Command(command) => command.id(),
        MenuActivation::ZoomStep(_) => CommandId::ViewZoomSet,
    };
    let enabled = availability.is_enabled(command_id);
    let check_atom = menu_check_atom(checked);
    let mut label_atom = egui::Atom::from(
        egui::RichText::new(label).size(metrics.item_text_size),
    );
    label_atom.grow = true;
    label_atom.align = egui::Align2::LEFT_CENTER;
    let response = ui.add_enabled(
        enabled,
        egui::Button::new((check_atom, label_atom))
            .shortcut_text(egui::RichText::new(shortcut).size(metrics.shortcut_text_size))
            .min_size(egui::vec2(0.0, metrics.row_height))
            .corner_radius(egui::CornerRadius::same(metrics.row_corner_radius)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, accessibility_name.to_owned())
    });
    if response.clicked() {
        pending.enqueue_activation(activation, CommandOrigin::Menu);
        ui.close();
        ui.ctx().request_repaint();
    }
}

fn dynamic_element_menu_metrics(
    ui: &egui::Ui,
    projection: &DynamicElementMenuProjection,
    metrics: MenuPresentationMetrics,
) -> MenuContentMetrics {
    let font = menu_font_id(ui, metrics.item_text_size);
    let spacing = ui.spacing();
    let padding_x = nonnegative_finite_dimension(spacing.button_padding.x);
    let gap_x = nonnegative_finite_dimension(spacing.item_spacing.x);
    let mut width = 0.0_f32;
    for choice in projection.choices() {
        let label_width = measured_menu_text_width(ui, choice.label(), font.clone());
        let row_width = add_finite_dimensions(
            add_finite_dimensions(metrics.icon_slot_width, gap_x),
            label_width,
        );
        width = width.max(add_finite_dimensions(
            row_width,
            add_finite_dimensions(padding_x, padding_x),
        ));
    }
    let row_height = nonnegative_finite_dimension(metrics.row_height);
    let row_count = projection.choices().len();
    MenuContentMetrics {
        required_content_width: nonnegative_finite_dimension(width),
        desired_content_height: popup_rows_content_height(
            row_count,
            row_height,
            spacing.item_spacing.y,
        ),
    }
}

fn render_dynamic_element_menu_popup(
    ui: &mut egui::Ui,
    projection: &DynamicElementMenuProjection,
    availability: AppCommandAvailability,
    pending: &mut PendingDesktopCommands,
    metrics: MenuPresentationMetrics,
    available_inner_viewport_height: f32,
) {
    let content = dynamic_element_menu_metrics(ui, projection, metrics);
    ui.set_min_width(content.required_content_width);
    render_popup_content(
        ui,
        content.desired_content_height,
        available_inner_viewport_height,
        |ui| {
            for choice in projection.choices() {
                let checked = projection.focused_type_key.as_ref() == Some(&choice.key);
                let check_atom = menu_check_atom(checked);
                let mut label_atom = egui::Atom::from(
                    egui::RichText::new(choice.label()).size(metrics.item_text_size),
                );
                label_atom.grow = true;
                label_atom.align = egui::Align2::LEFT_CENTER;
                let button = egui::Button::new((check_atom, label_atom))
                    .min_size(egui::vec2(0.0, metrics.row_height))
                    .corner_radius(egui::CornerRadius::same(metrics.row_corner_radius));
                let enabled = availability.is_enabled(CommandId::EditorElementChange);
                let response = ui.add_enabled(enabled, button);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        enabled,
                        choice.label().to_owned(),
                    )
                });
                if response.clicked() {
                    pending.enqueue(CommandInvocation::new(
                        Command::EditorElementChange {
                            element_type: ElementTypeValue::from_key(choice.key.clone()),
                            surface: EditorSurface::Body,
                        },
                        CommandOrigin::Menu,
                    ));
                    ui.close();
                    ui.ctx().request_repaint();
                }
            }
        },
    );
}

fn menu_item_activation(item: &MenuItemSpec) -> Option<MenuActivation> {
    match item.behavior {
        MenuItemBehavior::Command(id) => crate::commands::command_for_id(id)
            .map(MenuActivation::Command),
        MenuItemBehavior::MenuCommand(action) => Some(activation_for_action(action, EditorSurface::Body)),
        MenuItemBehavior::DisabledPlaceholder(_) => None,
    }
}

fn menu_icon_atom(
    item: &MenuItemSpec,
    script_view_mode: ScriptViewMode,
    formatting: &FormattingMenuProjection,
    view: ViewMenuProjection,
    metrics: MenuPresentationMetrics,
    leading_slot: bool,
) -> Option<egui::Atom<'static>> {
    if !leading_slot {
        return None;
    }
    let slot_size = egui::vec2(metrics.icon_slot_width, metrics.row_height);
    if super::menu_items::menu_item_is_checked(item, script_view_mode, formatting, view) {
        return Some(menu_check_atom(true));
    }
    Some(match item.icon.map(material_icon) {
        Some(icon) => {
            let mut atom = egui::Atom::from(
                crate::presentation::material_font::rich_text(icon, None).size(metrics.icon_size),
            );
            atom.size = Some(slot_size);
            atom
        }
        None => egui::Atom::custom(egui::Id::new(("menu-icon-slot", item.id)), slot_size),
    })
}

/// Shared Material check atom for in-window menus and the Body context popup.
pub(crate) fn menu_check_atom(checked: bool) -> egui::Atom<'static> {
    let mut atom = if checked {
        egui::Atom::from(
            crate::presentation::material_font::rich_text(
                egui_material_icons::icons::ICON_CHECK,
                None,
            )
            .size(MENU_PRESENTATION.icon_size),
        )
    } else {
        egui::Atom::from("")
    };
    atom.size = Some(egui::vec2(
        MENU_PRESENTATION.icon_slot_width,
        MENU_PRESENTATION.row_height,
    ));
    atom.align = egui::Align2::CENTER_CENTER;
    atom
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
        ShortcutKey::Colon => ":",
        ShortcutKey::Period => ".",
        ShortcutKey::Plus => "+",
        ShortcutKey::Equals => "=",
        ShortcutKey::Minus => "-",
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
        ShortcutKey::ArrowLeft => "←",
        ShortcutKey::ArrowUp => "↑",
        ShortcutKey::ArrowDown => "↓",
        ShortcutKey::Home => "Home",
        ShortcutKey::End => "End",
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
    // The Plus key already represents the shifted physical equals key; do not
    // render a redundant Shift modifier beside the plus glyph.
    if shortcut.shift && shortcut.key != ShortcutKey::Plus {
        parts.push("Shift");
    }
    parts.push(key);
    parts.join("+")
}

impl crate::app::ScriptCraftApp {
    /// Dispatches the detached queue batch in order. Each shared command
    /// performs its normal current-availability check before it can mutate.
    pub(crate) fn dispatch_pending_commands(&mut self, ctx: &eframe::egui::Context) {
        for action in self.pending_commands.take_pending() {
            match action {
                PendingDesktopAction::Invocation(invocation) => {
                    self.dispatch_command(invocation, ctx);
                }
                PendingDesktopAction::ZoomStep { direction, origin } => {
                    let current = self.active_zoom();
                    let target = zoom_step_target(current, direction, self.active_zoom_policy());
                    if target != current {
                        if let Ok(percent) = ZoomPercent::new(target) {
                            self.dispatch_command(
                                CommandInvocation::new(Command::ViewZoomSet { percent }, origin),
                                ctx,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn zoom_step_target(current: u16, direction: ZoomStepDirection, policy: crate::chrome::status_bar::ZoomPolicy) -> u16 {
    let presets = crate::chrome::status_bar::ZOOM_PRESETS;
    match direction {
        ZoomStepDirection::In => presets
            .into_iter()
            .find(|preset| *preset > current)
            .unwrap_or(policy.max),
        ZoomStepDirection::Out => presets
            .into_iter()
            .rev()
            .find(|preset| *preset < current)
            .unwrap_or(policy.min),
    }
}
