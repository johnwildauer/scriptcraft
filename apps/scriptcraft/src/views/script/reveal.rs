//! Conditional Script View caret reveal and reveal-intent ownership.

use super::{
    caret::{viewport_caret_rect, CaretPresentation},
    ScriptCraftApp,
};
use crate::chrome::status_bar::PresentedPage;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealIntent {
    None,
    Conditional,
}

pub(super) fn caret_needs_reveal(caret_rect: egui::Rect, visible_clip: egui::Rect) -> bool {
    !visible_clip.contains_rect(caret_rect)
}

pub(super) fn reveal_caret_if_needed(
    ui: &mut egui::Ui,
    reveal_intent: RevealIntent,
    zoom_compensation_queued: bool,
    caret_page_index: Option<usize>,
    caret_geometry: Option<CaretPresentation>,
    presented_pages: &[PresentedPage],
    scale: f32,
    editor_clip: egui::Rect,
) {
    if reveal_intent == RevealIntent::Conditional && !zoom_compensation_queued {
        if let (Some(page_index), Some(caret)) = (caret_page_index, caret_geometry) {
            let presented_page = presented_pages[page_index];
            let (caret_rect, caret_clip) = viewport_caret_rect(caret, presented_page, scale);
            let caret_is_clipped_out = caret_clip.is_some_and(|clip| !clip.intersects(caret_rect));
            if !caret_is_clipped_out && caret_needs_reveal(caret_rect, editor_clip) {
                // `None` asks egui for nearest/minimal scrolling;
                // centering would override the writer's viewport.
                ui.scroll_to_rect(caret_rect, None);
            }
        }
    }
}

impl ScriptCraftApp {
    pub(super) fn reveal_intent_after_frame(&mut self, zoom_compensation_queued: bool) {
        if !zoom_compensation_queued {
            self.reveal_intent = RevealIntent::None;
        }
    }
}
