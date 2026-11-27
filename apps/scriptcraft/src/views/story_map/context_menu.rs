//! Whole-strip Outline Editor context popup and transient placement state.

use eframe::egui::{self, Pos2, Rect, Vec2};

use super::constants::{STORY_MAP_CONTEXT_MENU_ROW_HEIGHT, STORY_MAP_CONTEXT_MENU_WIDTH};

#[derive(Debug, Default)]
pub(crate) struct StoryMapContextMenuState {
    pub(crate) open: bool,
    anchor: Option<Pos2>,
    popup_rect: Option<Rect>,
}

impl StoryMapContextMenuState {
    pub(crate) fn open_at(&mut self, anchor: Pos2) {
        self.open = true;
        self.anchor = Some(anchor);
        self.popup_rect = None;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.anchor = None;
        self.popup_rect = None;
    }

    pub(crate) fn popup_rect(&self) -> Option<Rect> {
        self.popup_rect
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum StoryMapContextMenuAction {
    HideOutlineEditor,
    ChangeTargetScriptLength { anchor: Pos2 },
}

pub(crate) fn show(
    state: &mut StoryMapContextMenuState,
    ctx: &egui::Context,
    root_bounds: Rect,
) -> Option<StoryMapContextMenuAction> {
    if !state.open {
        return None;
    }
    let (outside_primary, pointer, escape) = ctx.input(|input| {
        (
            input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.interact_pos(),
            input.key_pressed(egui::Key::Escape),
        )
    });
    if escape {
        ctx.input_mut(|input| {
            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        });
        state.close();
        return None;
    }
    if outside_primary
        && pointer.is_some_and(|point| !state.popup_rect.is_some_and(|rect| rect.contains(point)))
    {
        state.close();
        return None;
    }

    let anchor = state.anchor.unwrap_or(root_bounds.min);
    let row_size = Vec2::new(STORY_MAP_CONTEXT_MENU_WIDTH, STORY_MAP_CONTEXT_MENU_ROW_HEIGHT * 2.0);
    let margin = ctx.style().spacing.menu_margin;
    let size = row_size + Vec2::new(
        f32::from(margin.left) + f32::from(margin.right),
        f32::from(margin.top) + f32::from(margin.bottom),
    );
    let position = clamp_origin(anchor, size, root_bounds);
    let mut action = None;
    let area = egui::Area::new(egui::Id::new("story-map-context-menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(STORY_MAP_CONTEXT_MENU_WIDTH);
                ui.set_max_width(STORY_MAP_CONTEXT_MENU_WIDTH);
                for (label, selected) in [
                    ("Hide Outline Editor", StoryMapContextMenuAction::HideOutlineEditor),
                    ("Change Target Script Length", StoryMapContextMenuAction::ChangeTargetScriptLength { anchor }),
                ] {
                    let response = ui.add_sized(
                        Vec2::new(STORY_MAP_CONTEXT_MENU_WIDTH, STORY_MAP_CONTEXT_MENU_ROW_HEIGHT),
                        egui::Button::new(label).frame(false),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                    });
                    if response.clicked() {
                        action = Some(selected);
                    }
                }
            });
        });
    state.popup_rect = Some(area.response.rect);
    if action.is_some() {
        state.close();
    }
    action
}

fn clamp_origin(anchor: Pos2, size: Vec2, bounds: Rect) -> Pos2 {
    let maximum = bounds.max - size;
    Pos2::new(
        anchor.x.clamp(bounds.left(), maximum.x.max(bounds.left())),
        anchor.y.clamp(bounds.top(), maximum.y.max(bounds.top())),
    )
}
