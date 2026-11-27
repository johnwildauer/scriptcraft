//! Primary-click intents for visible read-only scene blocks.

use eframe::egui::{self, Pos2, Rect, Sense};
use scriptcraft_model::{Project, StableId};
use scriptcraft_layout::{SceneProjection, StoryMapProjection};

use super::{
    constants::STORY_MAP_BEAT_TAIL_TARGET,
    geometry::{placed_beat_rect, StripGeometry, TimelineGeometry},
    script_track::visible_scene_anchor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeatPointerMode {
    Move,
    TailResize,
}

/// A hit on a visible, clipped, resolved placed Beat. Tail mode takes priority
/// over ordinary movement in the final five visible points.
pub(crate) fn hit_placed_beat(
    ui: &mut egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    project: &Project,
    point: Pos2,
) -> Option<(StableId, BeatPointerMode)> {
    let clip = beat_clip(strip, ui.clip_rect());
    for (card_id, rect) in visible_placed_beats(strip, timeline, project, clip) {
        if !ui.is_rect_visible(rect) { continue; }
        let mode = if point.x >= (rect.right() - STORY_MAP_BEAT_TAIL_TARGET).max(rect.left()) {
            BeatPointerMode::TailResize
        } else {
            BeatPointerMode::Move
        };
        let response = ui.interact(
            rect,
            ui.id().with(("story-map-beat-placement", card_id.as_str())),
            Sense::click_and_drag(),
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(match mode {
                BeatPointerMode::Move => egui::CursorIcon::Grab,
                BeatPointerMode::TailResize => egui::CursorIcon::ResizeHorizontal,
            });
        }
        if rect.contains(point) {
            return Some((card_id, mode));
        }
    }
    None
}

/// Geometry-only list shared by registered primary hit testing and middle-button
/// empty-space eligibility. It never creates egui responses.
fn visible_placed_beats(
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    project: &Project,
    clip: Rect,
) -> Vec<(StableId, Rect)> {
    if !clip.is_positive() {
        return Vec::new();
    }
    let story_map = project.story_map();
    let mut visible = Vec::new();
    for card in project.beat_board().cards.iter().rev() {
        let Some(placement) = card.story_map_placement.as_ref() else { continue };
        let mut matching = story_map.lanes.iter().enumerate()
            .filter(|(_, lane)| lane.uuid == placement.lane_uuid);
        let Some((lane_index, _)) = matching.next() else { continue };
        if matching.next().is_some() { continue; }
        let Some(rect) = placed_beat_rect(
            strip,
            lane_index,
            placement.start_eighths,
            placement.length_eighths,
            story_map.target_pages,
            timeline,
            clip,
        ) else { continue };
        visible.push((card.id.clone(), rect));
    }
    visible
}

fn beat_clip(strip: &StripGeometry, ui_clip: Rect) -> Rect {
    let mut clip = strip.timeline_viewport.intersect(strip.rect).intersect(ui_clip);
    clip.max.y = clip.max.y.min(strip.resize_band.top());
    clip
}

/// Pure geometry predicate for visible, clipped placed Beat blocks.
pub(crate) fn pointer_over_placed_beat(
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    project: &Project,
    clip: Rect,
    point: Pos2,
) -> bool {
    finite_pos(point)
        && visible_placed_beats(strip, timeline, project, beat_clip(strip, clip))
            .iter()
            .any(|(_, rect)| rect.contains(point))
}

/// Pure geometry predicate for visible scene blocks. Missing projection is
/// handled by the caller as ineligible Script-row space (fail closed).
pub(crate) fn pointer_over_scene_geometry(
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    snapshot: &StoryMapProjection,
    clip: Rect,
    point: Pos2,
) -> bool {
    let clip = strip
        .script_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(clip);
    clip.is_positive()
        && finite_pos(point)
        && snapshot.scenes.iter().any(|scene| {
            visible_scene_anchor(strip, timeline, scene, clip)
                .is_some_and(|anchor| anchor.contains(point))
        })
}

/// Empty visible lane/ruler/Script geometry eligible for the Outline adapter.
/// Controls, labels, the scrollbar row, and resize band are excluded structurally.
pub(crate) fn middle_auto_scroll_eligible(
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    project: &Project,
    snapshot: Option<&StoryMapProjection>,
    clip: Rect,
    point: Pos2,
) -> bool {
    if !finite_pos(point)
        || !clip.is_positive()
        || strip.label_column.contains(point)
        || strip.scrollbar_row.contains(point)
        || strip.resize_band.contains(point)
    {
        return false;
    }
    let timeline_clip = strip.timeline_viewport.intersect(strip.rect).intersect(clip);
    let in_lane = strip.lane_rows.iter().any(|row| {
        row.intersect(timeline_clip).is_positive() && row.intersect(timeline_clip).contains(point)
    });
    let ruler = strip.ruler_row.intersect(timeline_clip);
    let script = strip.script_row.intersect(timeline_clip);
    let in_ruler = ruler.is_positive() && ruler.contains(point);
    let in_script = script.is_positive() && script.contains(point);
    if !(in_lane || in_ruler || in_script) {
        return false;
    }
    if pointer_over_placed_beat(strip, timeline, project, clip, point) {
        return false;
    }
    if in_script {
        let Some(snapshot) = snapshot else { return false };
        if pointer_over_scene_geometry(strip, timeline, snapshot, clip, point) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone)]
pub(crate) struct SceneClick {
    pub(crate) element_index: usize,
    pub(crate) owner_id: scriptcraft_model::StableId,
    pub(crate) anchor: Rect,
}

/// Registers hit targets only for positively clipped scene blocks and returns
/// a single primary-click intent. All geometry comes from the paint helper.
pub(crate) fn primary_scene_click(
    ui: &mut egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    snapshot: &StoryMapProjection,
    enabled: bool,
) -> Option<SceneClick> {
    if !enabled {
        return None;
    }
    let clip = strip
        .script_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    if !clip.is_positive() {
        return None;
    }
    for scene in &snapshot.scenes {
        let Some(anchor) = visible_scene_anchor(strip, timeline, scene, clip) else {
            continue;
        };
        let response = ui.interact(
            anchor,
            ui.id().with(("story-map-scene-preview", scene.element_index)),
            egui::Sense::click(),
        );
        if response.clicked_by(egui::PointerButton::Primary) {
            return Some(SceneClick {
                element_index: scene.element_index,
                owner_id: scene.owner_id.clone(),
                anchor,
            });
        }
    }
    None
}

/// Reuses the exact visible anchor computation when resolving an open preview.
pub(crate) fn current_anchor(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    scene: &SceneProjection,
) -> Option<Rect> {
    let clip = strip
        .script_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    visible_scene_anchor(strip, timeline, scene, clip)
}

/// Reports whether the current pointer lies over a positively clipped scene
/// block, using the same anchor geometry as painting and click registration.
pub(crate) fn pointer_over_scene(
    ui: &egui::Ui,
    strip: &StripGeometry,
    timeline: &TimelineGeometry,
    snapshot: &StoryMapProjection,
    point: egui::Pos2,
) -> bool {
    pointer_over_scene_geometry(strip, timeline, snapshot, ui.clip_rect(), point)
}

fn finite_pos(point: Pos2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}
