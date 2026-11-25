//! Body-editor ScriptNote marker projection, paint, and ordered navigation.

use crate::{chrome::status_bar::PresentedPage, commands::CommandInvocation};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect};
use scriptcraft_commands::{
    Command, CommandOrigin, ScriptNoteIdValue, ScriptNotePresentValue,
};
use scriptcraft_layout::ScreenplayLayout;
use scriptcraft_model::{
    body_offsets::{flat_offset_to_position, paragraph_flat_start},
    position::Position,
    Project, ScriptNote, ScriptNoteRange, StableId,
};
use std::collections::BTreeMap;

const MARKER_RIGHT_INSET: f32 = 18.0;
const MARKER_SIZE: f32 = 14.0;
const MARKER_HOVER_SIZE: f32 = 16.0;
const GROUP_COUNT_SIZE: f32 = 7.0;
const NOTE_HIGHLIGHT_ALPHA: u8 = 40;

/// Stable identity for one canonical visual baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct NoteLineKey {
    pub(super) page_index: usize,
    pub(super) line_index: usize,
}

/// Ephemeral marker geometry shared by paint and pointer handling for one frame.
#[derive(Debug, Clone)]
pub(super) struct NoteMarkerGeometry {
    pub(super) line_key: NoteLineKey,
    pub(super) glyph_rect: Rect,
    pub(super) hit_rect: Rect,
    pub(super) baseline: f32,
    pub(super) marker_left: f32,
    pub(super) marker_right: f32,
    pub(super) baseline_visible: bool,
    pub(super) hit_visible: bool,
    pub(super) color: scriptcraft_model::script_notes::ScriptNoteColorProjection,
    pub(super) note_ids: Vec<StableId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteNavigationDirection {
    Previous,
    Next,
}

/// Return note identities in semantic navigation order, independent of screen geometry.
pub(crate) fn ordered_note_ids(project: &Project) -> Vec<StableId> {
    let mut ordered = project
        .script_notes()
        .iter()
        .enumerate()
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(source_index, note)| {
        (
            note.range.start(),
            note.range.end(),
            *source_index,
        )
    });
    ordered
        .into_iter()
        .map(|(_, note)| note.id.clone())
        .collect()
}

fn line_for_position(layout: &ScreenplayLayout, position: Position) -> Option<NoteLineKey> {
    for (page_index, page) in layout.pages.iter().enumerate() {
        if page.title_page {
            continue;
        }
        for (line_index, line) in page.lines.iter().enumerate() {
            if line.element == position.element {
                return Some(NoteLineKey {
                    page_index,
                    line_index,
                });
            }
        }
    }
    None
}

/// Publish current marker anchors for all notes; hit geometry remains visible-only.
pub(super) fn project_visible_markers(
    layout: &ScreenplayLayout,
    project: &Project,
    pages: &[PresentedPage],
    scale: f32,
    editor_clip: Rect,
) -> Vec<NoteMarkerGeometry> {
    let mut grouped = BTreeMap::<NoteLineKey, Vec<&ScriptNote>>::new();
    for note_id in ordered_note_ids(project) {
        let Some(note) = project.script_notes().get(&note_id) else {
            continue;
        };
        let Some(position) = flat_offset_to_position(project, note.range.start()) else {
            // Invalid imported ranges are model/import defects; never guess a line.
            continue;
        };
        let Some(key) = line_for_position(layout, position) else {
            continue;
        };
        grouped.entry(key).or_default().push(note);
    }

    grouped
        .into_iter()
        .filter_map(|(line_key, notes)| {
            let presented_page = *pages.get(line_key.page_index)?;
            let line = layout
                .pages
                .get(line_key.page_index)?
                .lines
                .get(line_key.line_index)?;
            let baseline = crate::views::script::page_policy::document_y_to_screen(
                presented_page,
                line.baseline_y().to_points_f32(),
                scale,
            );
            let right = presented_page.rect.right() - MARKER_RIGHT_INSET * scale;
            let hit_size = MARKER_HOVER_SIZE * scale;
            let marker_hit_rect = Rect::from_min_max(
                Pos2::new(right - hit_size, baseline - hit_size),
                Pos2::new(right, baseline),
            );
            let hit_rect = marker_hit_rect.intersect(editor_clip);
            let glyph_size = MARKER_SIZE * scale;
            let glyph_rect = Rect::from_min_max(
                Pos2::new(right - glyph_size, baseline - glyph_size),
                Pos2::new(right, baseline),
            );
            let first = notes.first()?;
            Some(NoteMarkerGeometry {
                line_key,
                glyph_rect,
                hit_rect,
                baseline,
                marker_left: marker_hit_rect.left(),
                marker_right: right,
                baseline_visible: presented_page.rect.intersects(editor_clip)
                    && baseline >= editor_clip.top()
                    && baseline <= editor_clip.bottom(),
                hit_visible: hit_rect.is_positive(),
                color: first.color_projection(),
                note_ids: notes.into_iter().map(|note| note.id.clone()).collect(),
            })
        })
        .collect()
}

pub(super) fn hovered_marker(
    markers: &[NoteMarkerGeometry],
    pointer: Option<Pos2>,
) -> Option<NoteLineKey> {
    let pointer = pointer?;
    markers
        .iter()
        .find(|marker| marker.hit_visible && marker.hit_rect.contains(pointer))
        .map(|marker| marker.line_key)
}

pub(super) fn marker_at(
    markers: &[NoteMarkerGeometry],
    pointer: Pos2,
) -> Option<&NoteMarkerGeometry> {
    markers
        .iter()
        .find(|marker| marker.hit_visible && marker.hit_rect.contains(pointer))
}

pub(super) fn paint_markers(
    painter: &Painter,
    page_rect: Rect,
    scale: f32,
    page_paper: Color32,
    page_text: Color32,
    markers: &[NoteMarkerGeometry],
    hovered: Option<NoteLineKey>,
) {
    let marker_painter = painter.with_clip_rect(painter.clip_rect().intersect(page_rect));
    for marker in markers {
        if !marker.hit_visible {
            continue;
        }
        let is_hovered = hovered == Some(marker.line_key);
        let size = if is_hovered {
            MARKER_HOVER_SIZE * scale
        } else {
            MARKER_SIZE * scale
        };
        let right = marker.marker_right;
        let icon = egui_material_icons::icons::ICON_CHAT_BUBBLE.filled();
        let font = FontId::new(size, icon.font_family());
        let color = crate::widgets::script_note::properties::resolve_color(marker.color, page_text).color;
        let galley = marker_painter.layout_no_wrap(icon.codepoint.to_owned(), font, color);
        let baseline_offset = size * 0.8;
        let origin: Pos2 = Pos2::new(right - galley.size().x, marker.baseline - baseline_offset);
        let icon_center = origin + galley.size() * 0.5;
        marker_painter.galley(origin, galley, color);

        if marker.note_ids.len() > 1 {
            let count = marker.note_ids.len().to_string();
            // let hover_scale = if is_hovered { 
            //     MARKER_HOVER_SIZE/MARKER_SIZE
            // } else {
            //     1.0
            // };
            marker_painter.text(
                icon_center,
                Align2::CENTER_CENTER,
                count,
                FontId::new(
                    GROUP_COUNT_SIZE * scale,
                    FontFamily::Name("Courier Prime".into()),
                ),
                page_paper,
            );
        }
    }
}

/// Resolve one note's half-open flattened range to display boundaries in a source span.
pub(super) fn highlight_display_range(
    project: &Project,
    range: ScriptNoteRange,
    span: &scriptcraft_layout::PositionedStyledSpan,
) -> Option<(usize, usize)> {
    if range.start() >= range.end() {
        return None;
    }
    let paragraph = project.elements.get(span.source.element)?;
    let paragraph_start = paragraph_flat_start(project, span.source.element)?;
    let mut display_start = None;
    let mut display_end = None;
    for segment in span.display_source_map.iter() {
        if segment.source_start >= segment.source_end
            || segment.source_end > paragraph.text.len()
            || !paragraph.text.is_char_boundary(segment.source_start)
            || !paragraph.text.is_char_boundary(segment.source_end)
        {
            continue;
        }
        let source_start = paragraph_start
            .checked_add(paragraph.text[..segment.source_start].encode_utf16().count())?;
        let source_end = paragraph_start
            .checked_add(paragraph.text[..segment.source_end].encode_utf16().count())?;
        if range.start() < source_end && range.end() > source_start {
            display_start = Some(display_start.map_or(segment.display_start, |current: usize| {
                current.min(segment.display_start)
            }));
            display_end = Some(display_end.map_or(segment.display_end, |current: usize| {
                current.max(segment.display_end)
            }));
        }
    }
    display_start.zip(display_end)
}

pub(super) fn highlight_color(
    projection: scriptcraft_model::script_notes::ScriptNoteColorProjection,
    page_text: Color32,
) -> Color32 {
    let color = crate::widgets::script_note::properties::resolve_color(projection, page_text).color;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), NOTE_HIGHLIGHT_ALPHA)
}

pub(crate) fn navigation_enabled(project: &Project, open_note: Option<&StableId>) -> (bool, bool) {
    let ordered = ordered_note_ids(project);
    let Some(index) = open_note.and_then(|id| ordered.iter().position(|candidate| candidate == id)) else {
        return (false, false);
    };
    (index > 0, index + 1 < ordered.len())
}

pub(crate) fn navigate(
    app: &mut super::ScriptCraftApp,
    direction: NoteNavigationDirection,
    context: &egui::Context,
) -> bool {
    let ordered = ordered_note_ids(app.session.project());
    let Some(current) = app.script_note_workspace.open_note.as_ref() else {
        return false;
    };
    let Some(index) = ordered.iter().position(|candidate| candidate == current) else {
        return false;
    };
    let target_index = match direction {
        NoteNavigationDirection::Previous => index.checked_sub(1),
        NoteNavigationDirection::Next => index.checked_add(1).filter(|next| *next < ordered.len()),
    };
    let Some(target) = target_index.and_then(|index| ordered.get(index)) else {
        return false;
    };
    let command = Command::ScriptNotePresent {
        presentation: ScriptNotePresentValue {
            note: ScriptNoteIdValue::from_core(target.clone()),
            move_caret: true,
            focus_body: false,
        },
    };
    let _ = app.dispatch_command(
        CommandInvocation::new(command, CommandOrigin::InternalFollowUp),
        context,
    );
    true
}
