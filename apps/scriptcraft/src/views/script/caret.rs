//! Script View caret geometry, projection lookup, and blink presentation.

use super::{page_policy::page_local_screen_origin, ScriptCraftApp};
use crate::{
    chrome::status_bar::PresentedPage, presentation::geometry as presentation_geometry,
    views::script::RevealIntent,
};
use eframe::egui;
use scriptcraft_commands::EditorSurface;
use scriptcraft_layout::{Caret, InteractionQueryDiagnostics, ScreenplayLayout};
use scriptcraft_session::editing::Selection;
use std::time::Duration;

pub(super) const CARET_WIDTH: f32 = 1.0;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
pub(super) enum CaretPresentation {
    Projected(presentation_geometry::ProjectedCaretRect),
}

pub(super) fn viewport_caret_rect(
    presentation: CaretPresentation,
    page: PresentedPage,
    scale: f32,
) -> (egui::Rect, Option<egui::Rect>) {
    match presentation {
        CaretPresentation::Projected(projected) => {
            let page_origin = page_local_screen_origin(page, scale);
            let viewport_rect = projected.rect.translate(page_origin.to_vec2());
            let viewport_clip = egui::Rect::from_min_max(
                egui::pos2(
                    page_origin.x + projected.horizontal_clip_x[0],
                    page.rect.top(),
                ),
                egui::pos2(
                    page_origin.x + projected.horizontal_clip_x[1],
                    page.rect.bottom(),
                ),
            )
            .intersect(page.rect);
            (viewport_rect, Some(viewport_clip))
        }
    }
}

pub(super) fn caret_blink_id(
    editor_id: egui::Id,
    revision: u64,
    caret: Caret,
    focus_generation: u64,
) -> egui::Id {
    editor_id.with((
        "caret-blink",
        revision,
        caret.element,
        caret.offset,
        focus_generation,
    ))
}

pub(super) fn caret_blink_visible(
    ctx: &egui::Context,
    blink_id: egui::Id,
    focused: bool,
    collapsed: bool,
    reset_time: f64,
) -> bool {
    if !focused || !collapsed {
        return false;
    }

    // egui owns both the clock and animation entry. A zero-duration animation
    // makes each 500 ms phase switch discrete rather than introducing a fade.
    // The caller's identity includes revision and caret position, so changes
    // begin a distinct transient entry without model or thread state.
    let time = ctx.input(|input| input.time.max(0.0));
    let elapsed = (time - reset_time).max(0.0);
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (elapsed / interval).floor() as u64;
    let target_visible = phase.is_multiple_of(2);
    let phase_value = ctx.animate_bool_with_time(blink_id, target_visible, 0.0);
    ctx.request_repaint_after(caret_blink_repaint_delay(elapsed));
    phase_value > 0.5
}

fn caret_blink_repaint_delay(time: f64) -> Duration {
    let interval = CARET_BLINK_INTERVAL.as_secs_f64();
    let phase = (time.max(0.0) / interval).floor() as u64;
    Duration::from_secs_f64(((phase + 1) as f64 * interval - time.max(0.0)).max(0.001))
}

impl ScriptCraftApp {
    pub(super) fn update_caret_blink_focus(&mut self, ctx: &egui::Context, surface_focused: bool) {
        self.caret_blink_focus_generation = self.caret_blink_focus_generation.wrapping_add(1);
        if surface_focused {
            self.caret_blink_reset_time = ctx.input(|input| input.time.max(0.0));
        }
    }

    pub(super) fn caret_page_index(
        &mut self,
        layout: &ScreenplayLayout,
        surface: EditorSurface,
        selection: Selection,
        reveal_intent: RevealIntent,
        paint_range: Option<(usize, usize)>,
    ) -> Option<usize> {
        if reveal_intent == RevealIntent::Conditional {
            let mut query_diagnostics = InteractionQueryDiagnostics::default();
            let page = match surface {
                EditorSurface::Body => layout.page_for_position_in_range_with_diagnostics(
                    selection.focus.element,
                    selection.focus.offset,
                    None,
                    &mut query_diagnostics,
                ),
                EditorSurface::TitlePage => {
                    layout.title_page_for_position(selection.focus.element, selection.focus.offset)
                }
            };
            self.performance.indexed_interaction_queries = self
                .performance
                .indexed_interaction_queries
                .saturating_add(1);
            self.performance.interaction_index_entries_examined = self
                .performance
                .interaction_index_entries_examined
                .saturating_add(query_diagnostics.index_entries_examined);
            page
        } else if selection.anchor == selection.focus {
            paint_range.and_then(|(first, last)| {
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let page = match surface {
                    EditorSurface::Body => layout.page_for_position_in_range_with_diagnostics(
                        selection.focus.element,
                        selection.focus.offset,
                        Some((first, last)),
                        &mut query_diagnostics,
                    ),
                    EditorSurface::TitlePage => layout
                        .title_page_for_position(selection.focus.element, selection.focus.offset)
                        .filter(|page| *page >= first && *page <= last),
                };
                self.performance.indexed_interaction_queries = self
                    .performance
                    .indexed_interaction_queries
                    .saturating_add(1);
                self.performance.interaction_index_entries_examined = self
                    .performance
                    .interaction_index_entries_examined
                    .saturating_add(query_diagnostics.index_entries_examined);
                page
            })
        } else {
            None
        }
    }

    pub(super) fn resolve_caret_geometry(
        &mut self,
        layout: &ScreenplayLayout,
        surface: EditorSurface,
        selection: Selection,
        reveal_intent: RevealIntent,
        caret_page_index: Option<usize>,
        frame_projections: &[Option<Vec<Option<presentation_geometry::PresentationLine<'_>>>>],
        scale: f32,
    ) -> Option<CaretPresentation> {
        let resolve_caret = caret_page_index.is_some_and(|page_index| {
            reveal_intent == RevealIntent::Conditional
                || selection.anchor == selection.focus
        });
        if !resolve_caret {
            return None;
        }

        let caret_started = std::time::Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let caret = Caret {
            element: selection.focus.element,
            offset: selection.focus.offset,
        };
        let located = if surface == EditorSurface::Body {
            layout.line_for_caret_with_diagnostics(&caret, &mut query_diagnostics)
        } else {
            // Title's prior wrapper did not publish its internal
            // diagnostics, so preserve the established desktop count.
            layout.line_for_caret_with_diagnostics(
                &caret,
                &mut InteractionQueryDiagnostics::default(),
            )
        };
        let result = located.ok().and_then(|located| {
            let projection = frame_projections
                .get(located.page_index)
                .and_then(Option::as_ref)
                .and_then(|projections| {
                    layout.pages[located.page_index]
                        .lines
                        .iter()
                        .position(|line| std::ptr::eq(line, located.line))
                        .and_then(|index| projections.get(index))
                        .and_then(Option::as_ref)
                });
            let fallback_projection = projection
                .is_none()
                .then(|| presentation_geometry::project_line(located.line, scale).ok());
            let projection =
                projection.or_else(|| fallback_projection.as_ref().and_then(Option::as_ref));
            projection
                .and_then(|projection| {
                    presentation_geometry::projected_caret_rect(
                        projection,
                        caret.offset,
                        scale,
                        CARET_WIDTH,
                    )
                })
                .map(CaretPresentation::Projected)
        });
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        self.performance.caret_lookup_nanos = self
            .performance
            .caret_lookup_nanos
            .saturating_add(caret_started.elapsed().as_nanos());
        result
    }
}
