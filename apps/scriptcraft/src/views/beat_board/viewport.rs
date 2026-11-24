use super::{cache::BeatLayoutCache, constants, geometry::{rect_visible, ViewportGeometry}, paint, pointer::BeatInteraction, text_state::BeatTextState};
use crate::{chrome::theme::ScriptWorkspacePalette, presentation::document_font::PreparedDocumentFonts};
use eframe::egui::{self, Pos2};
use scriptcraft_fdx::BeatBoardViewState;
use scriptcraft_layout::{BeatCardMetrics, LayoutEnvironment};
use scriptcraft_model::BeatBoard;
use scriptcraft_session::editing::BeatSelection;
use scriptcraft_commands::Command;

pub(crate) fn show(ui: &mut egui::Ui, board: &BeatBoard, view: &mut BeatBoardViewState, published: &mut Option<ViewportGeometry>, palette: ScriptWorkspacePalette, cache: &mut BeatLayoutCache, prepared: Option<(&LayoutEnvironment, &PreparedDocumentFonts)>, selection: &BeatSelection, token: u64, interaction: &mut BeatInteraction, text_state: &mut BeatTextState, enabled: bool) -> (bool, Vec<Command>) {
    let mut success = true;
    cache.retain_board(board);
    let (rect, _) = ui.allocate_exact_size(ui.available_size().max(egui::Vec2::ZERO), egui::Sense::hover());
    let response = ui.interact(rect, ui.id().with(constants::VIEWPORT_ID), egui::Sense::click_and_drag());
    text_state.viewport_id = Some(response.id);
    let pointer = ui.ctx().input(|input| input.pointer.hover_pos());
    let geometry = ViewportGeometry::new(rect, ui.clip_rect(), board, *view, pointer);
    let spatial_command = interaction.input(ui, &response, geometry, board, selection, token, enabled);
    let preview = interaction.preview_card(board, geometry, ui.ctx().input(|i| i.pointer.interact_pos()));
    *view = geometry.view;
    let mut commands = spatial_command.into_iter().collect::<Vec<_>>();
    if commands.iter().any(|command| matches!(command, Command::BeatBoardCardActivate { .. } | Command::BeatBoardCardGeometrySet { .. }
        | Command::BeatBoardSelectionSet { selection: scriptcraft_commands::BeatSelectionValue::Card { .. } | scriptcraft_commands::BeatSelectionValue::None })) {
        if let Some(id) = text_state.viewport_id { ui.ctx().memory_mut(|memory| memory.surrender_focus(id)); }
        text_state.blur();
    }
    if text_state.focused && !ui.ctx().memory(|memory| memory.has_focus(response.id)) {
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
                paint::paint_card_frame(&painter, geometry, card, palette.page, selected);
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
                        paint::paint_card_frame(&painter, geometry, &card, palette.page, true);
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
    if commands.is_empty() && enabled {
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
    if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) && !text_state.focused {
        text_state.clear_field_view();
    }
    *published = Some(geometry);
    (success, commands)
}
