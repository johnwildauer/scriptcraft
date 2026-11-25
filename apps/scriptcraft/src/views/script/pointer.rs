//! Script View pointer hit testing and selection activation.

use super::{page_policy::screen_y_to_document, ScriptCraftApp};
use crate::chrome::status_bar::PresentedPage;
use crate::commands::CommandInvocation;
use crate::presentation::geometry as presentation_geometry;
use eframe::egui;
use scriptcraft_commands::{
    Command, CommandOrigin, EditorSurface, PointerPhase, ScriptNoteIdValue,
    ScriptNotePresentValue,
};
use scriptcraft_layout::{InteractionQueryDiagnostics, ScreenplayLayout};
use scriptcraft_model::formatting::FixedPoint;
use scriptcraft_session::editing::Position;
use std::time::Instant;

use super::note_overlay::{marker_at, NoteMarkerGeometry};

impl ScriptCraftApp {
    fn caret_from_pointer(
        &mut self,
        layout: &ScreenplayLayout,
        page: usize,
        presented_page: PresentedPage,
        scale: f32,
        pointer: egui::Pos2,
        projections: Option<&[Option<Vec<Option<presentation_geometry::PresentationLine<'_>>>>]>,
    ) -> Option<Position> {
        let local_x = FixedPoint::from_points_f64(f64::from(
            (pointer.x - presented_page.rect.left()) / scale,
        ))?;
        let local_y = FixedPoint::from_points_f64(f64::from(screen_y_to_document(
            presented_page,
            pointer.y,
            scale,
        )))?;
        let started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let result = layout
            .nearest_line_location_fixed_with_diagnostics(
                page,
                local_x,
                local_y,
                &mut query_diagnostics,
            )
            .and_then(|located| {
                let line_index = layout
                    .pages
                    .get(located.page_index)?
                    .lines
                    .iter()
                    .position(|line| std::ptr::eq(line, located.line));
                let projected = line_index
                    .and_then(|index| {
                        projections
                            .and_then(|pages| pages.get(located.page_index))
                            .and_then(Option::as_ref)
                            .and_then(|lines| lines.get(index))
                    })
                    .and_then(Option::as_ref);
                let fallback_projection = projected
                    .is_none()
                    .then(|| presentation_geometry::project_line(located.line, scale).ok());
                let projected =
                    projected.or_else(|| fallback_projection.as_ref().and_then(Option::as_ref));
                if let Some(projected) = projected {
                    let screen_x = pointer.x - presented_page.rect.left();
                    Some(presentation_geometry::projected_caret_for_x(
                        projected, screen_x,
                    ))
                } else {
                    None
                }
            })
            .map(|caret| Position {
                element: caret.element,
                offset: caret.offset,
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
            .saturating_add(started.elapsed().as_nanos());
        result
    }

    fn pointer_surface_page(
        layout: &ScreenplayLayout,
        pages: &[PresentedPage],
        pointer: egui::Pos2,
        surface: EditorSurface,
    ) -> Option<usize> {
        pages
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                layout.pages[*index].title_page == (surface == EditorSurface::TitlePage)
            })
            .min_by_key(|(index, page)| {
                let distance = if pointer.y < page.rect.top() {
                    page.rect.top() - pointer.y
                } else if pointer.y > page.rect.bottom() {
                    pointer.y - page.rect.bottom()
                } else {
                    0.0
                };
                (distance.to_bits(), *index)
            })
            .map(|(index, _)| index)
    }

    pub(super) fn handle_pointer_response(
        &mut self,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
        response: &egui::Response,
        layout: &ScreenplayLayout,
        presented_pages: &[PresentedPage],
        scale: f32,
        projections: &[Option<Vec<Option<presentation_geometry::PresentationLine<'_>>>>],
        surface: EditorSurface,
        note_markers: &[NoteMarkerGeometry],
    ) {
        match surface {
            EditorSurface::Body => self.editor_surface_id = Some(editor_id),
            EditorSurface::TitlePage => self.title_editor_surface_id = Some(editor_id),
        }
        self.editor_surface_rect = Some(response.rect);
        let surface_focused = response.has_focus();
        if surface_focused != self.surface_focused {
            self.update_caret_blink_focus(ui.ctx(), surface_focused);
            self.request_repaint(ui.ctx());
        }
        self.surface_focused = surface_focused;
        if surface_focused {
            self.set_active_editor_surface(surface);
        } else {
            self.reconcile_body_active_group();
        }
        let pointer = response.interact_pointer_pos();
        let marker_hit = (surface == EditorSurface::Body
            && layout.purpose == scriptcraft_layout::LayoutPurpose::Editor)
            .then(|| pointer.and_then(|position| marker_at(note_markers, position)))
            .flatten();
        if marker_hit.is_some() {
            if response.secondary_clicked() {
                // A marker's secondary click is inert; it must not become a note
                // creation gesture or replace the screenplay range.
                return;
            }
            if response.clicked() || response.double_clicked() || response.triple_clicked() {
                let note = marker_hit
                    .and_then(|marker| marker.note_ids.first())
                    .cloned();
                if let Some(note) = note {
                    if let Some(marker) = marker_hit {
                        self.script_note_workspace.marker_anchor = Some(
                            crate::widgets::script_note::state::MarkerAnchor {
                                line_key: (marker.line_key.page_index, marker.line_key.line_index),
                                baseline: marker.baseline,
                                marker_left: marker.marker_left,
                                marker_right: marker.marker_right,
                                baseline_visible: marker.baseline_visible,
                            },
                        );
                    }
                    self.dispatch_command(
                        CommandInvocation::new(
                            Command::ScriptNotePresent {
                                presentation: ScriptNotePresentValue {
                                    note: ScriptNoteIdValue::from_core(note),
                                    move_caret: false,
                                    focus_body: false,
                                },
                            },
                            CommandOrigin::Pointer,
                        ),
                        ui.ctx(),
                    );
                }
                return;
            }
            if response.drag_started() || response.dragged() {
                return;
            }
        }
        if self.script_note_workspace.open_note.is_some()
            && pointer.is_some_and(|position| {
                presented_pages
                    .iter()
                    .enumerate()
                    .any(|(page_index, page)| {
                        !layout.pages[page_index].title_page && page.rect.contains(position)
                    })
            })
            && (response.clicked() || response.secondary_clicked())
        {
            self.dispatch_command(
                CommandInvocation::new(Command::ScriptNoteDismiss, CommandOrigin::Pointer),
                ui.ctx(),
            );
        }
        if surface == EditorSurface::Body && response.secondary_clicked() {
            self.native_input_diagnostics.pointer_activations = self
                .native_input_diagnostics
                .pointer_activations
                .saturating_add(1);
            self.set_active_editor_surface(EditorSurface::Body);
            self.dispatch_command(
                CommandInvocation::new(Command::EditorFocus, CommandOrigin::Pointer),
                ui.ctx(),
            );
            if let Some(pointer) = pointer {
                if let Some(page_index) = Self::pointer_surface_page(
                    layout,
                    presented_pages,
                    pointer,
                    EditorSurface::Body,
                ) {
                    if let Some(position) = self.caret_from_pointer(
                        layout,
                        page_index,
                        presented_pages[page_index],
                        scale,
                        pointer,
                        Some(projections),
                    ) {
                        let selection = self.session.selection();
                        let (start, end) = if
                            (selection.anchor.element, selection.anchor.offset)
                                <= (selection.focus.element, selection.focus.offset)
                        {
                            (selection.anchor, selection.focus)
                        } else {
                            (selection.focus, selection.anchor)
                        };
                        let hit = (position.element, position.offset);
                        let inside_existing_selection = selection.anchor != selection.focus
                            && (start.element, start.offset) <= hit
                            && hit <= (end.element, end.offset);
                        if !inside_existing_selection {
                            if let Ok(position) =
                                scriptcraft_commands::Position::from_core(position)
                            {
                                self.dispatch_command(
                                    CommandInvocation::new(
                                        Command::EditorSelectionPointer {
                                            position,
                                            phase: PointerPhase::Release,
                                            surface: EditorSurface::Body,
                                        },
                                        CommandOrigin::Pointer,
                                    ),
                                    ui.ctx(),
                                );
                            }
                        }
                    }
                }
            }
        } else if let Some(pointer) = pointer {
            let pointer_active = response.clicked()
                || response.double_clicked()
                || response.triple_clicked()
                || response.drag_started()
                || response.dragged()
                || (response.hovered() && ui.input(|input| input.pointer.primary_down()));
            if pointer_active {
                self.native_input_diagnostics.pointer_activations = self
                    .native_input_diagnostics
                    .pointer_activations
                    .saturating_add(1);
                self.set_active_editor_surface(surface);
                self.dispatch_command(
                    CommandInvocation::new(Command::EditorFocus, CommandOrigin::Pointer),
                    ui.ctx(),
                );
                if let Some(page_index) =
                    Self::pointer_surface_page(layout, presented_pages, pointer, surface)
                {
                    if let Some(position) = self.caret_from_pointer(
                        layout,
                        page_index,
                        presented_pages[page_index],
                        scale,
                        pointer,
                        Some(projections),
                    ) {
                        let phase = if response.drag_started() {
                            PointerPhase::Press
                        } else if response.dragged() {
                            PointerPhase::Drag
                        } else if response.triple_clicked() {
                            PointerPhase::TripleClick
                        } else if response.double_clicked() {
                            PointerPhase::DoubleClick
                        } else {
                            PointerPhase::Release
                        };
                        if let Ok(position) = scriptcraft_commands::Position::from_core(position) {
                            self.dispatch_command(
                                CommandInvocation::new(
                                    Command::EditorSelectionPointer {
                                        position,
                                        phase,
                                        surface,
                                    },
                                    CommandOrigin::Pointer,
                                ),
                                ui.ctx(),
                            );
                        }
                    }
                }
            }
        }
    }
}
