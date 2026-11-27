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
    let mut clip = strip.timeline_viewport.intersect(strip.rect).intersect(ui.clip_rect());
    clip.max.y = clip.max.y.min(strip.resize_band.top());
    if !clip.is_positive() { return None; }
    let story_map = project.story_map();
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
        if !ui.is_rect_visible(rect) { continue; }
        let mode = if point.x >= (rect.right() - STORY_MAP_BEAT_TAIL_TARGET).max(rect.left()) {
            BeatPointerMode::TailResize
        } else {
            BeatPointerMode::Move
        };
        let response = ui.interact(
            rect,
            ui.id().with(("story-map-beat-placement", card.id.as_str())),
            Sense::click_and_drag(),
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(match mode {
                BeatPointerMode::Move => egui::CursorIcon::Grab,
                BeatPointerMode::TailResize => egui::CursorIcon::ResizeHorizontal,
            });
        }
        if rect.contains(point) {
            return Some((card.id.clone(), mode));
        }
    }
    None
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
    let clip = strip
        .script_row
        .intersect(strip.timeline_viewport)
        .intersect(strip.rect)
        .intersect(ui.clip_rect());
    clip.is_positive()
        && snapshot.scenes.iter().any(|scene| {
            visible_scene_anchor(strip, timeline, scene, clip)
                .is_some_and(|anchor| anchor.contains(point))
        })
}
