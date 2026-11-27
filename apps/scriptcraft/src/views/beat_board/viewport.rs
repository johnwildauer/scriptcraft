use super::{cache::BeatLayoutCache, constants, geometry::{rect_visible, ViewportGeometry}, paint, pointer::BeatInteraction, text_state::BeatTextState};
use crate::{chrome::theme::ScriptWorkspacePalette, presentation::document_font::PreparedDocumentFonts};
use eframe::egui::{self, Pos2, Rect};
use scriptcraft_fdx::BeatBoardViewState;
use scriptcraft_layout::{BeatCardMetrics, LayoutEnvironment};
use scriptcraft_model::BeatBoard;
use scriptcraft_session::editing::BeatSelection;
use scriptcraft_commands::Command;
use crate::views::beat_drag::{BeatDragCoordinator, BeatDragSource};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct BeatAutoScrollState {
    gesture: crate::presentation::auto_scroll::AutoScrollGesture,
    signature: Option<BeatAutoScrollSignature>,
}

struct BeatAutoScrollSignature {
    viewport: Rect,
    clip: Rect,
    board_width: i64,
    board_height: i64,
    zoom_milli_percent: u32,
    document_identity: Arc<()>,
}

impl BeatAutoScrollSignature {
    fn matches(&self, other: &Self) -> bool {
        self.viewport == other.viewport
            && self.clip == other.clip
            && self.board_width == other.board_width
            && self.board_height == other.board_height
            && self.zoom_milli_percent == other.zoom_milli_percent
            && Arc::ptr_eq(&self.document_identity, &other.document_identity)
    }
}

impl BeatAutoScrollState {
    pub(crate) fn reset(&mut self) {
        self.gesture.reset();
        self.signature = None;
    }

    pub(crate) fn cancel(&mut self) {
        self.gesture.cancel();
    }

    pub(crate) fn is_active(&self) -> bool {
        self.gesture.is_active()
    }

    pub(crate) fn prepare(&mut self, geometry: ViewportGeometry, document_identity: &Arc<()>) -> bool {
        if !usable_geometry(geometry) {
            self.reset();
            return false;
        }
        let next = BeatAutoScrollSignature {
            viewport: geometry.viewport,
            clip: geometry.clip,
            board_width: geometry.board_width.raw(),
            board_height: geometry.board_height.raw(),
            zoom_milli_percent: geometry.view.zoom_milli_percent(),
            document_identity: Arc::clone(document_identity),
        };
        if self.signature.as_ref().is_some_and(|current| !current.matches(&next)) {
            self.reset();
        }
        self.signature = Some(next);
        true
    }
}

fn finite_rect(rect: Rect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width().is_finite()
        && rect.height().is_finite()
}

fn usable_geometry(geometry: ViewportGeometry) -> bool {
    finite_rect(geometry.viewport)
        && finite_rect(geometry.clip)
        && geometry.viewport.is_positive()
        && geometry.clip.is_positive()
        && geometry.board_width.raw() > 0
        && geometry.board_height.raw() > 0
        && geometry.view.zoom_milli_percent() > 0
}

pub(crate) fn show(ui: &mut egui::Ui, board: &BeatBoard, view: &mut BeatBoardViewState, published: &mut Option<ViewportGeometry>, palette: ScriptWorkspacePalette, cache: &mut BeatLayoutCache, prepared: Option<(&LayoutEnvironment, &PreparedDocumentFonts)>, selection: &BeatSelection, token: u64, document_identity: Arc<()>, interaction: &mut BeatInteraction, auto_scroll: &mut BeatAutoScrollState, text_state: &mut BeatTextState, coordinator: &mut BeatDragCoordinator, enabled: bool, card_delete_keyboard_enabled: bool) -> (bool, Vec<Command>, bool) {
    let mut success = true;
    cache.retain_board(board);
    let (rect, _) = ui.allocate_exact_size(ui.available_size().max(egui::Vec2::ZERO), egui::Sense::hover());
    let response = ui.interact(rect, ui.id().with(constants::VIEWPORT_ID), egui::Sense::click_and_drag());
    text_state.viewport_id = Some(response.id);
    let pointer = ui.ctx().input(|input| input.pointer.hover_pos());
    interaction.observe_view(ui.ctx(), *view);
    let mut geometry = ViewportGeometry::new(rect, ui.clip_rect(), board, *view, pointer);
    let scrollbar_consumed = interaction.handle_scrollbars(ui, geometry, view, token, enabled, coordinator);
    geometry = geometry.with_view(*view);
    if !enabled || coordinator.external_engaged() || !auto_scroll.prepare(geometry, &document_identity) {
        auto_scroll.reset();
    } else if scrollbar_consumed {
        auto_scroll.cancel();
    }
    let gesture_frame = if enabled && !coordinator.external_engaged() && usable_geometry(geometry) {
        auto_scroll.gesture.process_beat_frame(ui.ctx(), |point| {
            interaction.middle_scroll_eligible(geometry, point)
        })
    } else {
        auto_scroll.reset();
        crate::presentation::auto_scroll::GestureFrame::default()
    };
    let gesture_owns_input = gesture_frame.active || gesture_frame.owned_input;
    if gesture_frame.active && (gesture_frame.delta.x != 0.0 || gesture_frame.delta.y != 0.0) {
        let origin = geometry.pan(-gesture_frame.delta);
        if let Some(next_view) = BeatBoardViewState::new(origin, geometry.view.zoom_milli_percent()) {
            *view = next_view;
            geometry = geometry.with_view(next_view);
            interaction.observe_view(ui.ctx(), next_view);
        }
    }
    let spatial_command = if scrollbar_consumed || gesture_owns_input { None } else { interaction.input(ui, &response, geometry, board, selection, token, Arc::clone(&document_identity), enabled, coordinator) };
    let preview = if gesture_owns_input { None } else { interaction.preview_card(board, geometry, ui.ctx().input(|i| i.pointer.interact_pos())) };
    *view = geometry.view;
    let mut commands = spatial_command.into_iter().collect::<Vec<_>>();
    if commands.iter().any(|command| matches!(command, Command::BeatBoardCardActivate { .. } | Command::BeatBoardCardGeometrySet { .. }
        | Command::BeatBoardSelectionSet { selection: scriptcraft_commands::BeatSelectionValue::Card { .. } | scriptcraft_commands::BeatSelectionValue::None })) {
        if let Some(id) = text_state.viewport_id { ui.ctx().memory_mut(|memory| memory.surrender_focus(id)); }
        text_state.blur();
    }
    if coordinator.external_engaged() && text_state.focused {
        if let Some(id) = text_state.viewport_id { ui.ctx().memory_mut(|memory| memory.surrender_focus(id)); }
        text_state.blur();
    }
    if !gesture_owns_input && text_state.focused && !ui.ctx().memory(|memory| memory.has_focus(response.id)) {
        text_state.blur();
    }
    let (outside, backdrop, grid_color) = constants::colors(palette);
    ui.painter().rect_filled(rect, 0.0, outside);
    let board_clip = geometry.clip.intersect(geometry.board_screen_rect());
    if board_clip.is_positive() {
        let painter = ui.painter().with_clip_rect(board_clip);
        painter.rect_filled(board_clip, 0.0, backdrop);
        if let Some(visible) = geometry.visible {
            let pixels_per_point = ui.ctx().pixels_per_point();
            let pixels_per_point = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
            let snap = |v: f32| ((v * pixels_per_point - 0.5).round() + 0.5) / pixels_per_point;
            let grid = egui::Stroke::new(constants::GRID_STROKE_PIXELS / pixels_per_point, grid_color);
            let ordinals = |min: f64, max: f64| -> Option<(i64, i64)> {
                let first = ((min - constants::GRID_FIRST_LINE) / constants::GRID_CELL).ceil().max(0.0);
                let last = ((max - constants::GRID_FIRST_LINE) / constants::GRID_CELL).floor();
                (first.is_finite() && last.is_finite() && first <= last && last <= i64::MAX as f64).then_some((first as i64, last as i64))
            };
            if let Some((first, last)) = ordinals(visible.left, visible.right) {
                for n in first..=last {
                    let x = snap(geometry.board_to_screen(constants::GRID_FIRST_LINE + constants::GRID_CELL * n as f64, 0.0).x);
                    painter.line_segment([Pos2::new(x, board_clip.top()), Pos2::new(x, board_clip.bottom())], grid);
                }
            }
            if let Some((first, last)) = ordinals(visible.top, visible.bottom) {
                for n in first..=last {
                    let y = snap(geometry.board_to_screen(0.0, constants::GRID_FIRST_LINE + constants::GRID_CELL * n as f64).y);
                    painter.line_segment([Pos2::new(board_clip.left(), y), Pos2::new(board_clip.right(), y)], grid);
                }
            }
            for card in board.cards.iter().filter(|card| preview.as_ref().is_none_or(|(id, _)| id != &card.id)).filter(|card| rect_visible(card.rect, visible)) {
                let selected = matches!(selection, BeatSelection::Card { card: id } | BeatSelection::Text { card: id, .. } if id == &card.id);
                paint::paint_card_frame(&painter, geometry, card, palette.page, palette.beat.unselected_card_border, selected);
                if let Some((environment, fonts)) = prepared {
                    match cache.get_or_build(card, environment, fonts, BeatCardMetrics::default()) {
                        Ok(layouts) => {
                            super::text_viewport::reveal(&layouts.card, selection, text_state);
                            super::text_paint::paint(&painter, geometry, card, &layouts.card, selection, text_state, palette.page, true);
                            success &= paint::paint_card(&painter, geometry, card, layouts, palette.page, fonts, text_state);
                            super::text_paint::paint(&painter.with_clip_rect(painter.clip_rect().intersect(geometry.board_screen_rect())), geometry, card, &layouts.card, selection, text_state, palette.page, false);
                        },
                        Err(_) => success = false,
                    }
                }
            }
            if let Some((id, rect)) = &preview {
                if let Some(card) = board.cards.iter().find(|card| &card.id == id) {
                    let mut card = card.clone();
                    card.rect = *rect;
                    if rect_visible(card.rect, visible) {
                        paint::paint_card_frame(&painter, geometry, &card, palette.page, palette.beat.unselected_card_border, true);
                        if let Some((environment, fonts)) = prepared {
                            match cache.get_or_build(&card, environment, fonts, BeatCardMetrics::default()) {
                                Ok(layouts) => success &= paint::paint_card(&painter, geometry, &card, layouts, palette.page, fonts, text_state),
                                Err(_) => success = false,
                            }
                        }
                    }
                }
            }
        }
    }
    if !gesture_owns_input && commands.is_empty() && enabled && !scrollbar_consumed && !coordinator.external_engaged() {
        let selected_id = match selection { BeatSelection::Card { card } | BeatSelection::Text { card, .. } => Some(card), _ => None };
        if let (Some(card), Some((environment, fonts))) = (selected_id.and_then(|id| board.cards.iter().find(|card| &card.id == id)), prepared) {
            if let Ok(layouts) = cache.get_or_build(card, environment, fonts, BeatCardMetrics::default()) {
                if let Some(command) = super::text_pointer::handle(ui, &response, geometry, board, selection, Some(&layouts.card), text_state) {
                    commands.push(command);
                }
                if !commands.iter().any(|command| matches!(command, Command::BeatBoardSelectionSet { .. })) {
                    commands.extend(super::text_input::handle(ui.ctx(), selection, board, Some(&layouts.card), text_state));
                }
            }
        } else if !ui.ctx().input(|input| input.pointer.primary_down()) {
            text_state.blur();
        }
    }
    if commands.is_empty() && card_delete_keyboard_enabled && !text_state.focused
        && matches!(selection, BeatSelection::Card { .. })
        && !scrollbar_consumed
        && ui.ctx().input(|input| input.events.iter().any(|event| matches!(event,
            egui::Event::Key { key: egui::Key::Backspace | egui::Key::Delete, pressed: true,
                repeat: false, modifiers, .. }
                if crate::chrome::shortcuts::modifiers_are_none(*modifiers)))) {
        commands.push(Command::EditDelete);
    }
    if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) && !text_state.focused {
        text_state.clear_field_view();
    }
    interaction.paint_scrollbars(ui, geometry, palette.editor_scrollbar);
    if gesture_frame.active {
        crate::presentation::auto_scroll::paint_anchor(
            ui.painter(),
            geometry.clip,
            auto_scroll.gesture.anchor(),
            palette.page.subdued_text,
        );
    }
    crate::presentation::auto_scroll::set_active_cursor(ui.ctx(), gesture_frame.active);
    *published = Some(geometry);
    (success, commands, gesture_owns_input)
}
