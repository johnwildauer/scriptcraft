//! Single app-private immediate native ScriptNote viewport.

use super::{geometry::{self, CardRegions}, paint, properties, state::ScriptNoteWorkspaceState, text_input, text_paint, text_viewport};
use crate::{
    app::{RootDocumentView, ScriptCraftApp},
    chrome::theme,
    commands::CommandInvocation,
    views::script::{navigate, navigation_enabled, NoteNavigationDirection},
};
use eframe::egui::{self, pos2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use scriptcraft_commands::{
    Command, CommandOrigin, ScriptNoteColorValue, ScriptNoteIdValue, ScriptNotePropertiesValue,
    ScriptNoteSelectionValue, ScriptNoteTextPositionValue, ScriptNoteTimestampValue,
};
use scriptcraft_layout::{LayoutEnvironment, ScriptNoteMetrics};
use scriptcraft_session::editing::{ScriptNoteField, ScriptNoteSelection, ScriptNoteTextPosition};

const VIEWPORT_NAME: &str = "scriptcraft-script-note";

pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of(VIEWPORT_NAME)
}

fn viewport_builder(position: Option<Pos2>) -> egui::ViewportBuilder {
    let builder = egui::ViewportBuilder::default()
        .with_title("ScriptNote")
        .with_inner_size(geometry::CARD_SIZE)
        .with_min_inner_size(geometry::CARD_SIZE)
        .with_max_inner_size(geometry::CARD_SIZE)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false);
    position.map_or(builder.clone(), |position| builder.with_position(position))
}

pub(crate) fn close(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Close);
}

pub(crate) fn apply_property_menu_action(
    app: &mut ScriptCraftApp,
    root_ctx: &egui::Context,
    action: properties::PropertyMenuAction,
) {
    let Some(note) = app.script_note_workspace.open_note.as_ref()
        .and_then(|id| app.session.project().script_notes().get(id))
        .cloned()
    else { return };
    let command = match action {
        properties::PropertyMenuAction::Color(color) => {
            let Some(color) = ScriptNoteColorValue::from_core(color) else { return };
            note_properties_command(&note, Some(color), note.note_type.clone())
        }
        properties::PropertyMenuAction::ColorDefault => {
            note_properties_command(&note, Some(ScriptNoteColorValue::ThemeDefault), note.note_type.clone())
        }
        properties::PropertyMenuAction::Type(properties::TypeMenuChoice::New) => {
            app.script_note_workspace.type_draft.clear();
            app.script_note_workspace.type_return_focus = app.script_note_workspace.field;
            app.script_note_workspace.type_dialog_error = false;
            app.script_note_workspace.new_type_dialog_open = true;
            return;
        }
        properties::PropertyMenuAction::Type(properties::TypeMenuChoice::None) => {
            note_properties_command(&note, None, None)
        }
        properties::PropertyMenuAction::Type(properties::TypeMenuChoice::Existing(value)) => {
            note_properties_command(&note, None, Some(value))
        }
    };
    if let Some(command) = command {
        let _ = app.dispatch_command(CommandInvocation::new(command, CommandOrigin::Pointer), root_ctx);
    } else {
        app.show_error("ScriptNote timestamp could not be represented; note unchanged");
    }
}

fn note_properties_command(
    note: &scriptcraft_model::ScriptNote,
    color: Option<ScriptNoteColorValue>,
    note_type: Option<String>,
) -> Option<Command> {
    let modified = ScriptNoteTimestampValue::new(scriptcraft_fdx::current_timestamp()).ok()?;
    Some(Command::ScriptNotePropertiesSet {
        properties: ScriptNotePropertiesValue {
            note: ScriptNoteIdValue::from_core(note.id.clone()),
            color,
            note_type,
            modified,
        },
    })
}

fn global_position_from_root(
    pointer: Pos2,
    root_outer_origin: Pos2,
    root_content_origin: Pos2,
) -> Pos2 {
    // Native viewport outer positions and egui pointer coordinates are both
    // logical points. Restore the root outer origin and remove its content inset.
    Pos2::new(
        root_outer_origin.x + pointer.x - root_content_origin.x,
        root_outer_origin.y + pointer.y - root_content_origin.y,
    )
}

pub(crate) fn show(app: &mut ScriptCraftApp, root_ctx: &egui::Context) {
    let Some(note_id) = app.script_note_workspace.open_note.clone() else {
        close(root_ctx);
        return;
    };
    if app.root_document_view != RootDocumentView::Script
        || app.active_editor_surface() != scriptcraft_commands::EditorSurface::Body
        || app.title_view.surface_focused
        || app.element_windows.owns_input()
        || app.replacement_open_pending()
        || app.pending_fdx_open.is_some()
        || app.native_output_modal_active
    {
        app.dismiss_script_note(root_ctx);
        return;
    }
    let Some(note) = app.session.project().script_notes().get(&note_id).cloned() else {
        app.dismiss_script_note(root_ctx);
        return;
    };
    let prepared = match app.document_font_store.prepare_document(app.session.project()) {
        Ok(prepared) => prepared,
        Err(_) => {
            app.show_error("SCRIPT_NOTE_FONT_PREPARATION_FAILED: card unavailable; document unchanged");
            return;
        }
    };
    match app.document_font_store.register_prepared_fonts(root_ctx, &prepared) {
        Ok(true) => {}
        Ok(false) => {
            root_ctx.request_repaint();
            return;
        }
        Err(_) => {
            app.show_error("SCRIPT_NOTE_FONT_REGISTRATION_FAILED: card unavailable; document unchanged");
            // Dismiss the transient card after this bounded diagnostic so a
            // persistent registration error cannot flood every subsequent frame.
            app.dismiss_script_note(root_ctx);
            return;
        }
    }
    let environment = LayoutEnvironment::new(app.session.layout_inputs().page_only(), prepared.environment.clone());
    let layouts = match app.script_note_layout_cache.get_or_build(
        &note, &environment, &prepared, ScriptNoteMetrics::default(),
    ) {
        Ok(layouts) => layouts,
        Err(_) => {
            app.show_error("SCRIPT_NOTE_LAYOUT_FAILED: card unavailable; document unchanged");
            return;
        }
    };
    let layout = layouts.actual;
    let placeholder_layout = layouts.placeholder;
    let Some(anchor) = app.script_note_workspace.marker_anchor else {
        app.dismiss_script_note(root_ctx);
        return;
    };
    if !anchor.baseline_visible && !app.script_note_workspace.reveal_pending {
        app.dismiss_script_note(root_ctx);
        return;
    }
    let (root_outer_rect, root_focused) = root_ctx.input(|input| (
        input.viewport().outer_rect,
        input.viewport().focused == Some(true),
    ));
    let outer_origin = root_outer_rect.map_or(Pos2::ZERO, |rect| rect.min);
    let content_origin = root_ctx.content_rect().min;
    let baseline = global_position_from_root(
        Pos2::new(anchor.marker_right, anchor.baseline),
        outer_origin,
        content_origin,
    );
    let marker_left = outer_origin.x + anchor.marker_left - content_origin.x;
    let marker_right = outer_origin.x + anchor.marker_right - content_origin.x;
    let work_bounds = app.script_note_workspace.work_bounds.map(|bounds| bounds.rect());
    let position = geometry::place_card(
        baseline.y,
        marker_left,
        marker_right,
        root_outer_rect,
        work_bounds,
    );
    let position_changed = app.script_note_workspace.native_position != Some(position);
    if position_changed {
        app.script_note_workspace.native_position = Some(position);
        root_ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::OuterPosition(position));
    }
    let position = app.script_note_workspace.native_position;
    let view_id = viewport_id();
    let current_selection = app.session.script_note_selection().clone();
    text_viewport::reveal_title_offset(&layout, &current_selection, &mut app.script_note_workspace);
    let note_id_copy = note_id.clone();
    let mut commands = Vec::new();
    let mut dismiss = false;
    let mut unsupported = false;
    let mut timestamp_error = false;
    let mut navigation = None;

    root_ctx.show_viewport_immediate(view_id, viewport_builder(position), |viewport_ui, class| {
        if class != egui::ViewportClass::Immediate {
            unsupported = true;
            return;
        }
        let ctx = viewport_ui.ctx().clone();
        app.script_note_workspace.viewport_focused = ctx.input(|input| input.viewport().focused) == Some(true);
        let native_scale = ctx.input(|input| input.viewport().native_pixels_per_point);
        ctx.set_zoom_factor(crate::chrome::status_bar::ui_zoom_factor(native_scale));
        let (_, _, child_palette) = theme::configure_appearance(
            &ctx, app.appearance_mode, app.appearance_applications.get(theme::AppearanceContext::Root),
        );
        let mut root = viewport_ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, geometry::CARD_SIZE)));
        root.set_min_size(geometry::CARD_SIZE);
        let regions = CardRegions::at(Pos2::ZERO);
        let (previous_enabled, next_enabled) =
            navigation_enabled(app.session.project(), Some(&note_id_copy));
        let dialog_topmost = app.script_note_workspace.new_type_dialog_open;
        let painter = root.painter().with_clip_rect(regions.card);
        paint::paint(&painter, regions, &note, &layout, placeholder_layout.as_ref(), &prepared, child_palette.script_workspace.page, &app.script_note_workspace, &note_id_copy, &current_selection, previous_enabled, next_enabled);

        let color_presentation = properties::resolve_color(
            note.color_projection(),
            child_palette.script_workspace.page.text,
        );
        let color_rect = regions.color_region;
        let color_name = egui::RichText::new(format!("Color: {}", color_presentation.accessible_name)).color(Color32::TRANSPARENT);
        let color_response = root.put(color_rect, egui::Button::new(color_name).fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
        let type_name = note.note_type.as_deref().unwrap_or("None");
        let type_accessible_name = egui::RichText::new(format!("Type: {type_name}")).color(Color32::TRANSPARENT);
        let type_response = root.put(regions.type_region, egui::Button::new(type_accessible_name).fill(Color32::TRANSPARENT).stroke(Stroke::NONE));
        if !dialog_topmost && color_response.clicked() {
            if app.script_note_workspace.color_menu_open {
                app.script_note_workspace.color_menu_open = false;
                app.script_note_workspace.color_menu_close_pending = true;
            } else {
                if app.script_note_workspace.type_menu_open {
                    app.script_note_workspace.type_menu_open = false;
                    app.script_note_workspace.type_menu_close_pending = true;
                }
                app.script_note_workspace.color_menu_open = true;
                app.script_note_workspace.color_menu_focus_pending = true;
            }
        }
        if !dialog_topmost && type_response.clicked() {
            if app.script_note_workspace.type_menu_open {
                app.script_note_workspace.type_menu_open = false;
                app.script_note_workspace.type_menu_close_pending = true;
            } else {
                if app.script_note_workspace.color_menu_open {
                    app.script_note_workspace.color_menu_open = false;
                    app.script_note_workspace.color_menu_close_pending = true;
                }
                app.script_note_workspace.type_menu_open = true;
                app.script_note_workspace.type_menu_focus_pending = true;
            }
        }

        let title_edit_rect = regions.title;
        let title_response = root.interact(title_edit_rect, root.id().with("script-note-title"), if dialog_topmost { Sense::hover() } else { Sense::click_and_drag() })
            .on_hover_cursor(egui::CursorIcon::Text);
        let body_response = root.interact(regions.body, root.id().with("script-note-body"), if dialog_topmost { Sense::hover() } else { Sense::click_and_drag() })
            .on_hover_cursor(egui::CursorIcon::Text);
        let delete_response = root.interact(regions.delete, root.id().with("script-note-delete"), if dialog_topmost { Sense::hover() } else { Sense::click() })
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let previous_response = root.add_enabled_ui(previous_enabled && !dialog_topmost, |ui| ui.interact(regions.previous, ui.id().with("script-note-previous"), Sense::click())).inner;
        let next_response = root.add_enabled_ui(next_enabled && !dialog_topmost, |ui| ui.interact(regions.next, ui.id().with("script-note-next"), Sense::click())).inner;
        app.script_note_workspace.delete_hovered = delete_response.hovered() || delete_response.is_pointer_button_down_on();

        for (field, response) in [(ScriptNoteField::Title, title_response), (ScriptNoteField::Body, body_response)] {
            if !dialog_topmost && (response.clicked() || response.drag_started()) {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let click_count = if response.triple_clicked() { 3 } else if response.double_clicked() { 2 } else { 1 };
                    if let Some((anchor, focus)) = text_viewport::selection_at(
                        &layout, regions, field, pointer, app.script_note_workspace.body_offset,
                        app.script_note_workspace.title_offset,
                        click_count, &note.title, &note.paragraphs,
                    ) {
                        app.script_note_workspace.field = Some(field);
                        app.script_note_workspace.focused = true;
                        app.script_note_workspace.pointer_captured = response.drag_started();
                        app.script_note_workspace.text_drag_anchor = Some(anchor);
                        commands.push(Command::ScriptNoteSelectionSet { selection: ScriptNoteSelectionValue::Text {
                            note: ScriptNoteIdValue::from_core(note_id_copy.clone()),
                            field: match field { ScriptNoteField::Title => scriptcraft_commands::ScriptNoteFieldValue::Title, ScriptNoteField::Body => scriptcraft_commands::ScriptNoteFieldValue::Body },
                            anchor: ScriptNoteTextPositionValue { paragraph: anchor.paragraph as u32, offset: anchor.offset as u32 },
                            focus: ScriptNoteTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
                        }});
                    }
                }
            }
            if !dialog_topmost && response.dragged() && app.script_note_workspace.pointer_captured {
                if let Some(pointer) = response.interact_pointer_pos() {
                    if let Some((anchor, focus)) = text_viewport::selection_at(
                        &layout, regions, field, pointer, app.script_note_workspace.body_offset,
                        app.script_note_workspace.title_offset,
                        1, &note.title, &note.paragraphs,
                    ) {
                        let selection_anchor = app.script_note_workspace.text_drag_anchor.unwrap_or_else(|| match &current_selection {
                            ScriptNoteSelection::Text { anchor, .. } => *anchor,
                            _ => anchor,
                        });
                        commands.push(Command::ScriptNoteSelectionSet { selection: ScriptNoteSelectionValue::Text {
                            note: ScriptNoteIdValue::from_core(note_id_copy.clone()),
                            field: match field { ScriptNoteField::Title => scriptcraft_commands::ScriptNoteFieldValue::Title, ScriptNoteField::Body => scriptcraft_commands::ScriptNoteFieldValue::Body },
                            anchor: ScriptNoteTextPositionValue { paragraph: selection_anchor.paragraph as u32, offset: selection_anchor.offset as u32 },
                            focus: ScriptNoteTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
                        }});
                    }
                }
            }
            if !dialog_topmost && response.drag_stopped() {
                app.script_note_workspace.pointer_captured = false;
                app.script_note_workspace.text_drag_anchor = None;
            }
        }
        if !dialog_topmost && delete_response.clicked() {
            commands.push(Command::ScriptNoteDelete { note: ScriptNoteIdValue::from_core(note_id_copy.clone()) });
            dismiss = true;
        }
        if !dialog_topmost && previous_response.clicked() { navigation = Some(NoteNavigationDirection::Previous); }
        if !dialog_topmost && next_response.clicked() { navigation = Some(NoteNavigationDirection::Next); }

        let is_closed = ctx.input(|input| input.viewport().close_requested());
        if !dialog_topmost && (is_closed || ctx.input(|input| input.key_pressed(egui::Key::Escape))) {
            dismiss = true;
        }
        if !dialog_topmost { if let Some(pointer) = ctx.input(|input| input.pointer.hover_pos()) {
            for event in ctx.input(|input| input.events.clone()) {
                if let egui::Event::MouseWheel { unit, delta, .. } = event {
                    let wheel = match unit {
                        egui::MouseWheelUnit::Point => delta.y,
                        egui::MouseWheelUnit::Line => delta.y * 24.0,
                        egui::MouseWheelUnit::Page => delta.y * regions.body.height(),
                    };
                    if text_viewport::body_wheel_scroll(&mut app.script_note_workspace, &layout, regions.body, pointer, wheel) {
                        ctx.request_repaint();
                    }
                }
            }
        }}
        if !dialog_topmost {
            let input_result = text_input::handle(
                &ctx, &note_id_copy, &note, &current_selection, &layout, &mut app.script_note_workspace,
            );
            commands.extend(input_result.commands);
            dismiss |= input_result.dismiss;
            timestamp_error |= input_result.timestamp_error;
        }

        let note_focused = ctx.input(|input| input.viewport().focused);
        if app.script_note_workspace.type_dialog_focused
            || app.script_note_workspace.type_menu_focused
            || app.script_note_workspace.color_menu_focused
            || note_focused == Some(true)
            || root_focused
        {
            app.script_note_workspace.family_focus_loss_frames = 0;
        } else {
            app.script_note_workspace.family_focus_loss_frames = app.script_note_workspace.family_focus_loss_frames.saturating_add(1);
            if app.script_note_workspace.family_focus_loss_frames >= 2 {
                app.script_note_workspace.focused = false;
                app.script_note_workspace.ime_active = false;
                app.script_note_workspace.ime_preedit.clear();
                app.script_note_workspace.pointer_captured = false;
                dismiss = true;
            }
        }

        if layout.body_content_height > layout.body_rect.height {
            let track = Rect::from_min_max(pos2(regions.body.right() - 3.0, regions.body.top()), pos2(regions.body.right(), regions.body.bottom()));
            let scrollbar_response = root.interact(track, root.id().with("script-note-body-scrollbar"), Sense::click_and_drag());
            painter.rect_filled(track, 1.5, child_palette.script_workspace.page.scrollbar_background);
            let overflow = (layout.body_content_height.to_points_f32() - layout.body_rect.height.to_points_f32()).max(1.0);
            let fraction = (layout.body_rect.height.to_points_f32() / layout.body_content_height.to_points_f32()).clamp(0.05, 1.0);
            let handle_h = track.height() * fraction;
            if scrollbar_response.drag_started() || scrollbar_response.clicked() {
                app.script_note_workspace.body_scrollbar_dragging = true;
            }
            if app.script_note_workspace.body_scrollbar_dragging {
                if let Some(pointer) = scrollbar_response.interact_pointer_pos() {
                    let travel = (track.height() - handle_h).max(1.0);
                    let normalized = ((pointer.y - track.top() - handle_h * 0.5) / travel).clamp(0.0, 1.0);
                    app.script_note_workspace.body_offset = normalized * overflow;
                    ctx.request_repaint();
                }
            }
            if scrollbar_response.drag_stopped() || ctx.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
                app.script_note_workspace.body_scrollbar_dragging = false;
            }
            let top = track.top() + (track.height() - handle_h) * (app.script_note_workspace.body_offset / overflow);
            painter.rect_filled(Rect::from_min_size(pos2(track.left(), top), Vec2::new(track.width(), handle_h)), 1.5, child_palette.script_workspace.page.scrollbar_neutral);
        }
        text_paint::paint_caret(&painter, regions, &layout, &note_id_copy, &current_selection, &app.script_note_workspace, child_palette.script_workspace.page);

        // Ordinary input focus belongs to this viewport while text focus is live.
        if app.script_note_workspace.focused {
            ctx.memory_mut(|memory| memory.set_focus_lock_filter(
                egui::Id::new(view_id),
                egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
            ));
        }
    });

    if unsupported {
        if !app.script_note_workspace.unsupported_viewport {
            app.show_error("SCRIPT_NOTE_VIEWPORT_UNSUPPORTED: native immediate viewport unavailable; note card closed");
        }
        app.script_note_workspace.unsupported_viewport = true;
        dismiss = true;
    }
    if timestamp_error {
        app.show_error("ScriptNote timestamp could not be represented; note unchanged");
    }
    for command in commands {
        let selection_changed = matches!(command, Command::ScriptNoteSelectionSet { .. });
        let _ = app.dispatch_command(CommandInvocation::new(command, CommandOrigin::Pointer), root_ctx);
        text_viewport::reveal_title_offset(
            &layout,
            app.session.script_note_selection(),
            &mut app.script_note_workspace,
        );
        if selection_changed {
            if let ScriptNoteSelection::Text { note, field: ScriptNoteField::Body, focus, .. } = app.session.script_note_selection() {
                if note == &note_id {
                    if let Some(line_index) = layout.body_lines.iter().position(|line| line.element == focus.paragraph && line.source.start <= focus.offset && focus.offset <= line.source.end) {
                        text_viewport::reveal_body_offset(&layout, line_index, &mut app.script_note_workspace);
                    }
                }
            }
        }
    }
    if let Some(direction) = navigation {
        navigate(app, direction, root_ctx);
    }
    if dismiss { app.dismiss_script_note(root_ctx); }
    if app.script_note_workspace.focus_viewport_on_next_present {
        app.script_note_workspace.focus_viewport_on_next_present = false;
        root_ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
    }
    if app.script_note_workspace.focus_body_on_next_present {
        app.script_note_workspace.focus_body_on_next_present = false;
        app.script_note_workspace.focused = true;
        app.script_note_workspace.field = Some(ScriptNoteField::Body);
        let pos = ScriptNoteTextPosition { paragraph: 0, offset: 0 };
        let cmd = Command::ScriptNoteSelectionSet { selection: ScriptNoteSelectionValue::Text {
            note: ScriptNoteIdValue::from_core(note_id),
            field: scriptcraft_commands::ScriptNoteFieldValue::Body,
            anchor: ScriptNoteTextPositionValue { paragraph: pos.paragraph as u32, offset: pos.offset as u32 },
            focus: ScriptNoteTextPositionValue { paragraph: pos.paragraph as u32, offset: pos.offset as u32 },
        }};
        let _ = app.dispatch_command(CommandInvocation::new(cmd, CommandOrigin::InternalFollowUp), root_ctx);
    }
}
