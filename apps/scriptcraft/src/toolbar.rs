use super::*;

// Manual toolbar geometry controls: tune Item group width and row spacing here.
// The 46-point normal slot is the compact baseline recorded by the focused
// geometry tests; labels may still expand a slot when active font metrics need it.
pub(super) const TOOLBAR_NORMAL_ITEM_WIDTH: f32 = 46.0;
pub(super) const TOOLBAR_ELEMENTS_WIDTH: f32 = 128.0;
#[allow(dead_code)] // Retained typed spacing policy; covered by deterministic toolbar tests.
pub(super) const TOOLBAR_SPACE_WIDTH: f32 = 72.0;
pub(super) const TOOLBAR_GROUP_HORIZONTAL_PADDING: f32 = 8.0;
pub(super) const TOOLBAR_OVERFLOW_WIDTH: f32 = 32.0;
pub(super) const TOOLBAR_ROW_PADDING: f32 = 4.0;
pub(super) const TOOLBAR_ITEM_GAP: f32 = 4.0;
pub(super) const TOOLBAR_TOP_PADDING: f32 = 0.0;
// Preserve the toolbar's text/control clearance. The trailing span is part of
// the exact top-panel allocation and must not be removed to fix a viewport row.
pub(super) const TOOLBAR_BOTTOM_PADDING: f32 = 2.0;
pub(super) const TOOLBAR_ICON_HOVER_PADDING: f32 = 2.0;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportPdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarItemKind {
    Action(ToolbarAction),
    Elements,
    #[allow(dead_code)] // Supported non-flexible spacing Item, not in the default composition.
    Space,
    FlexibleSpace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ToolbarWidthOverride {
    Measured,
    Fixed(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ToolbarItem {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) icon: &'static str,
    pub(super) kind: ToolbarItemKind,
    pub(super) width_override: ToolbarWidthOverride,
}

impl ToolbarItem {
    pub(super) const fn action(
        id: &'static str,
        label: &'static str,
        icon: &'static str,
        action: ToolbarAction,
    ) -> Self {
        Self {
            id,
            label,
            icon,
            kind: ToolbarItemKind::Action(action),
            width_override: ToolbarWidthOverride::Measured,
        }
    }

    pub(super) const fn elements() -> Self {
        Self {
            id: "elements",
            label: "Elements",
            icon: "▾",
            kind: ToolbarItemKind::Elements,
            width_override: ToolbarWidthOverride::Fixed(TOOLBAR_ELEMENTS_WIDTH),
        }
    }

    pub(super) const fn flexible_space(id: &'static str) -> Self {
        Self {
            id,
            label: "Flexible Space",
            icon: "",
            kind: ToolbarItemKind::FlexibleSpace,
            width_override: ToolbarWidthOverride::Measured,
        }
    }

    #[allow(dead_code)] // Retained for typed compositions and covered by allocation tests.
    pub(super) const fn space(id: &'static str) -> Self {
        Self {
            id,
            label: "Space",
            icon: "",
            kind: ToolbarItemKind::Space,
            width_override: ToolbarWidthOverride::Fixed(TOOLBAR_SPACE_WIDTH),
        }
    }
}

pub(super) const SUPPORTED_SCREENPLAY_ROLES: [(Role, &str); 17] = [
    (Role::SceneHeading, "Scene Heading"),
    (Role::Action, "Action"),
    (Role::Character, "Character"),
    (Role::Dialogue, "Dialogue"),
    (Role::Parenthetical, "Parenthetical"),
    (Role::Transition, "Transition"),
    (Role::Note, "Note"),
    (Role::NewAct, "New Act"),
    (Role::EndOfAct, "End of Act"),
    (Role::CastList, "Cast List"),
    (Role::General, "General"),
    (Role::Shot, "Shot"),
    (Role::Sequence, "Sequence"),
    (Role::Outline1, "Outline 1"),
    (Role::Outline2, "Outline 2"),
    (Role::Outline3, "Outline 3"),
    (Role::Summary, "Summary"),
];

pub(super) fn screenplay_role_label(role: Role) -> &'static str {
    SUPPORTED_SCREENPLAY_ROLES
        .iter()
        .find_map(|(supported, label)| (*supported == role).then_some(*label))
        .unwrap_or("Unknown")
}

pub(super) fn default_toolbar_items() -> [ToolbarItem; 8] {
    [
        ToolbarItem::action("new", "New", "＋", ToolbarAction::New),
        ToolbarItem::action("open", "Open", "↥", ToolbarAction::Open),
        ToolbarItem::action("save", "Save", "▣", ToolbarAction::Save),
        ToolbarItem::action("save-as", "Save As", "⇩", ToolbarAction::SaveAs),
        ToolbarItem::flexible_space("flexible-space-before-elements"),
        ToolbarItem::elements(),
        ToolbarItem::flexible_space("flexible-space-after-elements"),
        ToolbarItem::action("export-pdf", "Export PDF", "⇱", ToolbarAction::ExportPdf),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ToolbarAllocation {
    pub(super) visible: Vec<(ToolbarItem, f32)>,
    pub(super) overflow: Vec<ToolbarItem>,
    pub(super) overflow_width: f32,
    pub(super) row_width: f32,
}

pub(super) fn is_flexible_toolbar_space(item: ToolbarItem) -> bool {
    matches!(item.kind, ToolbarItemKind::FlexibleSpace)
}

pub(super) fn toolbar_item_width<F>(item: ToolbarItem, measure_label: F) -> f32
where
    F: Fn(&str) -> f32,
{
    if is_flexible_toolbar_space(item) {
        return 0.0;
    }
    match item.width_override {
        ToolbarWidthOverride::Fixed(width) => width,
        ToolbarWidthOverride::Measured => {
            let label_width = measure_label(item.label) + TOOLBAR_GROUP_HORIZONTAL_PADDING;
            TOOLBAR_NORMAL_ITEM_WIDTH.max(label_width)
        }
    }
}

pub(super) fn toolbar_label_width(ui: &egui::Ui, label: &str) -> f32 {
    let font_id = ui
        .style()
        .text_styles
        .get(&TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| FontId::proportional(14.0));
    ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(label.to_owned(), font_id, Color32::TRANSPARENT)
            .size()
            .x
    })
}

pub(super) fn toolbar_content_width(widths: &[(ToolbarItem, f32)], overflow_width: f32) -> f32 {
    let child_count = widths.len() as f32 + f32::from(overflow_width > 0.0);
    widths.iter().map(|(_, width)| width).sum::<f32>()
        + TOOLBAR_ROW_PADDING * 2.0
        + (child_count - 1.0).max(0.0) * TOOLBAR_ITEM_GAP
        + overflow_width
}

pub(super) fn toolbar_row_width(widths: &[(ToolbarItem, f32)], overflow_width: f32) -> f32 {
    toolbar_content_width(widths, overflow_width)
}

pub(super) fn toolbar_actual_available_width(
    content_rect: egui::Rect,
    clip_rect: egui::Rect,
    row_left: f32,
) -> f32 {
    let actual_rect = content_rect.intersect(clip_rect);
    (actual_rect.right() - row_left).max(0.0)
}

#[cfg(test)]
pub(super) fn toolbar_row_right_edge(row_left: f32, allocation: &ToolbarAllocation) -> f32 {
    row_left + allocation.row_width
}

#[cfg(test)]
pub(super) fn toolbar_rect_contains(outer: egui::Rect, inner: egui::Rect) -> bool {
    outer.left() <= inner.left()
        && inner.right() <= outer.right()
        && outer.top() <= inner.top()
        && inner.bottom() <= outer.bottom()
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct ToolbarComboBoundaryLedger {
    pub(super) combo_response: egui::Rect,
    pub(super) decorative_label_response: egui::Rect,
    pub(super) group_response: egui::Rect,
}

#[cfg(test)]
pub(super) fn toolbar_combo_boundary_ledger(width: f32) -> ToolbarComboBoundaryLedger {
    let context = egui::Context::default();
    let mut ledger = ToolbarComboBoundaryLedger {
        combo_response: egui::Rect::NOTHING,
        decorative_label_response: egui::Rect::NOTHING,
        group_response: egui::Rect::NOTHING,
    };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1200.0, 200.0));
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let group = ui.allocate_ui_with_layout(
                    Vec2::new(width, ui.spacing().interact_size.y * 2.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_width(width);
                        ledger.combo_response = egui::ComboBox::from_id_salt("ledger-elements")
                            .width(width)
                            .wrap_mode(TextWrapMode::Truncate)
                            .selected_text("Scene Heading")
                            .show_ui(ui, |_ui| {})
                            .response
                            .rect;
                        let (_, label_rect) = ui.allocate_space(ui.spacing().interact_size);
                        ledger.decorative_label_response = label_rect;
                    },
                );
                ledger.group_response = group.response.rect;
            });
        },
    );
    ledger
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ElementsCursorLedger {
    pub(super) outer_group: egui::Rect,
    pub(super) stock_combo: egui::Rect,
    pub(super) closed_selected_galley: egui::Rect,
    pub(super) decorative_label: egui::Rect,
    pub(super) popup_row: egui::Rect,
    pub(super) group_gap: egui::Rect,
    pub(super) document: egui::Rect,
    pub(super) closed_cursor: egui::CursorIcon,
    pub(super) popup_cursor: egui::CursorIcon,
    pub(super) document_cursor: egui::CursorIcon,
}

#[cfg(test)]
pub(super) fn elements_cursor_ledger() -> ElementsCursorLedger {
    let outer_group = egui::Rect::from_min_size(egui::pos2(20.0, 10.0), Vec2::new(128.0, 48.0));
    let stock_combo = egui::Rect::from_min_size(outer_group.min, Vec2::new(128.0, 24.0));
    let closed_selected_galley =
        elements_closed_text_rect(stock_combo, Vec2::splat(4.0), 4.0, 12.0);
    let decorative_label = egui::Rect::from_min_size(
        egui::pos2(outer_group.left(), stock_combo.bottom()),
        Vec2::new(128.0, 24.0),
    );
    let popup_row = egui::Rect::from_min_size(
        egui::pos2(stock_combo.left(), outer_group.bottom() + 4.0),
        Vec2::new(128.0, 24.0),
    );
    let group_gap = egui::Rect::from_min_max(
        egui::pos2(outer_group.right(), outer_group.top()),
        egui::pos2(outer_group.right() + 4.0, outer_group.bottom()),
    );
    let document = egui::Rect::from_min_size(egui::pos2(0.0, 70.0), Vec2::new(400.0, 300.0));
    ElementsCursorLedger {
        outer_group,
        stock_combo,
        closed_selected_galley,
        decorative_label,
        popup_row,
        group_gap,
        document,
        closed_cursor: toolbar_cursor_icon(),
        popup_cursor: toolbar_cursor_icon(),
        document_cursor: document_cursor_icon(),
    }
}

pub(super) fn elements_closed_text_rect(
    control_rect: egui::Rect,
    button_padding: egui::Vec2,
    icon_spacing: f32,
    icon_width: f32,
) -> egui::Rect {
    let content_rect = control_rect.shrink2(button_padding);
    let text_right = (content_rect.right() - icon_spacing - icon_width).max(content_rect.left());
    egui::Rect::from_min_max(
        content_rect.left_top(),
        egui::pos2(text_right, content_rect.bottom()),
    )
}

pub(super) fn elements_closed_text_position(text_rect: egui::Rect, text_size: Vec2) -> egui::Pos2 {
    egui::Align2::CENTER_CENTER
        .align_size_within_rect(text_size, text_rect)
        .min
}

#[cfg(test)]
pub(super) fn toolbar_rect_ledger(
    content_rect: egui::Rect,
    clip_rect: egui::Rect,
    row_left: f32,
    allocation: &ToolbarAllocation,
) -> (egui::Rect, egui::Rect, egui::Rect, egui::Rect, egui::Rect) {
    let row_rect = egui::Rect::from_min_size(
        egui::pos2(row_left, content_rect.top()),
        Vec2::new(allocation.row_width, content_rect.height()),
    );
    let elements_index = allocation
        .visible
        .iter()
        .position(|(item, _)| matches!(item.kind, ToolbarItemKind::Elements));
    let mut child_left = row_left + TOOLBAR_ROW_PADDING;
    for (index, (_, width)) in allocation.visible.iter().copied().enumerate() {
        if index > 0 {
            child_left += TOOLBAR_ITEM_GAP;
        }
        if Some(index) == elements_index {
            break;
        }
        child_left += width;
    }
    let child_width = elements_index
        .and_then(|index| allocation.visible.get(index).map(|(_, width)| *width))
        .unwrap_or(0.0);
    let child_rect = egui::Rect::from_min_size(
        egui::pos2(child_left, content_rect.top()),
        Vec2::new(child_width, content_rect.height()),
    );
    let final_group_rect = allocation.visible.iter().enumerate().fold(
        row_left + TOOLBAR_ROW_PADDING,
        |left, (index, (_, width))| {
            let left = if index == 0 {
                left
            } else {
                left + TOOLBAR_ITEM_GAP
            };
            left + width
        },
    );
    let final_group_width = allocation
        .visible
        .last()
        .map(|(_, width)| *width)
        .unwrap_or(0.0);
    let final_label_rect = egui::Rect::from_min_max(
        egui::pos2(final_group_rect - final_group_width, content_rect.top()),
        egui::pos2(final_group_rect, content_rect.bottom()),
    );
    (
        content_rect,
        clip_rect,
        row_rect,
        child_rect,
        final_label_rect,
    )
}

/// Allocate the transient toolbar without mutating project state.
/// Spacing items have one fixed slot; rendered items use their complete,
/// measured label-aware group width. Flexible spaces share only the remaining
/// width once every normal item fits.
pub(super) fn allocate_toolbar<F>(
    items: &[ToolbarItem],
    available_width: f32,
    measure_label: F,
) -> ToolbarAllocation
where
    F: Fn(&str) -> f32 + Copy,
{
    let fixed_width = items
        .iter()
        .copied()
        .map(|item| toolbar_item_width(item, measure_label))
        .sum::<f32>();
    let full_fixed_width = fixed_width
        + TOOLBAR_ROW_PADDING * 2.0
        + (items.len().saturating_sub(1) as f32) * TOOLBAR_ITEM_GAP;
    let flexible_count = items
        .iter()
        .filter(|item| matches!(item.kind, ToolbarItemKind::FlexibleSpace))
        .count();

    if full_fixed_width <= available_width {
        let flexible_width = if flexible_count == 0 {
            0.0
        } else {
            ((available_width - full_fixed_width) / flexible_count as f32).max(0.0)
        };
        return ToolbarAllocation {
            visible: items
                .iter()
                .map(|item| {
                    let width = if is_flexible_toolbar_space(*item) {
                        flexible_width
                    } else {
                        toolbar_item_width(*item, measure_label)
                    };
                    (*item, width)
                })
                .collect(),
            overflow: Vec::new(),
            overflow_width: 0.0,
            row_width: available_width,
        };
    }

    let mut visible = Vec::new();
    let mut overflow = Vec::new();
    let mut fitting = true;
    for item in items.iter().copied() {
        if is_flexible_toolbar_space(item) {
            continue;
        }
        if matches!(item.kind, ToolbarItemKind::Space) {
            let mut candidate = visible.clone();
            candidate.push((item, toolbar_item_width(item, measure_label)));
            if fitting
                && toolbar_content_width(&candidate, TOOLBAR_OVERFLOW_WIDTH) <= available_width
            {
                visible.push((item, toolbar_item_width(item, measure_label)));
            }
            continue;
        }
        let mut candidate = visible.clone();
        candidate.push((item, toolbar_item_width(item, measure_label)));
        if fitting && toolbar_content_width(&candidate, TOOLBAR_OVERFLOW_WIDTH) <= available_width {
            visible.push((item, toolbar_item_width(item, measure_label)));
        } else {
            fitting = false;
            overflow.push(item);
        }
    }

    let row_width = toolbar_row_width(&visible, TOOLBAR_OVERFLOW_WIDTH);
    ToolbarAllocation {
        visible,
        overflow,
        overflow_width: TOOLBAR_OVERFLOW_WIDTH,
        row_width,
    }
}

impl ScriptCraftApp {
    pub(super) fn dispatch_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::New => {
                match Project::new("demo-project", "Untitled Screenplay", "Writer")
                    .map_err(|error| format!("new project failed: {error:?}"))
                    .and_then(|project| {
                        EditingSession::new(project)
                            .map_err(|error| format!("session failed: {error:?}"))
                    }) {
                    Ok(session) => {
                        self.session = session;
                        self.page_geometry.invalidate();
                        self.surface_focused = false;
                        self.native_path = None;
                        self.status = "New in-memory project".into();
                        self.clear_error();
                    }
                    Err(error) => self.show_error(error),
                }
            }
            ToolbarAction::Open => self.open_native(),
            ToolbarAction::Save => self.save_native(),
            ToolbarAction::SaveAs => self.save_as_native(),
            ToolbarAction::ExportPdf => self.export_pdf(),
        }
    }

    pub(super) fn toolbar_action_button(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItem,
        width: f32,
        presentation: ToolbarPresentationMetrics,
    ) {
        let (id, rect) = ui.allocate_space(Vec2::new(width, ui.spacing().interact_size.y * 2.0));
        let icon_control_size = Vec2::splat(presentation.icon_control_size);
        let icon_control_rect = toolbar_icon_control_rect(rect, icon_control_size);
        let icon_rect = toolbar_icon_hover_rect(icon_control_rect, TOOLBAR_ICON_HOVER_PADDING);
        let response = ui
            .interact(icon_rect, id.with("icon"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::Default);
        let fill = toolbar_group_fill(ui.style(), response.hovered());
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(icon_rect, 2.0, fill);
        }
        let body_font = presentation.font_id();
        let label_size = ui.fonts(|fonts| {
            fonts
                .layout_no_wrap(
                    item.label.to_owned(),
                    body_font.clone(),
                    Color32::TRANSPARENT,
                )
                .size()
        });
        let label_center = egui::pos2(
            rect.center().x,
            icon_control_rect.max.y + ui.spacing().item_spacing.y + label_size.y / 2.0,
        );
        ui.painter().text(
            icon_control_rect.center(),
            egui::Align2::CENTER_CENTER,
            item.icon,
            body_font.clone(),
            ui.visuals().text_color(),
        );
        ui.painter().text(
            label_center,
            egui::Align2::CENTER_CENTER,
            item.label,
            body_font,
            ui.visuals().text_color(),
        );
        if response.clicked() {
            if let ToolbarItemKind::Action(action) = item.kind {
                self.dispatch_toolbar_action(action);
            }
        }
    }

    pub(super) fn focused_role(&self) -> Option<Role> {
        self.session
            .project()
            .elements
            .get(self.session.selection().focus.element)
            .map(|element| element.role)
    }

    pub(super) fn change_role_from_toolbar(&mut self, role: Role) {
        let result = self.session.change_role(role);
        self.mutate(result, "Changed focused element role", RevealIntent::None);
    }

    pub(super) fn elements_toolbar_item(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItem,
        width: f32,
        presentation: ToolbarPresentationMetrics,
    ) {
        let selected_role = self.focused_role();
        let mut next_role = selected_role.unwrap_or(Role::Action);
        let group_response = ui.allocate_ui_with_layout(
            Vec2::new(width, ui.spacing().interact_size.y * 2.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                // ComboBox has an intrinsic minimum. Constrain this child to the
                // same explicit budget used by allocate_toolbar so it cannot
                // advance the parent row past the allocated Elements slot.
                ui.set_width(width);
                let combo_result = egui::ComboBox::from_id_salt(item.id)
                    .width(width)
                    .wrap_mode(TextWrapMode::Truncate)
                    // The stock ComboBox has no independent closed-text alignment
                    // setting. Keep its button, popup, ID, keyboard handling, and
                    // arrow intact, then paint only the closed selected galley below.
                    .selected_text("")
                    .show_ui(ui, |ui| {
                        for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                            ui.selectable_value(&mut next_role, role, label);
                        }
                    });
                let popup_open = combo_result.inner.is_some();
                let combo_response = combo_result.response.on_hover_cursor(toolbar_cursor_icon());
                if !popup_open {
                    let text_rect = elements_closed_text_rect(
                        combo_response.rect,
                        ui.spacing().button_padding,
                        ui.spacing().icon_spacing,
                        ui.spacing().icon_width,
                    );
                    let galley = WidgetText::from(
                        RichText::new(screenplay_role_label(next_role))
                            .font(presentation.font_id()),
                    )
                    .into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        text_rect.width(),
                        TextStyle::Button,
                    );
                    let text_position = elements_closed_text_position(text_rect, galley.size());
                    let visuals = ui.style().interact(&combo_response);
                    ui.painter()
                        .galley(text_position, galley, visuals.text_color());
                }
                // This caption is decorative. Allocate/paint it instead of adding a
                // label response that can compete with the ComboBox cursor owner.
                let (_, label_rect) = ui.allocate_space(ui.spacing().interact_size);
                let label_font = presentation.font_id();
                ui.painter().text(
                    label_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    item.label,
                    label_font,
                    ui.visuals().text_color(),
                );
                debug_assert!(combo_response.rect.width() <= width + f32::EPSILON);
            },
        );
        // Own the label-only portion at the group boundary without making the
        // decorative label itself interactive or changing ComboBox routing.
        group_response
            .response
            .on_hover_cursor(toolbar_cursor_icon());
        if selected_role != Some(next_role) {
            self.change_role_from_toolbar(next_role);
        }
    }

    pub(super) fn toolbar_overflow_menu(&mut self, ui: &mut egui::Ui, items: &[ToolbarItem]) {
        let mut chosen_action = None;
        let mut chosen_role = None;
        ui.menu_button(">>", |ui| {
            for item in items.iter().copied() {
                match item.kind {
                    ToolbarItemKind::Action(action) => {
                        if ui.button(item.label).clicked() {
                            chosen_action = Some(action);
                            ui.close_menu();
                        }
                    }
                    ToolbarItemKind::Elements => {
                        let selected_role = self.focused_role();
                        ui.menu_button(item.label, |ui| {
                            for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                                if ui
                                    .selectable_label(selected_role == Some(role), label)
                                    .clicked()
                                {
                                    chosen_role = Some(role);
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    ToolbarItemKind::Space | ToolbarItemKind::FlexibleSpace => {}
                }
            }
        });
        if let Some(action) = chosen_action {
            self.dispatch_toolbar_action(action);
        }
        if let Some(role) = chosen_role {
            self.change_role_from_toolbar(role);
        }
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        // Use the clipped panel content edge for both allocation and rendering.
        // The logical panel width can extend past the visible viewport after
        // frame/panel and native viewport insets have been applied.
        let content_rect = ui.available_rect_before_wrap();
        let row_left = ui.cursor().left();
        let available_width =
            toolbar_actual_available_width(content_rect, ui.clip_rect(), row_left);
        let allocation = allocate_toolbar(&default_toolbar_items(), available_width, |label| {
            toolbar_label_width(ui, label)
        });
        let presentation = live_toolbar_metrics(ui);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(TOOLBAR_TOP_PADDING);
            ui.allocate_ui_with_layout(
                Vec2::new(allocation.row_width, ui.spacing().interact_size.y * 2.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.add_space(TOOLBAR_ROW_PADDING);
                    let mut first = true;
                    for (item, width) in allocation.visible.iter().copied() {
                        if !first {
                            ui.add_space(TOOLBAR_ITEM_GAP);
                        }
                        first = false;
                        ui.push_id(item.id, |ui| match item.kind {
                            ToolbarItemKind::Action(_) => {
                                self.toolbar_action_button(ui, item, width, presentation)
                            }
                            ToolbarItemKind::Elements => {
                                self.elements_toolbar_item(ui, item, width, presentation)
                            }
                            ToolbarItemKind::Space | ToolbarItemKind::FlexibleSpace => {
                                ui.add_space(width);
                            }
                        });
                    }
                    if !allocation.overflow.is_empty() {
                        if !first {
                            ui.add_space(TOOLBAR_ITEM_GAP);
                        }
                        ui.allocate_ui(
                            Vec2::new(allocation.overflow_width, ui.spacing().interact_size.y),
                            |ui| self.toolbar_overflow_menu(ui, &allocation.overflow),
                        );
                    }
                    ui.add_space(TOOLBAR_ROW_PADDING);
                },
            );
            ui.add_space(TOOLBAR_BOTTOM_PADDING);
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ToolbarPresentationMetrics {
    pub(super) icon_control_size: f32,
    pub(super) font_size: f32,
}

impl ToolbarPresentationMetrics {
    pub(super) fn font_id(self) -> FontId {
        FontId::proportional(self.font_size)
    }
}

pub(super) fn live_toolbar_metrics(ui: &egui::Ui) -> ToolbarPresentationMetrics {
    let font = ui
        .style()
        .text_styles
        .get(&TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| FontId::proportional(14.0));
    ToolbarPresentationMetrics {
        icon_control_size: ui.spacing().interact_size.y,
        font_size: font.size,
    }
}

pub(super) fn toolbar_chrome_color(ctx: &egui::Context) -> Color32 {
    ctx.style().visuals.widgets.open.weak_bg_fill
}

pub(super) fn toolbar_icon_control_rect(
    group_rect: egui::Rect,
    icon_control_size: Vec2,
) -> egui::Rect {
    egui::Rect::from_center_size(
        egui::pos2(
            group_rect.center().x,
            group_rect.min.y + icon_control_size.y / 2.0,
        ),
        icon_control_size,
    )
}

pub(super) fn toolbar_icon_hover_rect(icon_control_rect: egui::Rect, padding: f32) -> egui::Rect {
    egui::Rect::from_center_size(
        icon_control_rect.center(),
        icon_control_rect.size() + Vec2::splat(padding * 2.0),
    )
}

pub(super) fn toolbar_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Default
}

pub(super) fn toolbar_group_fill(style: &egui::Style, hovered: bool) -> Color32 {
    if hovered {
        style.visuals.widgets.hovered.bg_fill
    } else {
        Color32::TRANSPARENT
    }
}

pub(super) fn toolbar_content_height(style: &egui::Style) -> f32 {
    style.spacing.interact_size.y * 2.0 + TOOLBAR_BOTTOM_PADDING
}

#[cfg(test)]
pub(super) fn toolbar_layout_rect(panel: egui::Rect, toolbar_height: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        panel.left_top(),
        egui::pos2(
            panel.right(),
            (panel.top() + toolbar_height).min(panel.bottom()),
        ),
    )
}
