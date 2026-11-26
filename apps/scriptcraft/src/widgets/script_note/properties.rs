//! ScriptNote Color and Type values projected from the semantic model.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};
use scriptcraft_model::{
    formatted_text::InlineColor,
    script_notes::ScriptNoteColorProjection,
    ScriptNote,
    ScriptNotes,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypeMenuChoice {
    None,
    Existing(String),
    New,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PropertyMenuAction {
    Color(InlineColor),
    ColorDefault,
    Type(TypeMenuChoice),
}

const COLOR_VIEWPORT_NAME: &str = "scriptcraft-script-note-color-menu";
const TYPE_VIEWPORT_NAME: &str = "scriptcraft-script-note-type-menu";
const COLOR_ROW_HEIGHT: f32 = 22.0;
const TYPE_ROW_HEIGHT: f32 = 24.0;
const MENU_MAX_WIDTH: f32 = 360.0;

/// The note palette is None, Black, followed by the existing twelve menu colors.
/// InlineColor remains the sole authority for labels, source values, and paint RGB.
pub(crate) fn color_choices() -> impl Iterator<Item = InlineColor> {
    std::iter::once(InlineColor::Black).chain(InlineColor::MENU_PALETTE.iter().copied())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteColorPresentation {
    pub(crate) color: Color32,
    pub(crate) checked_color: Option<InlineColor>,
    pub(crate) accessible_name: String,
    pub(crate) default_color: Option<InlineColor>,
}

/// The single desktop projection for a note color. Theme-default follows the
/// active page palette; unsupported source remains visibly Black and unchecked.
pub(crate) fn resolve_color(
    projection: ScriptNoteColorProjection,
    page_text: Color32,
) -> NoteColorPresentation {
    match projection {
        ScriptNoteColorProjection::ThemeDefault => NoteColorPresentation {
            color: page_text,
            checked_color: None,
            accessible_name: "Theme default".to_owned(),
            default_color: None,
        },
        ScriptNoteColorProjection::Explicit(color) => {
            let [red, green, blue] = color.rgb8();
            NoteColorPresentation {
                color: Color32::from_rgb(red, green, blue),
                checked_color: Some(color),
                accessible_name: color.label().to_owned(),
                default_color: Some(color),
            }
        }
        ScriptNoteColorProjection::Unsupported => NoteColorPresentation {
            color: Color32::BLACK,
            checked_color: None,
            accessible_name: "Unsupported".to_owned(),
            default_color: None,
        },
    }
}

/// Current semantic Types in source/creation order. Equality is deliberately
/// exact and case-sensitive; the list is rebuilt from the Project each time.
pub(crate) fn used_types(notes: &ScriptNotes) -> Vec<String> {
    let mut types = Vec::new();
    for note in notes.iter() {
        if let Some(note_type) = note.note_type.as_deref().filter(|value| !value.is_empty()) {
            if !types.iter().any(|existing| existing == note_type) {
                types.push(note_type.to_owned());
            }
        }
    }
    types
}

pub(crate) fn color_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of(COLOR_VIEWPORT_NAME)
}

pub(crate) fn type_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of(TYPE_VIEWPORT_NAME)
}

pub(crate) fn close_color_viewport(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(color_viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn close_type_viewport(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(type_viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn flush_pending_closes(
    ctx: &egui::Context,
    state: &mut super::state::ScriptNoteWorkspaceState,
) {
    if state.color_menu_close_pending {
        close_color_viewport(ctx);
        state.color_menu_close_pending = false;
        state.color_menu_focused = false;
        state.color_menu_focus_pending = false;
        state.color_menu_measured_size = None;
    }
    if state.type_menu_close_pending {
        close_type_viewport(ctx);
        state.type_menu_close_pending = false;
        state.type_menu_focused = false;
        state.type_menu_focus_pending = false;
        state.type_menu_measured_size = None;
    }
}

pub(crate) fn show_native_menus(
    ctx: &egui::Context,
    state: &mut super::state::ScriptNoteWorkspaceState,
    note: &ScriptNote,
    types: &[String],
) -> Option<PropertyMenuAction> {
    if state.new_type_dialog_open {
        return None;
    }
    if !state.color_menu_open {
        state.color_menu_measured_size = None;
    }
    if !state.type_menu_open {
        state.type_menu_measured_size = None;
    }
    let mut action = None;
    if state.color_menu_open {
        show_color_viewport(ctx, state, note.color_projection(), &mut action);
        if state.color_menu_open && state.color_menu_focus_pending {
            ctx.send_viewport_cmd_to(color_viewport_id(), egui::ViewportCommand::Focus);
            state.color_menu_focus_pending = false;
        }
    }
    if state.type_menu_open && action.is_none() {
        show_type_viewport(ctx, state, note, types, &mut action);
        if state.type_menu_open && state.type_menu_focus_pending {
            ctx.send_viewport_cmd_to(type_viewport_id(), egui::ViewportCommand::Focus);
            state.type_menu_focus_pending = false;
        }
    }
    action
}

fn menu_metrics(
    ctx: &egui::Context,
    labels: impl Iterator<Item = String>,
    font_size: f32,
    extra_width: f32,
    popup_horizontal_margin: f32,
) -> f32 {
    let font = FontId::proportional(font_size);
    let widest = ctx.fonts_mut(|fonts| {
        labels
            .map(|label| fonts.layout_no_wrap(label, font.clone(), Color32::WHITE).size().x)
            .fold(0.0_f32, f32::max)
    });
    (widest + extra_width + popup_horizontal_margin).clamp(128.0, MENU_MAX_WIDTH)
}

fn popup_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).shadow(egui::Shadow::NONE)
}

fn popup_total_margins(style: &egui::Style) -> (f32, f32) {
    let margin = popup_frame(style).total_margin();
    (
        margin.left + margin.right,
        margin.top + margin.bottom,
    )
}

fn reconcile_menu_size(
    ctx: &egui::Context,
    id: egui::ViewportId,
    native_size: Vec2,
    popup_size: Option<Vec2>,
    measured_size: &mut Option<Vec2>,
) {
    let Some(popup_size) = popup_size.filter(|size| {
        size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0
    }) else {
        return;
    };

    *measured_size = Some(popup_size);
    if popup_size != native_size {
        ctx.send_viewport_cmd_to(id, egui::ViewportCommand::InnerSize(popup_size));
    }
}

fn menu_viewport_geometry(
    state: &super::state::ScriptNoteWorkspaceState,
    id: egui::ViewportId,
    width: f32,
    desired_height: f32,
    local_left: f32,
) -> (egui::ViewportBuilder, Vec2) {
    let card = state.native_position.unwrap_or(Pos2::ZERO);
    let top = card.y + super::geometry::CARD_SIZE.y + 4.0;
    let bounds = state.work_bounds.map(|bounds| bounds.rect())
        .filter(|rect| rect.is_finite() && rect.is_positive());
    let width = bounds.map_or(width, |rect| width.min(rect.width()).max(1.0));
    let height = bounds
        .map_or(desired_height, |rect| desired_height.min((rect.bottom() - top).max(1.0)))
        .max(1.0);
    let mut position = Pos2::new(card.x + local_left, top);
    if let Some(bounds) = bounds {
        position.x = position.x.clamp(bounds.left(), (bounds.right() - width).max(bounds.left()));
    }
    let size = Vec2::new(width, height);
    let builder = egui::ViewportBuilder::default()
        .with_title(if id == color_viewport_id() { "ScriptNote Color" } else { "ScriptNote Type" })
        .with_position(position)
        .with_inner_size(size)
        .with_min_inner_size(size)
        .with_max_inner_size(size)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false);
    (builder, size)
}

fn show_color_viewport(
    root_ctx: &egui::Context,
    state: &mut super::state::ScriptNoteWorkspaceState,
    projection: ScriptNoteColorProjection,
    action: &mut Option<PropertyMenuAction>,
) {
    let (horizontal_margin, vertical_margin) =
        popup_total_margins(root_ctx.style().as_ref());
    let desired_width = menu_metrics(
        root_ctx,
        std::iter::once("None".to_owned()).chain(color_choices().map(|color| color.label().to_owned())),
        12.0,
        54.0,
        horizontal_margin,
    );
    let color_row_count = color_choices().count() + 1;
    let row_gap = 2.0;
    let desired_height = vertical_margin
        + color_row_count as f32 * COLOR_ROW_HEIGHT
        + color_row_count.saturating_sub(1) as f32 * row_gap;
    let desired_size = state.color_menu_measured_size
        .unwrap_or_else(|| Vec2::new(desired_width, desired_height));
    let (builder, size) = menu_viewport_geometry(
        state,
        color_viewport_id(),
        desired_size.x,
        desired_size.y,
        super::geometry::HORIZONTAL_INSET,
    );
    let was_focused = state.color_menu_focused;
    let mut selected = None;
    let mut focused = false;
    let mut dismissed = false;
    let mut unsupported = false;
    let mut popup_size = None;
    root_ctx.show_viewport_immediate(color_viewport_id(), builder, |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            unsupported = true;
            return;
        }
        let ctx = viewport_ui.ctx().clone();
        focused = ctx.input(|input| input.viewport().focused) == Some(true);
        let mut ui = viewport_ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, size)));
        ui.set_min_size(size);
        ui.style_mut().visuals.menu_corner_radius =
            egui::CornerRadius::same(super::geometry::CARD_RADIUS as u8);
        let popup = popup_frame(ui.style()).show(&mut ui, |ui| {
            ui.set_min_width((size.x - horizontal_margin).max(0.0));
            ui.spacing_mut().button_padding = Vec2::new(4.0, 0.0);
            ui.spacing_mut().item_spacing = Vec2::new(6.0, row_gap);
            ui.spacing_mut().interact_size.y = COLOR_ROW_HEIGHT;
            egui::ScrollArea::vertical()
                .id_salt("script-note-color-rows")
                .max_height((size.y - vertical_margin).max(1.0))
                .show(ui, |ui| {
                    let none_check = crate::chrome::menu::menu_check_atom(
                        projection == ScriptNoteColorProjection::ThemeDefault,
                    );
                    let mut none_swatch = egui::Atom::custom(
                        egui::Id::new("script-note-color-no-color-swatch-slot"),
                        Vec2::new(18.0, COLOR_ROW_HEIGHT),
                    );
                    none_swatch.size = Some(Vec2::new(18.0, COLOR_ROW_HEIGHT));
                    none_swatch.align = egui::Align2::CENTER_CENTER;
                    let mut none_label = egui::Atom::from(egui::RichText::new("None").size(12.0));
                    none_label.grow = true;
                    none_label.align = egui::Align2::LEFT_CENTER;
                    let none_row = ui.add(
                        egui::Button::new((none_check, none_swatch, none_label))
                            .frame(false)
                            .fill(Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .min_size(Vec2::new((size.x - horizontal_margin).max(0.0), COLOR_ROW_HEIGHT)),
                    );
                    none_row.widget_info(|| egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        "None".to_owned(),
                    ));
                    crate::chrome::menu::paint_script_note_no_color_swatch(ui, none_row.rect);
                    if none_row.clicked() { selected = Some(PropertyMenuAction::ColorDefault); }

                    for color in color_choices() {
                        let checked = projection == ScriptNoteColorProjection::Explicit(color);
                        let check = crate::chrome::menu::menu_check_atom(checked);
                        let mut swatch = egui::Atom::custom(
                            egui::Id::new(("script-note-color-swatch-slot", color.label())),
                            Vec2::new(18.0, COLOR_ROW_HEIGHT),
                        );
                        swatch.size = Some(Vec2::new(18.0, COLOR_ROW_HEIGHT));
                        swatch.align = egui::Align2::CENTER_CENTER;
                        let mut label = egui::Atom::from(egui::RichText::new(color.label()).size(12.0));
                        label.grow = true;
                        label.align = egui::Align2::LEFT_CENTER;
                        let row = ui.add(
                            egui::Button::new((check, swatch, label))
                                .frame(false)
                                .fill(Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .min_size(Vec2::new((size.x - horizontal_margin).max(0.0), COLOR_ROW_HEIGHT)),
                        );
                        row.widget_info(|| egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            color.label().to_owned(),
                        ));
                        let [red, green, blue] = color.rgb8();
                        crate::chrome::menu::paint_script_note_color_swatch(ui, row.rect, [red, green, blue]);
                        if row.clicked() { selected = Some(PropertyMenuAction::Color(color)); }
                    }
                });
        });
        popup_size = Some(popup.response.rect.size());
        dismissed = ctx.input(|input| {
            input.viewport().close_requested()
                || input.key_pressed(egui::Key::Escape)
                || (input.pointer.any_click()
                    && input.pointer.interact_pos().is_some_and(|pos| !popup.response.rect.contains(pos)))
        });
    });
    state.color_menu_focused = focused;
    if selected.is_some() || dismissed || unsupported || (was_focused && !focused) {
        state.color_menu_open = false;
        state.color_menu_close_pending = true;
        state.color_menu_measured_size = None;
    } else {
        reconcile_menu_size(
            root_ctx,
            color_viewport_id(),
            size,
            popup_size,
            &mut state.color_menu_measured_size,
        );
    }
    if let Some(selected) = selected { *action = Some(selected); }
}

fn show_type_viewport(
    root_ctx: &egui::Context,
    state: &mut super::state::ScriptNoteWorkspaceState,
    note: &ScriptNote,
    types: &[String],
    action: &mut Option<PropertyMenuAction>,
) {
    let (horizontal_margin, vertical_margin) =
        popup_total_margins(root_ctx.style().as_ref());
    let labels = std::iter::once("None".to_owned())
        .chain(types.iter().cloned())
        .chain(std::iter::once("New Type...".to_owned()));
    let desired_width = menu_metrics(root_ctx, labels, 13.0, 34.0, horizontal_margin);
    let row_count = types.len() + 2;
    let row_gap = root_ctx.style().spacing.item_spacing.y;
    let separator_height = root_ctx.style().visuals.widgets.noninteractive.bg_stroke.width;
    let desired_height = vertical_margin
        + row_count as f32 * TYPE_ROW_HEIGHT
        + row_count as f32 * row_gap
        + separator_height;
    let desired_size = state.type_menu_measured_size
        .unwrap_or_else(|| Vec2::new(desired_width, desired_height));
    let (builder, size) = menu_viewport_geometry(
        state,
        type_viewport_id(),
        desired_size.x,
        desired_size.y,
        super::geometry::HORIZONTAL_INSET + 58.0,
    );
    let was_focused = state.type_menu_focused;
    let mut selected = None;
    let mut focused = false;
    let mut dismissed = false;
    let mut unsupported = false;
    let mut popup_size = None;
    root_ctx.show_viewport_immediate(type_viewport_id(), builder, |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            unsupported = true;
            return;
        }
        let ctx = viewport_ui.ctx().clone();
        focused = ctx.input(|input| input.viewport().focused) == Some(true);
        let mut ui = viewport_ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, size)));
        ui.set_min_size(size);
        ui.style_mut().visuals.menu_corner_radius =
            egui::CornerRadius::same(super::geometry::CARD_RADIUS as u8);
        let popup = popup_frame(ui.style()).show(&mut ui, |ui| {
            ui.set_min_width((size.x - horizontal_margin).max(0.0));
            ui.spacing_mut().button_padding = Vec2::new(4.0, 0.0);
            ui.spacing_mut().item_spacing.y = row_gap;
            ui.spacing_mut().interact_size.y = TYPE_ROW_HEIGHT;
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            egui::ScrollArea::vertical()
                .id_salt("script-note-type-rows")
                .max_height((size.y - vertical_margin).max(1.0))
                .show(ui, |ui| {
                    let option_font = FontId::proportional(13.0);
                    let row_width = (size.x - horizontal_margin).max(0.0);
                    let none_choice = TypeMenuChoice::None;
                    let none_row = add_type_row(
                        ui,
                        row_width,
                        TYPE_ROW_HEIGHT,
                        note.note_type.is_none(),
                        "None",
                        option_font.clone(),
                    );
                    if none_row.clicked() {
                        selected = Some(none_choice);
                    }
                    for note_type in types {
                        let choice = TypeMenuChoice::Existing(note_type.clone());
                        let row = add_type_row(
                            ui,
                            row_width,
                            TYPE_ROW_HEIGHT,
                            note.note_type.as_ref() == Some(note_type),
                            note_type,
                            option_font.clone(),
                        );
                        if row.clicked() {
                            selected = Some(choice);
                        }
                    }
                    ui.separator();
                    if add_type_row(
                        ui,
                        row_width,
                        TYPE_ROW_HEIGHT,
                        false,
                        "New Type...",
                        option_font,
                    ).clicked() {
                        selected = Some(TypeMenuChoice::New);
                    }
                });
        });
        popup_size = Some(popup.response.rect.size());
        dismissed = ctx.input(|input| {
            input.viewport().close_requested()
                || input.key_pressed(egui::Key::Escape)
                || (input.pointer.any_click()
                    && input.pointer.interact_pos().is_some_and(|pos| !popup.response.rect.contains(pos)))
        });
    });
    state.type_menu_focused = focused;
    if selected.is_some() || dismissed || unsupported || (was_focused && !focused) {
        state.type_menu_open = false;
        state.type_menu_close_pending = true;
        state.type_menu_measured_size = None;
    } else {
        reconcile_menu_size(
            root_ctx,
            type_viewport_id(),
            size,
            popup_size,
            &mut state.type_menu_measured_size,
        );
    }
    if let Some(choice) = selected { *action = Some(PropertyMenuAction::Type(choice)); }
}

fn add_type_row(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    checked: bool,
    text: &str,
    font: FontId,
) -> egui::Response {
    let check = crate::chrome::menu::menu_check_atom(checked);
    let mut label = egui::Atom::from(egui::RichText::new(text).font(font));
    label.grow = true;
    label.align = egui::Align2::LEFT_CENTER;
    let response = ui.add(
        egui::Button::new((check, label))
            .frame(false)
            .fill(Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .min_size(Vec2::new(width, height)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(
        egui::WidgetType::Button,
        true,
        text.to_owned(),
    ));
    response
}
