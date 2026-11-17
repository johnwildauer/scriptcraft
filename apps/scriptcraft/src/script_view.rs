use super::{
    add_performance_counter, add_performance_nanos, document_font, status_bar, theme,
    PerformanceDiagnostics, RevealIntent, ScriptCraftApp,
};
use super::status_bar::PresentedPage;
use crate::presentation_style::AdvanceMode;
use super::app_commands::CommandInvocation;
use super::shortcuts::{editor_command_for_key_event, ShortcutPlatform};
use eframe::egui::{self, Color32, FontFamily, FontId, Key, Sense, Stroke, Vec2};
use egui::epaint::text::{Galley, LayoutJob};
use scriptcraft_commands::{
    Command, CommandId, CommandOrigin, EditorSurface, HorizontalDirection, LineEdge,
    PointerPhase, ScriptViewMode, VerticalDirection,
};
use scriptcraft_core::editing::{EditError, Position, Selection};
use scriptcraft_core::formatting::EffectiveTextStyle;
use scriptcraft_core::layout::{
    Caret, FixedRect, InteractionQueryDiagnostics, LayoutLine, PositionedStyledSpan,
    ScreenplayLayout,
};
use scriptcraft_core::FixedPoint;
use std::time::{Duration, Instant};

const EDITOR_SURFACE_ID: &str = "page-editor-surface";
const CARET_WIDTH: f32 = 1.5;
const PAGE_HORIZONTAL_PADDING: f32 = 16.0;
const PAGE_VERTICAL_GAP: f32 = 12.0;
const SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING: f32 = 8.0;
const SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING: f32 = 0.0;
const PAGE_BORDER_WIDTH: f32 = 2.5;
const NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH: f32 = 1.0;
const PAGE_PAINT_OVERSCAN: usize = 1;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
enum CaretPresentation {
    Fixed(FixedRect),
    Projected(super::presentation_geometry::ProjectedCaretRect),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PresentedPagePolicy {
    page_height: f32,
    document_y_offset: f32,
    explicit_gap: f32,
    vertical_item_spacing: f32,
}

fn resolve_presentation_mode(
    surface: EditorSurface,
    body_mode: impl FnOnce() -> ScriptViewMode,
) -> ScriptViewMode {
    match surface {
        EditorSurface::Body => body_mode(),
        EditorSurface::TitlePage => ScriptViewMode::Page,
    }
}

fn presented_page_policy(
    mode: ScriptViewMode,
    paper_height: f32,
    content_top: f32,
    content_height: f32,
    scale: f32,
    vertical_item_spacing: f32,
) -> PresentedPagePolicy {
    let document_y_offset = match mode {
        ScriptViewMode::Speed => content_top,
        ScriptViewMode::Normal | ScriptViewMode::Page => 0.0,
    };
    let speed_view = mode == ScriptViewMode::Speed;
    PresentedPagePolicy {
        page_height: if speed_view {
            content_height * scale
        } else {
            paper_height * scale
        },
        document_y_offset,
        explicit_gap: if speed_view { 0.0 } else { PAGE_VERTICAL_GAP },
        vertical_item_spacing: if speed_view {
            0.0
        } else {
            vertical_item_spacing
        },
    }
}

fn page_chrome_visible(mode: ScriptViewMode) -> bool {
    mode == ScriptViewMode::Page
}

fn normal_view_page_bottom_rule_visible(
    mode: ScriptViewMode,
    page_index: usize,
    page_count: usize,
) -> bool {
    mode == ScriptViewMode::Normal && page_index < page_count.saturating_sub(1)
}

fn normal_view_page_bottom_rule_geometry(page_rect: egui::Rect) -> [egui::Pos2; 2] {
    [
        egui::pos2(page_rect.left(), page_rect.bottom()),
        egui::pos2(page_rect.right(), page_rect.bottom()),
    ]
}

fn script_view_backdrop(
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> Color32 {
    match mode {
        ScriptViewMode::Page => palette.background,
        ScriptViewMode::Normal | ScriptViewMode::Speed => palette.page.paper,
    }
}

fn document_y_to_screen(page: PresentedPage, document_y: f32, scale: f32) -> f32 {
    page.rect.top() + (document_y - page.document_y_offset) * scale
}

fn screen_y_to_document(page: PresentedPage, screen_y: f32, scale: f32) -> f32 {
    page.document_y_offset + (screen_y - page.rect.top()) / scale
}

fn page_local_screen_origin(page: PresentedPage, scale: f32) -> egui::Pos2 {
    egui::pos2(
        page.rect.left(),
        document_y_to_screen(page, 0.0, scale),
    )
}

fn viewport_caret_rect(
    presentation: CaretPresentation,
    page: PresentedPage,
    scale: f32,
) -> (egui::Rect, Option<egui::Rect>) {
    match presentation {
        CaretPresentation::Fixed(rect) => (
            egui::Rect::from_min_size(
                egui::pos2(
                    page.rect.left() + rect.x.to_points_f32() * scale,
                    document_y_to_screen(page, rect.y.to_points_f32(), scale),
                ),
                Vec2::new(CARET_WIDTH, rect.height.to_points_f32() * scale),
            ),
            None,
        ),
        CaretPresentation::Projected(projected) => {
            let page_origin = page_local_screen_origin(page, scale);
            let viewport_rect = projected.rect.translate(page_origin.to_vec2());
            let viewport_clip = egui::Rect::from_min_max(
                page_origin + projected.clip_bounds.min.to_vec2(),
                page_origin + projected.clip_bounds.max.to_vec2(),
            )
            .intersect(page.rect);
            (viewport_rect, Some(viewport_clip))
        }
    }
}

fn dual_group_navigation_edge(
    project: &scriptcraft_core::Project,
    group: &scriptcraft_core::dual_dialogue::DualDialogueGroup,
    forward: bool,
) -> Option<Position> {
    let element = if forward {
        scriptcraft_core::dual_dialogue::first_member(project, group)?
    } else {
        scriptcraft_core::dual_dialogue::last_member(project, group)?
    };
    let offset = if forward {
        0
    } else {
        project.elements.get(element)?.text.len()
    };
    Some(Position { element, offset })
}

fn viewport_paint_page_range(
    pages: &[PresentedPage],
    editor_clip: egui::Rect,
) -> Option<(usize, usize)> {
    let mut first_visible = None;
    let mut last_visible = None;
    for (page_index, page) in pages.iter().enumerate() {
        if page.rect.intersects(editor_clip) {
            first_visible.get_or_insert(page_index);
            last_visible = Some(page_index);
        }
    }
    let first_visible = first_visible?;
    let last_visible = last_visible?;
    Some((
        first_visible.saturating_sub(PAGE_PAINT_OVERSCAN),
        last_visible
            .saturating_add(PAGE_PAINT_OVERSCAN)
            .min(pages.len().saturating_sub(1)),
    ))
}

pub(super) fn has_nonzero_mouse_wheel(events: &[egui::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            egui::Event::MouseWheel { delta, .. } if *delta != Vec2::ZERO
        )
    })
}

#[cfg(test)]
mod presented_page_policy_tests {
    use super::{
        boundary_screen_geometry, document_y_to_screen, page_chrome_visible,
        presented_page_policy, presentation_metrics_at_zoom, resolve_presentation_mode,
        screen_y_to_document, viewport_caret_rect, CaretPresentation,
        normal_view_page_bottom_rule_geometry, normal_view_page_bottom_rule_visible,
        prepaint_zoom_scroll_offset, script_view_backdrop, PrepaintZoomPlan, PresentedPage,
        NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH, PAGE_VERTICAL_GAP,
        page_scroll_geometry, SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING,
    };
    use eframe::egui::{self, vec2, Vec2};
    use scriptcraft_commands::{EditorSurface, ScriptViewMode};
    use super::theme::{ApplicationPalette, ResolvedAppearance};

    fn workspace_palette(appearance: ResolvedAppearance) -> super::theme::ScriptWorkspacePalette {
        let visuals = match appearance {
            ResolvedAppearance::Day => egui::Visuals::light(),
            ResolvedAppearance::Night | ResolvedAppearance::Midnight => egui::Visuals::dark(),
        };
        ApplicationPalette::from_visuals(appearance, &visuals).script_workspace
    }

    #[test]
    fn allocation_policy_crops_speed_to_printable_height_and_preserves_other_modes() {
        let paper_height = 900.0;
        let top_margin = 37.0;
        let bottom_margin = 91.0;
        let content_height = paper_height - top_margin - bottom_margin;
        let vertical_item_spacing = 8.0;
        for mode in [ScriptViewMode::Normal, ScriptViewMode::Page, ScriptViewMode::Speed] {
            for zoom_percent in [75, 100, 300] {
                let scale = presentation_metrics_at_zoom(
                    1_000.0,
                    612.0,
                    12.0,
                    zoom_percent,
                )
                .scale;
                let policy = presented_page_policy(
                    mode,
                    paper_height,
                    top_margin,
                    content_height,
                    scale,
                    vertical_item_spacing,
                );

                assert_eq!(
                    policy.page_height,
                    if mode == ScriptViewMode::Speed {
                        (paper_height - top_margin - bottom_margin) * scale
                    } else {
                        paper_height * scale
                    }
                );
                assert_eq!(
                    policy.document_y_offset,
                    if mode == ScriptViewMode::Speed {
                        top_margin
                    } else {
                        0.0
                    }
                );
                assert_eq!(
                    policy.explicit_gap,
                    if mode == ScriptViewMode::Speed {
                        0.0
                    } else {
                        PAGE_VERTICAL_GAP
                    }
                );
                assert_eq!(
                    policy.vertical_item_spacing,
                    if mode == ScriptViewMode::Speed {
                        0.0
                    } else {
                        vertical_item_spacing
                    }
                );
            }
        }
        assert_eq!(SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING, 8.0);
        assert_eq!(super::SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING, 0.0);
    }

    #[test]
    fn speed_rows_are_adjacent_and_keep_each_source_pages_content_offset_and_index() {
        let page_content = [(42.0, 640.0), (31.0, 705.0), (58.0, 612.0)];
        let scale = 1.5;
        let page_width = 612.0 * scale;
        let horizontal = page_scroll_geometry(1_000.0, page_width);
        let page_left = horizontal.leading_padding;
        assert_eq!(horizontal.content_width, 1_000.0);
        assert_eq!(page_left, 41.0);
        assert_eq!(
            horizontal.content_width - page_left - page_width,
            41.0,
        );
        let mut next_top = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING;
        let presented = page_content
            .iter()
            .enumerate()
            .map(|(page_index, (content_top, content_height))| {
                let policy = presented_page_policy(
                    ScriptViewMode::Speed,
                    792.0,
                    *content_top,
                    *content_height,
                    scale,
                    8.0,
                );
                let page = PresentedPage {
                    rect: egui::Rect::from_min_size(
                        egui::pos2(page_left, next_top),
                        vec2(page_width, policy.page_height),
                    ),
                    document_y_offset: policy.document_y_offset,
                };
                next_top += policy.page_height + policy.explicit_gap + policy.vertical_item_spacing;
                (page_index, page)
            })
            .collect::<Vec<_>>();

        assert_eq!(presented.len(), page_content.len());
        for (page_index, (content_top, _)) in page_content.iter().enumerate() {
            assert_eq!(presented[page_index].0, page_index);
            assert_eq!(presented[page_index].1.document_y_offset, *content_top);
            assert_eq!(presented[page_index].1.rect.left(), page_left);
            assert_eq!(presented[page_index].1.rect.width(), page_width);
            if let Some(next_page) = presented.get(page_index + 1) {
                assert_eq!(presented[page_index].1.rect.bottom(), next_page.1.rect.top());
            }
        }

        let one_page_content = [(42.0, 640.0)];
        let one_page = one_page_content
            .iter()
            .enumerate()
            .map(|(page_index, (content_top, content_height))| {
                (
                    page_index,
                    presented_page_policy(
                        ScriptViewMode::Speed,
                        792.0,
                        *content_top,
                        *content_height,
                        scale,
                        8.0,
                    ),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(one_page.len(), 1);
        assert_eq!(one_page[0].0, 0);
        assert_eq!(one_page[0].1.page_height, 640.0 * scale);
        assert_eq!(one_page[0].1.explicit_gap, 0.0);
        assert_eq!(one_page[0].1.vertical_item_spacing, 0.0);
        assert_eq!(presented_page_policy(
            ScriptViewMode::Page,
            792.0,
            42.0,
            640.0,
            scale,
            8.0,
        ).page_height, 792.0 * scale);
        assert_eq!(presented_page_policy(
            ScriptViewMode::Normal,
            792.0,
            42.0,
            640.0,
            scale,
            8.0,
        ).page_height, 792.0 * scale);
    }

    #[test]
    fn pointer_coordinate_conversions_round_trip_speed_and_keep_page_normal_identity() {
        for (scale, top) in [(0.75, 41.0), (1.0, 125.5), (3.0, 8.0)] {
            for mode in [ScriptViewMode::Page, ScriptViewMode::Normal, ScriptViewMode::Speed] {
                let offset = if mode == ScriptViewMode::Speed { 24.0 } else { 0.0 };
                let page = PresentedPage {
                    rect: egui::Rect::from_min_size(
                        egui::pos2(0.0, top),
                        vec2(612.0 * scale, 792.0 * scale),
                    ),
                    document_y_offset: offset,
                };
                for document_y in [0.0, offset, offset + 0.25, 320.25, 792.0] {
                    let screen_y = document_y_to_screen(page, document_y, scale);
                    let round_trip = screen_y_to_document(page, screen_y, scale);
                    assert!((round_trip - document_y).abs() <= 0.0001);
                    if mode != ScriptViewMode::Speed {
                        assert_eq!(screen_y, top + document_y * scale);
                    }
                }
            }
        }
    }

    #[test]
    fn fixed_caret_spans_and_dual_boundaries_share_one_speed_y_translation() {
        let content_top = 29.0;
        let document_y = 185.0;
        let x = 17.0;
        let width = 43.0;
        for scale in [0.75, 1.0, 3.0] {
            let page_rect = egui::Rect::from_min_size(
                egui::pos2(101.0, 53.0),
                vec2(612.0 * scale, 792.0 * scale),
            );
            let page = PresentedPage {
                rect: page_rect,
                document_y_offset: content_top,
            };
            let fixed = super::FixedRect {
                x: scriptcraft_core::FixedPoint::from_points(x as i64),
                y: scriptcraft_core::FixedPoint::from_points(document_y as i64),
                width: scriptcraft_core::FixedPoint::from_points(width as i64),
                height: scriptcraft_core::FixedPoint::from_points(12),
            };
            let (caret, _) = viewport_caret_rect(CaretPresentation::Fixed(fixed), page, scale);
            assert_eq!(caret.left(), page_rect.left() + x * scale);
            assert_eq!(
                caret.top(),
                page_rect.top() + document_y * scale - content_top * scale
            );

            let (start_x, end_x, boundary_y) =
                boundary_screen_geometry(page, fixed, scale);
            assert_eq!(start_x, page_rect.left() + x * scale);
            assert_eq!(end_x, start_x + width * scale);
            assert_eq!(boundary_y, caret.top());
            assert_eq!(
                document_y_to_screen(page, document_y, scale),
                boundary_y
            );
            let fixed_span_origin_y = document_y_to_screen(page, document_y, scale);
            let projected_span_origin_y = document_y_to_screen(page, document_y, scale);
            assert_eq!(fixed_span_origin_y, projected_span_origin_y);
        }
    }

    #[test]
    fn projected_caret_and_clip_translate_once_without_mutating_natural_page_local_geometry() {
        use scriptcraft_core::formatting::{
            EffectiveTextStyle, FontFallback, ParagraphAlignment,
        };
        use scriptcraft_core::layout::{
            FixedRect, LayoutLine, PositionedStyledSpan, Rect, SourceRange,
        };
        use std::sync::Arc;

        let text_style = EffectiveTextStyle {
            declared_family: "Arial".to_owned(),
            family: FontFallback::CourierPrime,
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: None,
        };
        let source = SourceRange {
            element: 0,
            start: 0,
            end: 2,
        };
        let span = PositionedStyledSpan {
            display_text: Arc::from("AB"),
            source,
            display_source_map: Arc::from([]),
            text_style: text_style.clone(),
            char_width: scriptcraft_core::FixedPoint::from_points(10),
            geometry: FixedRect {
                x: scriptcraft_core::FixedPoint::from_points(15),
                y: scriptcraft_core::FixedPoint::from_points(30),
                width: scriptcraft_core::FixedPoint::from_points(20),
                height: scriptcraft_core::FixedPoint::from_points(12),
            },
            underline: false,
            strikethrough: false,
        };
        let core_line = LayoutLine {
            page: 0,
            role: scriptcraft_core::Role::Action,
            element: 0,
            source,
            text: Arc::from("AB"),
            display_text: Arc::from("AB"),
            display_source_map: Arc::from([]),
            spans: vec![span].into(),
            text_style,
            char_width: scriptcraft_core::FixedPoint::from_points(10),
            alignment: ParagraphAlignment::Left,
            unaligned_line_left: scriptcraft_core::FixedPoint::from_points(15),
            geometry: FixedRect {
                x: scriptcraft_core::FixedPoint::from_points(15),
                y: scriptcraft_core::FixedPoint::from_points(30),
                width: scriptcraft_core::FixedPoint::from_points(20),
                height: scriptcraft_core::FixedPoint::from_points(12),
            },
            paragraph_bounds: FixedRect {
                x: scriptcraft_core::FixedPoint::from_points(10),
                y: scriptcraft_core::FixedPoint::from_points(30),
                width: scriptcraft_core::FixedPoint::from_points(100),
                height: scriptcraft_core::FixedPoint::from_points(20),
            },
            rect: Rect {
                x: 10,
                y: 30,
                width: 100,
                height: 20,
            },
            keep_with_next: false,
            oversized: false,
        };
        let core_span = &core_line.spans[0];
        let projected_line = super::super::presentation_geometry::PresentationLine {
            core: &core_line,
            line_origin_x: 15.0,
            total_advance: 20.0,
            clip_bounds: egui::Rect::from_min_max(
                egui::pos2(10.0, 30.0),
                egui::pos2(110.0, 50.0),
            ),
            spans: vec![super::super::presentation_geometry::PresentationSpan {
                core: core_span,
                advance_mode: super::super::presentation_style::AdvanceMode::FixedCell,
                origin_x: 15.0,
                advance: 20.0,
                display_boundary_offsets: Arc::from([0.0, 10.0, 20.0]),
                natural: None,
            }],
        };
        for (scale, content_top) in [(0.75, 16.0), (1.0, 29.0), (3.0, 42.0)] {
            let projected = super::super::presentation_geometry::projected_caret_rect(
                &projected_line,
                1,
                scale,
                super::CARET_WIDTH,
            )
            .expect("fixture produces a projected caret");
            let natural_rect = projected.rect;
            let natural_clip = projected.clip_bounds;
            assert_eq!(natural_rect.top(), 30.0 * scale);
            let page_rect = egui::Rect::from_min_size(
                egui::pos2(101.0, 53.0),
                vec2(612.0 * scale, 792.0 * scale),
            );
            let page = PresentedPage {
                rect: page_rect,
                document_y_offset: content_top,
            };
            let (viewport_rect, viewport_clip) =
                viewport_caret_rect(CaretPresentation::Projected(projected), page, scale);
            let translation = egui::vec2(
                page_rect.left(),
                page_rect.top() - content_top * scale,
            );
            assert_eq!(viewport_rect, natural_rect.translate(translation));
            assert_eq!(
                viewport_clip,
                Some(natural_clip.translate(translation).intersect(page_rect))
            );
            assert_eq!(projected.rect, natural_rect);
            assert_eq!(projected.clip_bounds, natural_clip);
        }
    }

    #[test]
    fn title_page_always_resolves_to_page_independently_of_body_mode() {
        for body_mode in [ScriptViewMode::Normal, ScriptViewMode::Speed] {
            let body_mode_was_read = std::cell::Cell::new(false);
            assert_eq!(
                resolve_presentation_mode(EditorSurface::TitlePage, || {
                    body_mode_was_read.set(true);
                    body_mode
                }),
                ScriptViewMode::Page
            );
            assert!(!body_mode_was_read.get());
        }
        assert_eq!(
            resolve_presentation_mode(EditorSurface::Body, || ScriptViewMode::Normal),
            ScriptViewMode::Normal
        );
        assert_eq!(
            resolve_presentation_mode(EditorSurface::Body, || ScriptViewMode::Speed),
            ScriptViewMode::Speed
        );
    }

    #[test]
    fn page_row_origins_and_stride_remain_the_existing_full_page_geometry() {
        let policy = presented_page_policy(ScriptViewMode::Page, 792.0, 0.0, 612.0, 1.0, 8.0);
        let stride = policy.page_height + policy.explicit_gap + policy.vertical_item_spacing;
        assert_eq!(stride, 812.0);
        let row_origins = (0..3)
            .map(|page_index| {
                SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING + page_index as f32 * stride
            })
            .collect::<Vec<_>>();
        assert_eq!(row_origins, vec![8.0, 820.0, 1632.0]);

        let speed_policy = presented_page_policy(ScriptViewMode::Speed, 792.0, 40.0, 612.0, 1.0, 8.0);
        assert_eq!(speed_policy.page_height, 612.0);
        assert_eq!(
            speed_policy.page_height
                + speed_policy.explicit_gap
                + speed_policy.vertical_item_spacing,
            612.0,
        );
    }

    #[test]
    fn backdrop_policy_uses_workspace_for_page_and_paper_for_normal_and_speed() {
        for appearance in [
            ResolvedAppearance::Day,
            ResolvedAppearance::Night,
            ResolvedAppearance::Midnight,
        ] {
            let palette = workspace_palette(appearance);
            assert_eq!(
                script_view_backdrop(ScriptViewMode::Page, palette),
                palette.background,
                "Page backdrop for {appearance:?}",
            );
            for mode in [ScriptViewMode::Normal, ScriptViewMode::Speed] {
                assert_eq!(
                    script_view_backdrop(mode, palette),
                    palette.page.paper,
                    "{mode:?} backdrop for {appearance:?}",
                );
            }
        }
    }

    #[test]
    fn per_page_paper_chrome_is_enabled_only_for_page_mode() {
        assert!(page_chrome_visible(ScriptViewMode::Page));
        assert!(!page_chrome_visible(ScriptViewMode::Normal));
        assert!(!page_chrome_visible(ScriptViewMode::Speed));
    }

    #[test]
    fn normal_bottom_rule_is_visible_only_between_normal_pages() {
        let page_count = 3;
        for mode in [ScriptViewMode::Normal, ScriptViewMode::Page, ScriptViewMode::Speed] {
            assert_eq!(
                normal_view_page_bottom_rule_visible(mode, 0, page_count),
                mode == ScriptViewMode::Normal,
                "first page rule for {mode:?}",
            );
            assert_eq!(
                normal_view_page_bottom_rule_visible(mode, 1, page_count),
                mode == ScriptViewMode::Normal,
                "middle page rule for {mode:?}",
            );
            assert!(!normal_view_page_bottom_rule_visible(mode, 2, page_count));
            assert!(!normal_view_page_bottom_rule_visible(mode, 0, 1));
            assert!(!normal_view_page_bottom_rule_visible(mode, 0, 0));
        }

        for body_mode in [ScriptViewMode::Normal, ScriptViewMode::Speed] {
            let title_page_mode =
                resolve_presentation_mode(EditorSurface::TitlePage, || body_mode);
            assert_eq!(title_page_mode, ScriptViewMode::Page);
            assert!(!normal_view_page_bottom_rule_visible(title_page_mode, 0, page_count));
        }
    }

    #[test]
    fn normal_bottom_rule_uses_full_presented_paper_edges_and_preserves_page_geometry() {
        let first_rect = egui::Rect::from_min_max(egui::pos2(37.25, 11.5), egui::pos2(649.25, 803.5));
        let second_rect = first_rect.translate(vec2(0.0, 812.0));
        let page_rects = [first_rect, second_rect];
        let stride = 792.0 + PAGE_VERTICAL_GAP + 8.0;
        let scroll_extent = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING as f32
            + page_rects.len() as f32 * (792.0 + PAGE_VERTICAL_GAP + 8.0);

        assert_eq!(
            normal_view_page_bottom_rule_geometry(first_rect),
            [egui::pos2(37.25, 803.5), egui::pos2(649.25, 803.5)],
        );
        assert_eq!(NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH, 1.0);
        assert_eq!(page_rects, [first_rect, second_rect]);
        assert_eq!(stride, 812.0);
        assert_eq!(scroll_extent, 1632.0);
    }

    #[test]
    fn title_page_backdrop_and_page_chrome_resolve_from_forced_page_mode() {
        let palette = workspace_palette(ResolvedAppearance::Midnight);
        for body_mode in [ScriptViewMode::Normal, ScriptViewMode::Speed] {
            let mode = resolve_presentation_mode(EditorSurface::TitlePage, || body_mode);
            assert_eq!(mode, ScriptViewMode::Page);
            assert_eq!(script_view_backdrop(mode, palette), palette.background);
            assert!(page_chrome_visible(mode));
        }
    }

    fn viewport_anchor(
        page_index: usize,
        document_y: f32,
    ) -> super::super::status_bar::ViewportCenterAnchor {
        super::super::status_bar::ViewportCenterAnchor {
            page_index,
            page_local_document_point: vec2(280.0, document_y),
            outside_page_offset_points: vec2(0.0, 0.0),
            viewport_center: egui::pos2(0.0, 0.0),
        }
    }

    fn assert_plan_matches_allocation_policy(
        plan: &PrepaintZoomPlan,
        mode: ScriptViewMode,
        zoom_percent: u16,
        page_content: &[(f32, f32)],
        anchor: super::super::status_bar::ViewportCenterAnchor,
    ) {
        let paper_width = 612.0;
        let paper_height = 792.0;
        let base_font_size = 12.0;
        let vertical_spacing = 8.0;
        let outer = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), vec2(900.0, 600.0));
        let metrics = presentation_metrics_at_zoom(
            outer.width(),
            paper_width,
            base_font_size,
            zoom_percent,
        );
        let horizontal = page_scroll_geometry(outer.width(), metrics.page_width);
        let mut next_top = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING;
        let mut total_height =
            SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING + super::SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING;
        let mut expected_target = None;
        for (index, (content_top, content_height)) in page_content.iter().copied().enumerate() {
            let policy = presented_page_policy(
                mode,
                paper_height,
                content_top,
                content_height,
                metrics.scale,
                vertical_spacing,
            );
            let stride = policy.page_height + policy.explicit_gap + policy.vertical_item_spacing;
            assert_eq!(plan.page_origins[index], next_top);
            assert_eq!(plan.page_heights[index], policy.page_height);
            assert_eq!(plan.page_strides[index], stride);
            if index == anchor.page_index {
                let target_document_y = if mode == ScriptViewMode::Speed {
                    anchor
                        .page_local_document_point
                        .y
                        .clamp(content_top, content_top + content_height)
                } else {
                    anchor.page_local_document_point.y.clamp(0.0, paper_height)
                };
                expected_target = Some(
                    outer.min
                        + vec2(horizontal.leading_padding, next_top)
                        + vec2(
                            anchor.page_local_document_point.x,
                            target_document_y - policy.document_y_offset,
                        ) * metrics.scale,
                );
            }
            next_top += stride;
            total_height += stride;
        }
        assert_eq!(
            plan.content_size,
            vec2(horizontal.content_width, total_height),
        );
        let viewport_center = outer.center();
        let requested = expected_target.expect("anchor page is represented") - viewport_center;
        let maximum = (plan.content_size - outer.size()).max(Vec2::ZERO);
        assert_eq!(
            plan.scroll_offset,
            vec2(
                requested.x.clamp(0.0, maximum.x),
                requested.y.clamp(0.0, maximum.y),
            ),
        );
    }

    #[test]
    fn prepaint_geometry_matches_allocated_origins_strides_extent_and_offsets_at_zoom_levels() {
        let page_content = [(35.0, 650.0), (50.0, 620.0), (42.0, 660.0), (30.0, 670.0)];
        for mode in [ScriptViewMode::Page, ScriptViewMode::Normal, ScriptViewMode::Speed] {
            for zoom_percent in [75, 100, 300] {
                let anchor = viewport_anchor(2, 315.0);
                let plan = prepaint_zoom_scroll_offset(
                    anchor,
                    page_content.len(),
                    612.0,
                    792.0,
                    mode,
                    page_content.iter().copied(),
                    12.0,
                    zoom_percent,
                    egui::Rect::from_min_size(egui::pos2(20.0, 30.0), vec2(900.0, 600.0)),
                    8.0,
                )
                .expect("finite page allocation yields a zoom plan");
                assert_plan_matches_allocation_policy(
                    &plan,
                    mode,
                    zoom_percent,
                    &page_content,
                    anchor,
                );
                if mode == ScriptViewMode::Speed {
                    assert!(plan
                        .page_strides
                        .iter()
                        .zip(&plan.page_heights)
                        .all(|(stride, height)| stride == height));
                }
            }
        }
    }

    #[test]
    fn target_speed_crop_clamps_paper_margin_anchors_and_preserves_page_local_y() {
        let page_content = [(35.0, 650.0), (50.0, 620.0), (42.0, 660.0)];
        let outer = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), vec2(900.0, 600.0));
        for (document_y, expected_target_y) in [(10.0, 50.0), (780.0, 670.0)] {
            let anchor = viewport_anchor(1, document_y);
            let speed = prepaint_zoom_scroll_offset(
                anchor,
                page_content.len(),
                612.0,
                792.0,
                ScriptViewMode::Speed,
                page_content.iter().copied(),
                12.0,
                100,
                outer,
                8.0,
            )
            .expect("Speed target policy is valid");
            assert_plan_matches_allocation_policy(
                &speed,
                ScriptViewMode::Speed,
                100,
                &page_content,
                anchor,
            );
            let speed_policy = presented_page_policy(
                ScriptViewMode::Speed,
                792.0,
                page_content[1].0,
                page_content[1].1,
                presentation_metrics_at_zoom(outer.width(), 612.0, 12.0, 100).scale,
                8.0,
            );
            assert_eq!(
                document_y.clamp(
                    page_content[1].0,
                    page_content[1].0 + page_content[1].1,
                ),
                expected_target_y,
            );
            assert_eq!(speed.page_origins.len(), page_content.len());
            assert_eq!(speed_policy.document_y_offset, page_content[1].0);

            let page = prepaint_zoom_scroll_offset(
                anchor,
                page_content.len(),
                612.0,
                792.0,
                ScriptViewMode::Page,
                page_content.iter().copied(),
                12.0,
                100,
                outer,
                8.0,
            )
            .expect("Page target policy is valid");
            assert_eq!(page.page_origins.len(), page_content.len());
            assert_plan_matches_allocation_policy(
                &page,
                ScriptViewMode::Page,
                100,
                &page_content,
                anchor,
            );
        }
    }

    #[test]
    fn prepaint_handles_deep_horizontal_overflow_and_one_page_without_mutating_inputs() {
        let page_content = [(35.0, 650.0), (50.0, 620.0), (42.0, 660.0), (30.0, 670.0)];
        let anchor = viewport_anchor(3, 640.0);
        let original_anchor = anchor;
        let original_pages = page_content;
        let deep = prepaint_zoom_scroll_offset(
            anchor,
            page_content.len(),
            612.0,
            792.0,
            ScriptViewMode::Speed,
            page_content.iter().copied(),
            12.0,
            300,
            egui::Rect::from_min_size(egui::pos2(20.0, 30.0), vec2(400.0, 260.0)),
            8.0,
        )
        .expect("deep multi-page overflow has a finite plan");
        assert!(deep.content_size.x > 400.0);
        assert!(deep.scroll_offset.x > 0.0);
        assert!(deep.scroll_offset.y > 0.0);
        assert!(deep.scroll_offset.y <= deep.content_size.y - 260.0);
        assert_eq!(anchor, original_anchor);
        assert_eq!(page_content, original_pages);

        let one_page = [(42.0, 612.0)];
        let one_page_plan = prepaint_zoom_scroll_offset(
            viewport_anchor(0, 300.0),
            1,
            612.0,
            792.0,
            ScriptViewMode::Speed,
            one_page.iter().copied(),
            12.0,
            100,
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(900.0, 700.0)),
            8.0,
        )
        .expect("one-page allocation has a finite plan");
        assert_eq!(one_page_plan.page_origins.len(), 1);
        assert_eq!(one_page_plan.page_strides[0], one_page_plan.page_heights[0]);
    }
}

#[cfg(test)]
mod wheel_scroll_cancellation_tests {
    use super::has_nonzero_mouse_wheel;
    use eframe::egui::{self, Event, Modifiers, MouseWheelUnit, Vec2};

    fn wheel(modifiers: Modifiers) -> Event {
        Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: Vec2::new(0.0, 8.0),
            modifiers,
            phase: egui::TouchPhase::Move,
        }
    }

    #[test]
    fn consumed_zoom_wheel_does_not_cancel_anchor_but_ordinary_wheel_still_does() {
        let mut zoom_adapter = super::super::wheel_zoom::WheelZoomAdapter::default();
        let mut zoom_events = vec![wheel(Modifiers { command: true, ..Modifiers::NONE })];
        assert_eq!(zoom_adapter.adapt_events(&mut zoom_events, 600.0).len(), 1);
        assert!(!has_nonzero_mouse_wheel(&zoom_events));

        let mut ordinary_events = vec![wheel(Modifiers::NONE)];
        assert!(zoom_adapter.adapt_events(&mut ordinary_events, 600.0).is_empty());
        assert!(has_nonzero_mouse_wheel(&ordinary_events));
    }
}


impl ScriptCraftApp {
    fn caret_from_pointer(
        &mut self,
        layout: &ScreenplayLayout,
        page: usize,
        presented_page: PresentedPage,
        scale: f32,
        pointer: egui::Pos2,
        projections: Option<&[Option<Vec<Option<super::presentation_geometry::PresentationLine<'_>>>>]>,
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
                let line_index = layout.pages.get(located.page_index)?.lines.iter()
                    .position(|line| std::ptr::eq(line, located.line));
                let projected = line_index
                    .and_then(|index| {
                        projections
                            .and_then(|pages| pages.get(located.page_index))
                            .and_then(Option::as_ref)
                            .and_then(|lines| lines.get(index))
                    })
                    .and_then(Option::as_ref);
                if let Some(projected) = projected {
                    let screen_x = pointer.x - presented_page.rect.left();
                    Some(super::presentation_geometry::projected_caret_for_x(
                        projected,
                        screen_x,
                    ))
                } else {
                    let has_proportional = located.line.spans.iter().any(|span| {
                        resolve_span_paint(
                            &span.text_style,
                            span.text_style.size as f32 * scale,
                            self.theme_palette.script_workspace.page,
                            &self.document_font_availability,
                        ).advance_mode == AdvanceMode::Proportional
                    });
                    (!has_proportional).then(|| located.fixed_caret_for_x(local_x))
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

    pub(super) fn project_line_for_interaction<'a>(
        &mut self,
        ctx: &egui::Context,
        line: &'a LayoutLine,
        scale: f32,
    ) -> Result<Option<super::presentation_geometry::PresentationLine<'a>>, ()> {
        let palette = self.theme_palette.script_workspace.page;
        let resolved = line.spans.iter().map(|span| {
            resolve_span_paint(
                &span.text_style,
                span.text_style.size as f32 * scale,
                palette,
                &self.document_font_availability,
            )
        }).collect::<Vec<_>>();
        if !resolved.iter().any(|paint| paint.advance_mode == AdvanceMode::Proportional) {
            return Ok(None);
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("scriptcraft-interaction-projection"),
        ));
        let mut natural = Vec::with_capacity(line.spans.len());
        for (span, paint) in line.spans.iter().zip(&resolved) {
            if paint.advance_mode == AdvanceMode::Proportional {
                let (geometry, reused) = super::presentation_geometry::natural_singleline_geometry(
                    &painter,
                    &span.display_text,
                    &paint.font,
                    paint.advance_mode,
                    paint.color,
                ).map_err(|_| ())?;
                if reused {
                    self.performance.galleys_reused = self.performance.galleys_reused.saturating_add(1);
                } else {
                    self.performance.galleys_built = self.performance.galleys_built.saturating_add(1);
                }
                natural.push(Some(geometry));
            } else {
                natural.push(None);
            }
        }
        let modes = resolved.iter().map(|paint| paint.advance_mode).collect::<Vec<_>>();
        super::presentation_geometry::project_line(line, &modes, &natural, scale)
            .map(Some)
            .map_err(|_| ())
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

    fn selection_range(
        &self,
        line_start: usize,
        line_end: usize,
        element: usize,
        surface: EditorSurface,
    ) -> Option<(usize, usize)> {
        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        let (start, end) = ordered_selection(selection);
        if start.element > element || end.element < element {
            return None;
        }
        let from = if start.element == element {
            start.offset
        } else {
            line_start
        };
        let to = if end.element == element {
            end.offset
        } else {
            line_end
        };
        (from < line_end && to > line_start).then_some((from, to))
    }

    fn selected_display_range(
        span: &PositionedStyledSpan,
        source_selection: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        let (selection_start, selection_end) = source_selection?;
        let mut selected_start = None;
        let mut selected_end = None;
        for segment in span.display_source_map.iter() {
            if selection_start < segment.source_end && selection_end > segment.source_start {
                selected_start = Some(selected_start.map_or(segment.display_start, |current: usize| {
                    current.min(segment.display_start)
                }));
                selected_end = Some(selected_end.map_or(segment.display_end, |current: usize| {
                    current.max(segment.display_end)
                }));
            }
        }
        selected_start.zip(selected_end)
    }

    fn span_for_caret<'a>(
        line: &'a scriptcraft_core::layout::LayoutLine,
        offset: usize,
    ) -> Option<&'a PositionedStyledSpan> {
        line.spans
            .iter()
            .find(|span| offset >= span.source.start && offset < span.source.end)
            .or_else(|| {
                line.spans
                    .iter()
                    .rev()
                    .find(|span| offset == span.source.end)
            })
    }

    // Rendering needs these geometry and interaction inputs together; wrapping
    // them in a temporary parameter object would add noise to the paint path.
    #[allow(clippy::too_many_arguments)]
    fn paint_page(
        &mut self,
        ui: &mut egui::Ui,
        layout: &ScreenplayLayout,
        page_index: usize,
        presented_page: PresentedPage,
        metrics: PresentationMetrics,
        projections: &[Option<super::presentation_geometry::PresentationLine<'_>>],
        editor_id: egui::Id,
        caret_page_index: Option<usize>,
        caret_geometry: Option<CaretPresentation>,
        surface: EditorSurface,
        presentation_mode: ScriptViewMode,
    ) {
        let scale = metrics.scale;
        let page_rect = presented_page.rect;
        let paint_started = Instant::now();
        self.performance.painted_pages = self.performance.painted_pages.saturating_add(1);
        add_performance_counter(
            &mut self.performance.lines_submitted,
            layout.pages[page_index].lines.len() as u64,
        );
        let painter = ui.painter_at(page_rect);
        let palette = self.theme_palette.script_workspace.page;
        if page_chrome_visible(presentation_mode) {
            painter.rect_filled(page_rect, 2.0, palette.paper);
            painter.rect_stroke(
                page_rect,
                2.0,
                Stroke::new(PAGE_BORDER_WIDTH, palette.border),
                egui::StrokeKind::Middle,
            );
        }
        if normal_view_page_bottom_rule_visible(
            presentation_mode,
            page_index,
            layout.pages.len(),
        ) {
            let rule_painter = ui.painter().with_clip_rect(ui.clip_rect());
            rule_painter.line_segment(
                normal_view_page_bottom_rule_geometry(page_rect),
                Stroke::new(NORMAL_VIEW_PAGE_BOTTOM_RULE_WIDTH, palette.text),
            );
        }

        if surface == EditorSurface::Body
            && layout.purpose == scriptcraft_core::layout::LayoutPurpose::Editor
        {
            if let Some(active_group_id) = layout.active_group_id.as_ref() {
                for boundary in layout
                    .editor_group_boundaries
                    .iter()
                    .filter(|boundary| &boundary.group_id == active_group_id)
                {
                    for (boundary_page, geometry) in [
                        (boundary.upper_page, boundary.upper),
                        (boundary.lower_page, boundary.lower),
                    ] {
                        if boundary_page != page_index {
                            continue;
                        }
                        let (start_x, end_x, y) =
                            boundary_screen_geometry(presented_page, geometry, scale);
                        paint_dotted_boundary(
                            &painter,
                            start_x,
                            end_x,
                            y,
                            ui.ctx().pixels_per_point(),
                            palette.dual_dialogue_boundary,
                        );
                    }
                }
            }
        }

        'paint_lines: for (line_index, line) in layout.pages[page_index].lines.iter().enumerate() {
            let selection = self.selection_range(
                line.source.start,
                line.source.end,
                line.element,
                surface,
            );
            let resolved_paints = line
                .spans
                .iter()
                .map(|span| {
                    resolve_span_paint(
                        &span.text_style,
                        span.text_style.size as f32 * scale,
                        palette,
                        &self.document_font_availability,
                    )
                })
                .collect::<Vec<_>>();
            let has_proportional = resolved_paints
                .iter()
                .any(|resolved| resolved.advance_mode == AdvanceMode::Proportional);

            if has_proportional {
                // Unsupported natural topology fails closed. The same projection
                // object is also consumed by caret/reveal/IME and pointer dispatch.
                let Some(projection) = projections.get(line_index).and_then(Option::as_ref) else {
                    continue 'paint_lines;
                };
                debug_assert!(std::ptr::eq(projection.core, line));
                debug_assert!(projection.total_advance.is_finite());
                if let Some(first_span) = projection.spans.first() {
                    debug_assert_eq!(first_span.origin_x, projection.line_origin_x);
                }
                let page_origin = page_local_screen_origin(presented_page, scale);
                let line_clip = projection
                    .clip_bounds
                    .translate(page_origin.to_vec2())
                    .intersect(page_rect);
                let line_painter = painter.with_clip_rect(line_clip);
                for (projected, resolved) in projection.spans.iter().zip(&resolved_paints) {
                    let span = projected.core;
                    let span_y = document_y_to_screen(
                        presented_page,
                        span.geometry.y.to_points_f32(),
                        scale,
                    );
                    let span_height = span.geometry.height.to_points_f32() * scale;
                    let span_origin = egui::pos2(
                        page_rect.left() + projected.origin_x,
                        span_y,
                    );
                    let selected_display = Self::selected_display_range(span, selection);
                    if let Some((from, to)) = selected_display {
                        add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                        let boundaries = &projected.display_boundary_offsets;
                        let start = from.min(boundaries.len().saturating_sub(1));
                        let end = to.min(boundaries.len().saturating_sub(1));
                        let selected_x = span_origin.x + boundaries[start];
                        let selected_width = (boundaries[end] - boundaries[start]).max(0.0);
                        let selected_rect = egui::Rect::from_min_size(
                            egui::pos2(selected_x, span_y),
                            Vec2::new(selected_width.max(1.0), span_height),
                        );
                        line_painter.rect_filled(selected_rect, 0.0, palette.selection);
                        line_painter.rect_stroke(
                            selected_rect,
                            0.0,
                            Stroke::new(1.0, palette.selection_boundary),
                            egui::StrokeKind::Inside,
                        );
                    }

                    if projected.advance_mode == AdvanceMode::Proportional {
                        if let Some(geometry) = projected.natural.as_ref() {
                            line_painter.galley(
                                span_origin,
                                geometry.galley.clone(),
                                resolved.color,
                            );
                            if let Some((from, to)) = selected_display {
                                let boundaries = &projected.display_boundary_offsets;
                                let start = from.min(boundaries.len().saturating_sub(1));
                                let end = to.min(boundaries.len().saturating_sub(1));
                                let selected_rect = egui::Rect::from_min_max(
                                    egui::pos2(span_origin.x + boundaries[start], span_y),
                                    egui::pos2(
                                        span_origin.x + boundaries[end],
                                        span_y + span_height,
                                    ),
                                );
                                let selected_clip = line_clip.intersect(selected_rect);
                                if selected_rect.width() > 0.0
                                    && selected_clip.width() > 0.0
                                    && selected_clip.height() > 0.0
                                {
                                    let selected_galley =
                                        super::presentation_geometry::recolor_natural_galley(
                                            geometry,
                                            palette.selection_text,
                                        );
                                    painter
                                        .with_clip_rect(selected_clip)
                                        .galley(span_origin, selected_galley, palette.selection_text);
                                }
                            }
                        }
                    } else {
                        let display_count = span.display_text.chars().count();
                        let cell_width = if display_count == 0 {
                            span.char_width.to_points_f32() * scale
                        } else {
                            projected.advance / display_count as f32
                        };
                        paint_fixed_metric_text_with_selection(
                            &line_painter,
                            span_origin,
                            &span.display_text,
                            resolved.font.clone(),
                            cell_width,
                            FixedMetricTextStyle {
                                color: resolved.color,
                                selected_color: palette.selection_text,
                                selection: selected_display,
                            },
                            &mut self.performance,
                        );
                    }

                    if span.underline || span.strikethrough {
                        if span.underline {
                            let underline_y = span_y + span_height * 0.88;
                            line_painter.line_segment(
                                [
                                    egui::pos2(span_origin.x, underline_y),
                                    egui::pos2(span_origin.x + projected.advance, underline_y),
                                ],
                                Stroke::new(1.0_f32, resolved.color),
                            );
                        }
                        if span.strikethrough {
                            let strike_y = span_y + span_height * 0.52;
                            line_painter.line_segment(
                                [
                                    egui::pos2(span_origin.x, strike_y),
                                    egui::pos2(span_origin.x + projected.advance, strike_y),
                                ],
                                Stroke::new(1.0_f32, resolved.color),
                            );
                        }
                    }
                }
                continue;
            }

            for (span, resolved_paint) in line.spans.iter().zip(&resolved_paints) {
                let span_x = page_rect.left() + span.geometry.x.to_points_f32() * scale;
                let span_y = document_y_to_screen(
                    presented_page,
                    span.geometry.y.to_points_f32(),
                    scale,
                );
                let span_height = span.geometry.height.to_points_f32() * scale;
                let span_width = span.geometry.width.to_points_f32() * scale;
                let display_count = span.display_text.chars().count();
                if display_count == 0 {
                    continue;
                }
                let nominal_cell = span.char_width.to_points_f32() * scale;
                let cell_width = if span_width > nominal_cell * display_count as f32 {
                    span_width / display_count as f32
                } else {
                    nominal_cell
                };
                let selected_display = Self::selected_display_range(span, selection);
                if let Some((from, to)) = selected_display {
                    add_performance_counter(&mut self.performance.selection_spans_submitted, 1);
                    let selected_x = span_x + from as f32 * cell_width;
                    let selected_width = if display_count == 1 {
                        span_width
                    } else {
                        to.saturating_sub(from) as f32 * cell_width
                    };
                    let selected_rect = egui::Rect::from_min_size(
                        egui::pos2(selected_x, span_y),
                        Vec2::new(selected_width.max(1.0), span_height),
                    );
                    painter.rect_filled(selected_rect, 0.0, palette.selection);
                    painter.rect_stroke(
                        selected_rect,
                        0.0,
                        Stroke::new(1.0, palette.selection_boundary),
                        egui::StrokeKind::Inside,
                    );
                }
                let span_font = resolved_paint.font.clone();
                let color = resolved_paint.color;
                paint_fixed_metric_text_with_selection(
                    &painter,
                    egui::pos2(span_x, span_y),
                    &span.display_text,
                    span_font,
                    cell_width,
                    FixedMetricTextStyle {
                        color,
                        selected_color: palette.selection_text,
                        selection: selected_display,
                    },
                    &mut self.performance,
                );
                if span.underline || span.strikethrough {
                    if span.underline {
                        let underline_y = span_y + span_height * 0.88;
                        painter.line_segment(
                            [
                                egui::pos2(span_x, underline_y),
                                egui::pos2(span_x + span_width, underline_y),
                            ],
                            Stroke::new(1.0_f32, color),
                        );
                    }
                    if span.strikethrough {
                        let strike_y = span_y + span_height * 0.52;
                        painter.line_segment(
                            [
                                egui::pos2(span_x, strike_y),
                                egui::pos2(span_x + span_width, strike_y),
                            ],
                            Stroke::new(1.0_f32, color),
                        );
                    }
                }
            }
        }

        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        if caret_page_index == Some(page_index)
            && selection.anchor == selection.focus
        {
            if let Some(caret_geometry) = caret_geometry {
                let (caret_rect, caret_clip) =
                    viewport_caret_rect(caret_geometry, presented_page, scale);
                let caret = Caret {
                    element: selection.focus.element,
                    offset: selection.focus.offset,
                };
                let blink_id = editor_id.with((
                    "caret-blink",
                    self.session.revision(),
                    caret.element,
                    caret.offset,
                    self.caret_blink_focus_generation,
                ));
                let blink_active = self.surface_focused;
                if caret_blink_visible(
                    ui.ctx(),
                    blink_id,
                    blink_active,
                    true,
                    self.caret_blink_reset_time,
                ) {
                    if let Some(clip) = caret_clip {
                        painter.with_clip_rect(clip).rect_filled(caret_rect, 0.0, palette.caret);
                    } else {
                        painter.rect_filled(caret_rect, 0.0, palette.caret);
                    }
                }
                if self.surface_focused {
                    self.performance.repaint_requests =
                        self.performance.repaint_requests.saturating_add(1);
                }
                if !self.ime_preedit.is_empty() {
                    let caret_line = layout.pages[page_index].lines.iter().find(|line| {
                        line.element == caret.element
                            && caret.offset >= line.source.start
                            && caret.offset <= line.source.end
                    });
                    let caret_span = caret_line.and_then(|line| Self::span_for_caret(line, caret.offset));
                    let ime_font = caret_span.map_or_else(
                        || renderer_font(&layout.inputs.font_family, metrics.font_size),
                        |span| resolve_span_paint(
                            &span.text_style,
                            span.text_style.size as f32 * scale,
                            palette,
                            &self.document_font_availability,
                        ).font,
                    );
                    let ime_painter = caret_clip.map_or_else(
                        || painter.clone(),
                        |clip| painter.with_clip_rect(clip),
                    );
                    ime_painter.text(
                        caret_rect.min,
                        egui::Align2::LEFT_TOP,
                        &self.ime_preedit,
                        ime_font,
                        palette.ime_text,
                    );
                }
            }
        }

        self.performance.paint_nanos = self
            .performance
            .paint_nanos
            .saturating_add(paint_started.elapsed().as_nanos());
    }

    pub(super) fn live_document_surface(&mut self, ui: &mut egui::Ui, surface: EditorSurface) {
        let phase_started = self
            .development_mode
            .then(|| (Instant::now(), self.performance.paint_nanos));
        if self.development_mode {
            self.responsiveness.begin_document_surface(surface);
        }
        self.live_document_surface_inner(ui, surface);
        if let Some((started, paint_nanos_before)) = phase_started {
            self.responsiveness.record_document_surface(
                started.elapsed(),
                paint_nanos_before,
                self.performance.paint_nanos,
            );
        }
    }

    fn live_document_surface_inner(&mut self, ui: &mut egui::Ui, surface: EditorSurface) {
        let presentation_mode =
            resolve_presentation_mode(surface, || self.script_view_mode);
        let layout = match self.layout_for_render(surface) {
            Ok(layout) => layout,
            Err(error) => {
                self.page_geometry.invalidate();
                self.page_scale = None;
                self.show_error(format!(
                    "LAYOUT_BUILD_FAILED: {}; document unchanged",
                    crate::safe_layout_error_category(&error)
                ));
                return;
            }
        };
        self.rendered_revision = Some(self.session.revision());
        let editor_id = ui.id().with(EDITOR_SURFACE_ID);
        let scroll_outer_rect = ui.available_rect_before_wrap();
        let mut presented_pages = Vec::with_capacity(layout.pages.len());
        let mut frame_projections = std::iter::repeat_with(|| None)
            .take(layout.pages.len())
            .collect::<Vec<
                Option<Vec<Option<super::presentation_geometry::PresentationLine<'_>>>>,
            >>();
        let mut rendered_scale = 1.0;
        let mut visible_pages = 0;
        let reveal_intent = self.reveal_intent;
        let pending_zoom_anchor = self.pending_zoom_anchor;
        let zoom_compensation_offset = pending_zoom_anchor.and_then(|anchor| {
            prepaint_zoom_scroll_offset(
                anchor,
                layout.pages.len(),
                layout.inputs.paper_width as f32,
                layout.inputs.paper_height as f32,
                presentation_mode,
                layout.pages.iter().map(|page| {
                    (
                        page.content_bounds.y as f32,
                        page.content_bounds.height as f32,
                    )
                }),
                layout.inputs.font_size as f32,
                self.zoom_percent,
                scroll_outer_rect,
                ui.spacing().item_spacing.y,
            )
            .map(|plan| plan.scroll_offset)
        });
        let zoom_compensation_queued = zoom_compensation_offset.is_some();
        if zoom_compensation_queued {
            add_performance_counter(&mut self.performance.zoom_compensations_queued, 1);
        }
        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        // Temporarily override the current surface Ui style; keep the shared
        // context style and this ScrollArea's persistent identity unchanged.
        let scroll_fade = ui.style().spacing.scroll.fade;
        ui.style_mut().spacing.scroll.fade.strength = 0.0;
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt("page-editor-scroll")
            .content_margin(egui::Margin {
                left: 0,
                right: 0,
                top: SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING as i8,
                bottom: SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING as i8,
            });
        if let Some(offset) = zoom_compensation_offset {
            scroll_area = scroll_area.scroll_offset(offset);
        }
        let scroll_output = scroll_area.show(ui, |ui| {
            // ScrollArea's child max rect is the actual inner viewport after
            // reserved scrollbar width and parent clipping have been applied.
            let viewport_width = ui.available_width().max(0.0);
            let metrics = presentation_metrics_at_zoom(
                viewport_width,
                layout.inputs.paper_width as f32,
                layout.inputs.font_size as f32,
                self.zoom_percent,
            );
            let geometry = page_scroll_geometry(viewport_width, metrics.page_width);
            debug_assert_eq!(
                geometry.horizontal_overflow,
                geometry.content_width > geometry.viewport_width
            );
            let scale = metrics.scale;
            rendered_scale = scale;
            self.page_scale = Some(scale);
            ui.set_min_width(geometry.content_width);
            // Allocate every page row to preserve full scroll extent and source geometry;
            // page painting is a separate viewport-bounded pass below.
            let editor_clip = ui.clip_rect();
            let vertical_item_spacing = ui.spacing().item_spacing.y;
            if presentation_mode == ScriptViewMode::Speed {
                // Only page-row allocation uses this local Ui style; restore it
                // before paint/reveal widgets and leave the shared context intact.
                ui.spacing_mut().item_spacing.y = 0.0;
            }
            for (page_index, page) in layout.pages.iter().enumerate() {
                add_performance_counter(&mut self.performance.pages_considered, 1);
                let policy = presented_page_policy(
                    presentation_mode,
                    layout.inputs.paper_height as f32,
                    page.content_bounds.y as f32,
                    page.content_bounds.height as f32,
                    scale,
                    vertical_item_spacing,
                );
                ui.horizontal(|ui| {
                    // Explicit geometry owns all horizontal spacing; default
                    // row gaps would inflate measured scroll content.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(geometry.leading_padding);
                    let page_rect = ui
                        .allocate_space(Vec2::new(metrics.page_width, policy.page_height))
                        .1;
                    let trailing_padding =
                        (geometry.content_width - geometry.leading_padding - metrics.page_width)
                            .max(0.0);
                    ui.add_space(trailing_padding);
                    presented_pages.push(PresentedPage {
                        rect: page_rect,
                        document_y_offset: policy.document_y_offset,
                    });
                });
                if policy.explicit_gap > 0.0 {
                    ui.add_space(policy.explicit_gap);
                }
            }
            if presentation_mode == ScriptViewMode::Speed {
                ui.spacing_mut().item_spacing.y = vertical_item_spacing;
            }
            let paint_range = viewport_paint_page_range(&presented_pages, editor_clip);
            if self.development_mode {
                visible_pages = presented_pages
                    .iter()
                    .filter(|page| page.rect.intersects(editor_clip))
                    .count();
            }
            let painted_in_range = paint_range.map_or(0, |(first, last)| {
                last.saturating_sub(first).saturating_add(1)
            });
            add_performance_counter(
                &mut self.performance.pages_culled,
                presented_pages.len().saturating_sub(painted_in_range) as u64,
            );
            let caret_page_index = if reveal_intent == RevealIntent::Conditional {
                let mut query_diagnostics = InteractionQueryDiagnostics::default();
                let page = match surface {
                    EditorSurface::Body => layout.page_for_position_in_range_with_diagnostics(
                        selection.focus.element,
                        selection.focus.offset,
                        None,
                        &mut query_diagnostics,
                    ),
                    EditorSurface::TitlePage => layout.title_page_for_position(
                        selection.focus.element,
                        selection.focus.offset,
                    ),
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
                        EditorSurface::TitlePage => layout.title_page_for_position(
                            selection.focus.element,
                            selection.focus.offset,
                        ).filter(|page| *page >= first && *page <= last),
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
            };
            let resolve_caret = caret_page_index.is_some_and(|page_index| {
                reveal_intent == RevealIntent::Conditional
                    || (selection.anchor == selection.focus && !layout.pages[page_index].title_page)
            });
            let mut pages_to_project = paint_range
                .map(|(first, last)| (first..=last).collect::<Vec<_>>())
                .unwrap_or_default();
            if let Some(page_index) = caret_page_index {
                if !pages_to_project.contains(&page_index) {
                    pages_to_project.push(page_index);
                }
            }
            for page_index in pages_to_project {
                if frame_projections[page_index].is_none() {
                    frame_projections[page_index] = Some(
                        layout.pages[page_index]
                            .lines
                            .iter()
                            .map(|line| {
                                self.project_line_for_interaction(ui.ctx(), line, scale)
                                    .ok()
                                    .flatten()
                            })
                            .collect(),
                    );
                }
            }
            let caret_geometry = if resolve_caret {
                let caret_started = Instant::now();
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
                let result = located
                    .ok()
                    .and_then(|located| {
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
                        if let Some(projection) = projection {
                            super::presentation_geometry::projected_caret_rect(
                                projection,
                                caret.offset,
                                scale,
                                CARET_WIDTH,
                            )
                            .map(CaretPresentation::Projected)
                        } else if located.line.spans.iter().any(|span| {
                            resolve_span_paint(
                                &span.text_style,
                                span.text_style.size as f32 * scale,
                                self.theme_palette.script_workspace.page,
                                &self.document_font_availability,
                            )
                            .advance_mode
                                == AdvanceMode::Proportional
                        }) {
                            None
                        } else {
                            located
                                .fixed_caret_geometry(&caret)
                                .ok()
                                .map(CaretPresentation::Fixed)
                        }
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
            } else {
                None
            };
            let caret_paint_page_index = (selection.anchor == selection.focus)
                .then_some(caret_page_index)
                .flatten();
            if reveal_intent == RevealIntent::Conditional && !zoom_compensation_queued {
                if let (Some(page_index), Some(caret)) = (caret_page_index, caret_geometry) {
                    let presented_page = presented_pages[page_index];
                    let (caret_rect, caret_clip) =
                        viewport_caret_rect(caret, presented_page, scale);
                    let caret_is_clipped_out =
                        caret_clip.is_some_and(|clip| !clip.intersects(caret_rect));
                    if !caret_is_clipped_out && caret_needs_reveal(caret_rect, editor_clip) {
                        // `None` asks egui for nearest/minimal scrolling;
                        // centering would override the writer's viewport.
                        ui.scroll_to_rect(caret_rect, None);
                    }
                }
            }
            if let Some((first, last)) = paint_range {
                for (page_index, presented_page) in presented_pages
                    .iter()
                    .enumerate()
                    .take(last.saturating_add(1))
                    .skip(first)
                {
                    self.paint_page(
                        ui,
                        &layout,
                        page_index,
                        *presented_page,
                        metrics,
                        frame_projections[page_index].as_deref().unwrap_or(&[]),
                        editor_id,
                        caret_paint_page_index,
                        caret_geometry,
                        surface,
                        presentation_mode,
                    );
                }
            }
        });
        if self.development_mode {
            self.responsiveness
                .note_revision_painted(self.session.revision(), Instant::now());
            if surface == EditorSurface::Body {
                self.responsiveness
                    .note_pages(visible_pages, layout.pages.len());
            }
        }
        ui.style_mut().spacing.scroll.fade = scroll_fade;
        let scale = rendered_scale;
        let published_scroll_offset = scroll_output.state.offset;
        if !zoom_compensation_queued {
            self.reveal_intent = RevealIntent::None;
        }

        let actual_clip = scroll_output.inner_rect.intersect(ui.clip_rect());
        let geometry_count_matches = presented_pages.len() == layout.pages.len();
        let zoom_anchor_was_pending = pending_zoom_anchor.is_some();
        let needs_repaint = if geometry_count_matches {
            self.page_geometry.publish(
                self.performance.frames,
                self.session.revision(),
                ui.ctx().content_rect(),
                scale,
                &presented_pages,
                actual_clip,
                published_scroll_offset,
            )
        } else {
            self.page_geometry.invalidate();
            true
        };
        if geometry_count_matches {
            // The request is consumed only after the scaled page geometry has
            // been published. Invalid/missing anchors simply preserve old behavior.
            if zoom_anchor_was_pending {
                add_performance_counter(&mut self.performance.zoom_geometry_published, 1);
                add_performance_counter(&mut self.performance.zoom_anchor_consumed, 1);
            }
            self.pending_zoom_anchor = None;
        }
        if needs_repaint {
            self.request_repaint(ui.ctx());
        }

        // This is the only interactive response in the painted editor. Pages remain
        // source-mapped geometry, not independent focusable widgets.
        match surface {
            EditorSurface::Body => self.editor_surface_id = Some(editor_id),
            EditorSurface::TitlePage => self.title_editor_surface_id = Some(editor_id),
        }
        let response = ui
            .interact(scroll_output.inner_rect, editor_id, Sense::click_and_drag())
            .on_hover_cursor(document_cursor_icon());
        self.editor_surface_rect = Some(response.rect);
        let surface_focused = response.has_focus();
        if surface_focused != self.surface_focused {
            self.caret_blink_focus_generation = self.caret_blink_focus_generation.wrapping_add(1);
            if surface_focused {
                self.caret_blink_reset_time = ui.ctx().input(|input| input.time.max(0.0));
            }
            self.request_repaint(ui.ctx());
        }
        self.surface_focused = surface_focused;
        if surface_focused {
            self.set_active_editor_surface(surface);
        } else {
            self.reconcile_body_active_group();
        }
        if let Some(pointer) = response.interact_pointer_pos() {
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
                    Self::pointer_surface_page(&layout, &presented_pages, pointer, surface)
                {
                    if let Some(position) = self.caret_from_pointer(
                        &layout,
                        page_index,
                        presented_pages[page_index],
                        scale,
                        pointer,
                        Some(&frame_projections),
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
                                    Command::EditorSelectionPointer { position, phase, surface },
                                    CommandOrigin::Pointer,
                                ),
                                ui.ctx(),
                            );
                        }
                    }
                }
            }
        }
        self.native_input_diagnostics.focused = self.surface_focused;
        if self.surface_focused {
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    editor_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            self.native_input_diagnostics.focus_lock_installed = true;
        } else {
            self.native_input_diagnostics.focus_lock_installed = false;
        }
    }

    fn dispatch_surface_events(
        &mut self,
        events: &[egui::Event],
        modifiers: egui::Modifiers,
        ctx: &egui::Context,
        surface: EditorSurface,
    ) {
        let started = Instant::now();
        let ime_event_in_frame = events.iter().any(|event| matches!(event, egui::Event::Ime(_)));
        self.performance.events = self.performance.events.saturating_add(events.len() as u64);
        for (index, event) in events.iter().enumerate() {
            self.record_native_event(match event {
                egui::Event::Text(_) => "text",
                egui::Event::Paste(_) => "paste",
                egui::Event::Ime(egui::ImeEvent::Preedit(_)) => "ime-preedit",
                egui::Event::Ime(egui::ImeEvent::Commit(_)) => "ime-commit",
                egui::Event::Ime(egui::ImeEvent::Enabled) => "ime-enabled",
                egui::Event::Ime(egui::ImeEvent::Disabled) => "ime-disabled",
                egui::Event::Zoom(_) => "zoom",
                egui::Event::Key { pressed: true, .. } => "key-pressed",
                egui::Event::Key { pressed: false, .. } => "key-released",
                _ => "other",
            });
            let mut command = None;
            let result = match event {
                egui::Event::Zoom(factor) => {
                    if let Ok(factor) = scriptcraft_commands::ZoomFactor::new(*factor) {
                        command = Some(CommandInvocation::new(
                            Command::ViewZoomPinch { factor },
                            CommandOrigin::InternalFollowUp,
                        ));
                    }
                    None
                }
                egui::Event::Text(text)
                    if !text.is_empty()
                        && !self.ime_active
                        && !events[index + 1..].iter().any(|next| {
                            matches!(next, egui::Event::Ime(egui::ImeEvent::Commit(commit)) if commit == text)
                        })
                        && !modifiers.command
                        && !(modifiers.ctrl && !cfg!(target_os = "macos"))
                        && text.chars().all(|character| !character.is_control()) =>
                {
                    command = Some(CommandInvocation::new(
                        Command::EditorTextInsert { text: text.clone(), surface },
                        CommandOrigin::Keyboard,
                    ));
                    None
                }
                egui::Event::Paste(text) if !text.is_empty() => {
                    command = Some(CommandInvocation::new(
                        Command::EditorTextPaste { text: text.clone(), surface },
                        CommandOrigin::Keyboard,
                    ));
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                    self.ime_active = true;
                    self.ime_preedit.clone_from(text);
                    self.rendered_revision = None;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    if !text.is_empty() {
                        command = Some(CommandInvocation::new(
                            Command::EditorTextInsert { text: text.clone(), surface },
                            CommandOrigin::ImeCommit,
                        ));
                    }
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Enabled) => {
                    self.ime_active = true;
                    None
                }
                egui::Event::Ime(egui::ImeEvent::Disabled) => {
                    self.ime_active = false;
                    self.ime_preedit.clear();
                    None
                }
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if super::shortcuts::shortcut_input_allowed(
                        self.ime_active,
                        ime_event_in_frame,
                    ) {
                        if surface == EditorSurface::TitlePage
                            && self.menu_policy.in_window_shortcuts_owned()
                        {
                            command = super::menu::command_for_key_event_command(*key, *modifiers)
                                .map(|command| {
                                    CommandInvocation::new(command, CommandOrigin::Shortcut)
                                });
                        }
                        if command.is_none() {
                            command = super::shortcuts::dual_dialogue_toggle_for_focused_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                                self.menu_policy.in_window_shortcuts_owned(),
                            )
                            .filter(|_| surface == EditorSurface::Body)
                            .map(|command| {
                                CommandInvocation::new(command, CommandOrigin::Shortcut)
                            });
                        }
                        if command.is_none() {
                            command = editor_command_for_key_event(
                                *key,
                                *physical_key,
                                *modifiers,
                                ShortcutPlatform::current(),
                            )
                            .filter(|command| {
                                surface == EditorSurface::Body
                                    || !matches!(
                                        command,
                                        Command::EditorElementChange { .. }
                                            | Command::EditorParagraphTab
                                    )
                            })
                            .map(|command| {
                                let origin = match command.id() {
                                    CommandId::EditorParagraphEnter
                                    | CommandId::EditorParagraphTab
                                    | CommandId::EditorElementChange => CommandOrigin::Shortcut,
                                    _ => CommandOrigin::Keyboard,
                                };
                                CommandInvocation::new(command, origin)
                            });
                        }
                    }
                    if command.is_none() {
                        command = self
                            .navigation_key_event(*key, *modifiers, surface)
                            .map(|command| {
                            CommandInvocation::new(command, CommandOrigin::Keyboard)
                        });
                    }
                    None
                }
                _ => continue,
            };
            if let Some(command) = command {
                self.dispatch_command(command, ctx);
                continue;
            }
            if let Some(result) = result {
                self.mutate(
                    result,
                    RevealIntent::Conditional,
                );
            }
        }
        self.performance.event_nanos = self
            .performance
            .event_nanos
            .saturating_add(started.elapsed().as_nanos());
    }

    fn navigation_key_event(
        &self,
        key: Key,
        modifiers: egui::Modifiers,
        surface: EditorSurface,
    ) -> Option<Command> {
        match key {
            Key::ArrowLeft => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Left,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowRight => Some(Command::EditorNavigationHorizontal {
                direction: HorizontalDirection::Right,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowUp => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Up,
                extend: modifiers.shift,
                surface,
            }),
            Key::ArrowDown => Some(Command::EditorNavigationVertical {
                direction: VerticalDirection::Down,
                extend: modifiers.shift,
                surface,
            }),
            Key::Home => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::Start,
                extend: modifiers.shift,
                surface,
            }),
            Key::End => Some(Command::EditorNavigationLineEdge {
                edge: LineEdge::End,
                extend: modifiers.shift,
                surface: EditorSurface::Body,
            }),
            _ => None,
        }
    }

    pub(super) fn handle_surface_input(
        &mut self,
        ctx: &egui::Context,
        surface: EditorSurface,
    ) {
        if self.replacement_open_pending() {
            // Do not consume Body or Title Page semantic input while a selected
            // replacement is pending; the frame's events are intentionally not
            // queued for replay after a later safe resume.
            self.native_input_diagnostics.dispatch_enabled = false;
            return;
        }
        self.native_input_diagnostics.dispatch_enabled = self.surface_focused;
        if ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            // Explicit user scrolling wins over a not-yet-applied zoom request.
            // It must not be pulled back to the old viewport center afterward.
            if self.pending_zoom_anchor.is_some() {
                add_performance_counter(&mut self.performance.zoom_manual_scroll_cancellations, 1);
            }
            self.pending_zoom_anchor = None;
            self.reveal_intent = RevealIntent::None;
        }
        if !surface_input_is_dispatchable(
            self.surface_focused,
            self.active_editor_surface(),
            surface,
        ) {
            return;
        }
        self.native_input_diagnostics.dispatched_frames = self
            .native_input_diagnostics
            .dispatched_frames
            .saturating_add(1);
        let events = ctx.input(|input| input.events.clone());
        let modifiers = ctx.input(|input| input.modifiers);
        self.dispatch_surface_events(&events, modifiers, ctx, surface);
        if !events.is_empty() || ctx.input(|input| has_nonzero_mouse_wheel(&input.raw.events)) {
            self.request_repaint(ctx);
        }
    }

    pub(super) fn navigation_selection_horizontal(
        &mut self,
        direction: HorizontalDirection,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        if selection.anchor != selection.focus && !extend {
            let position = if direction == HorizontalDirection::Right {
                ordered_selection(selection).1
            } else {
                ordered_selection(selection).0
            };
            return Selection::caret(position);
        }
        let position = selection.focus;
        let project = self.session.project();
        let origin_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            position.element,
        );
        let item = &project.elements[position.element];
        let next = if direction == HorizontalDirection::Right {
            if position.offset < item.text.len() {
                Position {
                    offset: item.text[position.offset..]
                        .chars()
                        .next()
                        .map_or(position.offset, |c| position.offset + c.len_utf8()),
                    ..position
                }
            } else if position.element + 1 < project.elements.len() {
                Position {
                    element: position.element + 1,
                    offset: 0,
                }
            } else {
                position
            }
        } else if position.offset > 0 {
            Position {
                offset: item.text[..position.offset]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(at, _)| at),
                ..position
            }
        } else if position.element > 0 {
            let previous = &project.elements[position.element - 1];
            Position {
                element: position.element - 1,
                offset: previous.text.len(),
            }
        } else {
            position
        };
        let target_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            next.element,
        );
        let next = if let Some(group) = target_group.filter(|group| {
            origin_group.map_or(true, |origin| origin.id != group.id)
        }) {
            dual_group_navigation_edge(
                project,
                group,
                direction == HorizontalDirection::Right,
            )
            .unwrap_or(next)
        } else {
            next
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        }
    }

    pub(super) fn navigation_selection_vertical(
        &mut self,
        direction: VerticalDirection,
        extend: bool,
        ctx: &egui::Context,
    ) -> Result<Selection, EditError> {
        let selection = self.session.selection();
        let current = selection.focus;
        let layout = match self.layout_for_render(self.active_editor_surface()) {
            Ok(layout) => layout,
            Err(_) => {
                self.preferred_vertical_x = None;
                return Err(EditError::InvalidElement);
            }
        };
        let interaction_started = Instant::now();
        self.performance.caret_lookups = self.performance.caret_lookups.saturating_add(1);
        let mut query_diagnostics = InteractionQueryDiagnostics::default();
        let current_index = layout.body_line_index_for_position_with_diagnostics(
            current.element,
            current.offset,
            &mut query_diagnostics,
        );
        self.performance.indexed_interaction_queries = self
            .performance
            .indexed_interaction_queries
            .saturating_add(1);
        self.performance.interaction_index_entries_examined = self
            .performance
            .interaction_index_entries_examined
            .saturating_add(query_diagnostics.index_entries_examined);
        add_performance_nanos(
            &mut self.performance.caret_lookup_nanos,
            interaction_started.elapsed().as_nanos(),
        );
        let Some(current_index) = current_index else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let down = direction == VerticalDirection::Down;
        let target_index = if down {
            current_index
                .checked_add(1)
                .filter(|index| *index < layout.body_line_count())
        } else {
            current_index.checked_sub(1)
        };
        let Some(current_line) = layout.body_line(current_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let Some(target_index) = target_index else {
            self.preferred_vertical_x = None;
            let next = Position {
                element: current_line.element,
                offset: if down {
                    current_line.source.end
                } else {
                    current_line.source.start
                },
            };
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        };
        let Some(target_line) = layout.body_line(target_index) else {
            self.preferred_vertical_x = None;
            return Ok(selection);
        };
        let project = self.session.project();
        let origin_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            current.element,
        );
        let target_group = scriptcraft_core::dual_dialogue::group_at_element_index(
            project,
            target_line.element,
        );
        let mode_transition = match (origin_group, target_group) {
            (Some(origin), Some(target)) => origin.id != target.id,
            (None, Some(_)) | (Some(_), None) => true,
            (None, None) => false,
        };
        if let Some(group) = target_group.filter(|_| mode_transition) {
            self.preferred_vertical_x = None;
            let next = dual_group_navigation_edge(
                project,
                group,
                direction == VerticalDirection::Down,
            )
            .unwrap_or(Position {
                element: target_line.element,
                offset: target_line.source.start,
            });
            return Ok(if extend {
                Selection {
                    anchor: selection.anchor,
                    focus: next,
                }
            } else {
                Selection::caret(next)
            });
        }
        let scale = self.page_scale.unwrap_or(1.0);
        let current_projection = self
            .project_line_for_interaction(ctx, current_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let target_projection = self
            .project_line_for_interaction(ctx, target_line, scale)
            .map_err(|_| EditError::InvalidElement)?;
        let existing_goal = self.preferred_vertical_x;
        let goal = existing_goal.unwrap_or_else(|| {
            super::presentation_geometry::preferred_vertical_document_x(
                current_line,
                current.offset,
                current_projection.as_ref(),
                scale,
            )
            .unwrap_or(current_line.geometry.x.raw())
        });
        self.preferred_vertical_x = Some(goal);
        let target_display = if let Some(projection) = target_projection.as_ref() {
            super::presentation_geometry::projected_target_boundary(projection, goal, scale)
                .ok_or(EditError::InvalidElement)?
        } else {
            let cell = target_line.char_width.raw().max(1);
            (goal
                .saturating_sub(target_line.geometry.x.raw())
                .saturating_add(cell / 2)
                / cell) as u32 as usize
        };
        let next = Position {
            element: target_line.element,
            offset: target_line.source_for_display_char(target_display),
        };
        Ok(if extend {
            Selection {
                anchor: selection.anchor,
                focus: next,
            }
        } else {
            Selection::caret(next)
        })
    }

    pub(super) fn navigation_selection_line_edge(
        &mut self,
        edge: LineEdge,
        extend: bool,
    ) -> Selection {
        self.preferred_vertical_x = None;
        let selection = self.session.selection();
        let position = if edge == LineEdge::End {
            Position {
                element: selection.focus.element,
                offset: self.session.project().elements[selection.focus.element]
                    .text
                    .len(),
            }
        } else {
            Position {
                element: selection.focus.element,
                offset: 0,
            }
        };
        if extend {
            Selection {
                anchor: selection.anchor,
                focus: position,
            }
        } else {
            Selection::caret(position)
        }
    }
}

fn surface_input_is_dispatchable(
    surface_focused: bool,
    active_surface: EditorSurface,
    requested_surface: EditorSurface,
) -> bool {
    surface_focused && active_surface == requested_surface
}

#[cfg(test)]
mod focused_input_gate_tests {
    use super::{surface_input_is_dispatchable, EditorSurface};

    #[test]
    fn lost_focus_and_inactive_surface_do_not_dispatch_editor_input() {
        assert!(!surface_input_is_dispatchable(
            false,
            EditorSurface::Body,
            EditorSurface::Body,
        ));
        assert!(!surface_input_is_dispatchable(
            true,
            EditorSurface::TitlePage,
            EditorSurface::Body,
        ));
        assert!(surface_input_is_dispatchable(
            true,
            EditorSurface::Body,
            EditorSurface::Body,
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PresentationMetrics {
    page_width: f32,
    scale: f32,
    font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationPlatform {
    MacOS,
    #[cfg(target_os = "windows")]
    Windows,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Other,
}

fn logical_points_per_document_point(platform: PresentationPlatform) -> f32 {
    match platform {
        PresentationPlatform::MacOS => 1.0,
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        PresentationPlatform::Other => 1.0,
        #[cfg(target_os = "windows")]
        PresentationPlatform::Windows => 96.0 / 72.0,
    }
}

fn current_presentation_platform() -> PresentationPlatform {
    #[cfg(target_os = "macos")]
    {
        PresentationPlatform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        PresentationPlatform::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PresentationPlatform::Other
    }
}

fn presentation_metrics_at_zoom(
    available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
) -> PresentationMetrics {
    presentation_metrics_at_zoom_for_platform(
        available_width,
        paper_width,
        base_font_size,
        zoom_percent,
        current_presentation_platform(),
    )
}

fn presentation_metrics_at_zoom_for_platform(
    _available_width: f32,
    paper_width: f32,
    base_font_size: f32,
    zoom_percent: u16,
    platform: PresentationPlatform,
) -> PresentationMetrics {
    let normalized = status_bar::normalized_zoom_percent(zoom_percent as f64);
    let scale = logical_points_per_document_point(platform) * normalized as f32 / 100.0;
    let page_width = paper_width * scale;
    PresentationMetrics {
        page_width,
        scale,
        font_size: (base_font_size * scale).max(5.0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PageScrollGeometry {
    viewport_width: f32,
    content_width: f32,
    leading_padding: f32,
    horizontal_overflow: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct PrepaintZoomPlan {
    page_origins: Vec<f32>,
    page_heights: Vec<f32>,
    page_strides: Vec<f32>,
    content_size: Vec2,
    scroll_offset: Vec2,
}

fn paint_dotted_boundary(
    painter: &egui::Painter,
    start_x: f32,
    end_x: f32,
    y: f32,
    pixels_per_point: f32,
    color: Color32,
) {
    let physical_pixel = boundary_stroke_width(pixels_per_point);
    for [start, end] in dotted_boundary_segments(start_x, end_x, y, pixels_per_point) {
        painter.line_segment(
            [start, end],
            Stroke::new(physical_pixel, color),
        );
    }
}

fn boundary_stroke_width(pixels_per_point: f32) -> f32 {
    1.0 / pixels_per_point.max(f32::MIN_POSITIVE)
}

fn boundary_screen_geometry(
    page: PresentedPage,
    geometry: FixedRect,
    scale: f32,
) -> (f32, f32, f32) {
    let start_x = page.rect.left() + geometry.x.to_points_f32() * scale;
    let end_x = start_x + geometry.width.to_points_f32() * scale;
    let y = document_y_to_screen(page, geometry.y.to_points_f32(), scale);
    (start_x, end_x, y)
}

fn dotted_boundary_segments(
    start_x: f32,
    end_x: f32,
    y: f32,
    pixels_per_point: f32,
) -> Vec<[egui::Pos2; 2]> {
    let physical_pixel = 1.0 / pixels_per_point.max(f32::MIN_POSITIVE);
    let dash = physical_pixel * 2.0;
    let gap = physical_pixel * 2.0;
    let mut segments = Vec::new();
    let mut x = start_x;
    while x < end_x {
        let next = (x + dash).min(end_x);
        segments.push([egui::pos2(x, y), egui::pos2(next, y)]);
        x = next + gap;
    }
    segments
}

fn page_scroll_geometry(viewport_width: f32, page_width: f32) -> PageScrollGeometry {
    let viewport_width = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    let page_width = if page_width.is_finite() {
        page_width.max(0.0)
    } else {
        0.0
    };
    let buffered_page_width = page_width + PAGE_HORIZONTAL_PADDING * 2.0;
    let content_width = viewport_width.max(buffered_page_width);
    PageScrollGeometry {
        viewport_width,
        content_width,
        leading_padding: ((content_width - page_width) / 2.0).max(0.0),
        horizontal_overflow: buffered_page_width > viewport_width,
    }
}

// Pure geometry inputs stay explicit so this helper remains directly testable.
#[allow(clippy::too_many_arguments)]
fn prepaint_zoom_scroll_offset(
    anchor: status_bar::ViewportCenterAnchor,
    page_count: usize,
    paper_width: f32,
    paper_height: f32,
    mode: ScriptViewMode,
    page_content_bounds: impl ExactSizeIterator<Item = (f32, f32)> + Clone,
    base_font_size: f32,
    zoom_percent: u16,
    outer_rect: egui::Rect,
    vertical_item_spacing: f32,
) -> Option<PrepaintZoomPlan> {
    if anchor.page_index >= page_count
        || page_count == 0
        || page_content_bounds.len() != page_count
        || !paper_width.is_finite()
        || !paper_height.is_finite()
        || !base_font_size.is_finite()
        || !vertical_item_spacing.is_finite()
        || page_content_bounds
            .clone()
            .any(|(top, height)| !top.is_finite() || !height.is_finite())
    {
        return None;
    }
    // egui's overlay scrollbars do not consume the ScrollArea inner viewport.
    // Keep planning dimensions identical to the rect that will paint this frame.
    let inner_size = outer_rect.size().max(Vec2::ZERO);
    let metrics =
        presentation_metrics_at_zoom(inner_size.x, paper_width, base_font_size, zoom_percent);
    let geometry = page_scroll_geometry(inner_size.x, metrics.page_width);
    if !metrics.scale.is_finite() || metrics.scale <= 0.0 {
        return None;
    }
    let mut page_origins = Vec::with_capacity(page_count);
    let mut page_heights = Vec::with_capacity(page_count);
    let mut page_strides = Vec::with_capacity(page_count);
    let mut next_page_top = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING;
    let mut content_height = SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING
        + SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING;
    let mut target_policy = None;
    for (page_index, (content_top, content_page_height)) in
        page_content_bounds.enumerate()
    {
        let policy = presented_page_policy(
            mode,
            paper_height,
            content_top,
            content_page_height,
            metrics.scale,
            vertical_item_spacing,
        );
        if !policy.page_height.is_finite()
            || policy.page_height <= 0.0
            || !policy.document_y_offset.is_finite()
            || !policy.explicit_gap.is_finite()
            || !policy.vertical_item_spacing.is_finite()
        {
            return None;
        }
        if page_index == anchor.page_index {
            target_policy = Some((policy, content_top, content_page_height));
        }
        let page_stride =
            policy.page_height + policy.explicit_gap + policy.vertical_item_spacing;
        if !page_stride.is_finite() || page_stride <= 0.0 {
            return None;
        }
        page_origins.push(next_page_top);
        page_heights.push(policy.page_height);
        page_strides.push(page_stride);
        next_page_top += page_stride;
        content_height += page_stride;
    }
    let (target_policy, target_content_top, target_content_height) = target_policy?;
    let content_size = Vec2::new(
        geometry.content_width,
        content_height,
    );
    if !content_size.x.is_finite() || !content_size.y.is_finite() {
        return None;
    }
    let target_document_y = if mode == ScriptViewMode::Speed {
        anchor
            .page_local_document_point
            .y
            .clamp(target_content_top, target_content_top + target_content_height)
    } else {
        anchor.page_local_document_point.y.clamp(0.0, paper_height)
    };
    let target_presented_y = target_document_y - target_policy.document_y_offset;
    let content_target = outer_rect.min
        + Vec2::new(geometry.leading_padding, page_origins[anchor.page_index])
        + Vec2::new(
            anchor.page_local_document_point.x,
            target_presented_y,
        ) * metrics.scale
        + anchor.outside_page_offset_points;
    let viewport_center = outer_rect.min + inner_size * 0.5;
    let requested_offset = content_target - viewport_center;
    let max_offset = (content_size - inner_size).max(Vec2::ZERO);
    (requested_offset.x.is_finite() && requested_offset.y.is_finite()).then(|| {
        let scroll_offset = Vec2::new(
            requested_offset.x.clamp(0.0, max_offset.x),
            requested_offset.y.clamp(0.0, max_offset.y),
        );
        PrepaintZoomPlan {
            page_origins,
            page_heights,
            page_strides,
            content_size,
            scroll_offset,
        }
    })
}

struct FixedMetricTextStyle {
    color: Color32,
    selected_color: Color32,
    selection: Option<(usize, usize)>,
}

impl FixedMetricTextStyle {
    fn color_for(&self, selected: bool) -> Color32 {
        if selected {
            self.selected_color
        } else {
            self.color
        }
    }
}

// Text, selection, and paint instrumentation are independent view inputs.
#[allow(clippy::too_many_arguments)]
fn paint_fixed_metric_text_with_selection(
    painter: &egui::Painter,
    origin: egui::Pos2,
    text: &str,
    font: FontId,
    char_width: f32,
    style: FixedMetricTextStyle,
    performance: &mut PerformanceDiagnostics,
) {
    if text.is_empty() {
        return;
    }

    let char_count = text.chars().count();
    let selection = style
        .selection
        .map(|(start, end)| (start.min(char_count), end.min(char_count)));
    let mut spans = Vec::with_capacity(3);
    let mut span_start_byte = 0;
    let mut span_start_column = 0;
    let mut span_selected = selection.is_some_and(|(start, end)| start == 0 && end > 0);
    for (column, (byte, _)) in text.char_indices().enumerate().skip(1) {
        let selected = selection.is_some_and(|(start, end)| column >= start && column < end);
        if selected != span_selected {
            spans.push((span_start_byte..byte, span_start_column, span_selected));
            span_start_byte = byte;
            span_start_column = column;
            span_selected = selected;
        }
    }
    spans.push((
        span_start_byte..text.len(),
        span_start_column,
        span_selected,
    ));

    for (byte_range, start_column, selected) in spans {
        let span_text = &text[byte_range];
        if span_text.is_empty() {
            continue;
        }
        let color = style.color_for(selected);
        let span_origin = egui::pos2(fixed_metric_x(origin.x, start_column, char_width), origin.y);
        let (galley, reused, fixed_geometry) = fixed_cell_span_galley(
            painter,
            span_text,
            &font,
            char_width,
            color,
        );
        if reused {
            performance.galleys_reused = performance.galleys_reused.saturating_add(1);
        } else {
            performance.galleys_built = performance.galleys_built.saturating_add(1);
        }
        if fixed_geometry {
            painter.galley(span_origin, galley, color);
        } else {
            // Keep output geometry exact if a later epaint version changes its
            // single-row glyph/mesh contract; this compatibility path is not used
            // by the pinned 0.29 desktop implementation.
            for (column, character) in span_text.chars().enumerate() {
                painter.text(
                    egui::pos2(
                        fixed_metric_x(span_origin.x, column, char_width),
                        span_origin.y,
                    ),
                    egui::Align2::LEFT_TOP,
                    character.to_string(),
                    font.clone(),
                    color,
                );
            }
        }
    }
}

fn fixed_cell_span_galley(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    char_width: f32,
    color: Color32,
) -> (std::sync::Arc<Galley>, bool, bool) {
    // Keep ScriptCraft's fixed-cell transformation local to this egui pass.
    // Painter::layout_job can use egui's own atlas-aware layout cache, while no
    // UV-bearing transformed galley is retained by the application.
    let job = LayoutJob::simple_singleline(text.to_owned(), font.clone(), color);
    let mut galley = painter.layout_job(job).as_ref().clone();
    if !position_galley_in_fixed_cells(&mut galley, char_width) {
        // The epaint 0.29 single-row galley contract is one glyph record per
        // Unicode scalar and four mesh vertices per textured glyph. Preserve
        // correctness if a future backend violates that contract by leaving its
        // host layout untouched; the caller records the requested build.
        return (std::sync::Arc::new(galley), false, false);
    }
    let galley = std::sync::Arc::new(galley);
    (galley, false, true)
}

fn position_galley_in_fixed_cells(galley: &mut Galley, char_width: f32) -> bool {
    if galley.rows.len() != 1 {
        return false;
    }
    let char_count = galley.job.text.chars().count();
    let placed_row = &mut galley.rows[0];
    let row = std::sync::Arc::make_mut(&mut placed_row.row);
    if row.glyphs.len() != char_count {
        return false;
    }
    let expected_vertex_count = row
        .glyphs
        .iter()
        .filter(|glyph| !glyph.uv_rect.is_nothing())
        .count()
        .saturating_mul(4);
    if row.visuals.glyph_vertex_range.len() != expected_vertex_count {
        return false;
    }

    let pixels_per_point = galley.pixels_per_point;
    let mut vertex_index = row.visuals.glyph_vertex_range.start;
    let vertex_end = row.visuals.glyph_vertex_range.end;
    for (column, glyph) in row.glyphs.iter_mut().enumerate() {
        let fixed_x = column as f32 * char_width;
        if !glyph.uv_rect.is_nothing() {
            let Some(vertices) = row
                .visuals
                .mesh
                .vertices
                .get_mut(vertex_index..vertex_index.saturating_add(4))
            else {
                return false;
            };
            let round_to_pixel = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
            let left = fixed_x + round_to_pixel(glyph.uv_rect.offset.x);
            let right = left + glyph.uv_rect.size.x;
            vertices[0].pos.x = left;
            vertices[1].pos.x = right;
            vertices[2].pos.x = left;
            vertices[3].pos.x = right;
            vertex_index = vertex_index.saturating_add(4);
        }
        glyph.pos.x = fixed_x;
        glyph.advance_width = char_width;
    }
    if vertex_index != vertex_end {
        return false;
    }

    let fixed_width = char_count as f32 * char_width;
    placed_row.pos.x = 0.0;
    row.size.x = fixed_width;
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    galley.rect.min.x = 0.0;
    galley.rect.max.x = fixed_width;
    galley.mesh_bounds = galley.rows.iter().fold(egui::Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds)
    });
    true
}

fn renderer_font(font_family: &str, font_size: f32) -> FontId {
    FontId::new(
        font_size,
        if font_family == document_font::COURIER_PRIME_FAMILY {
            FontFamily::Name(document_font::COURIER_PRIME_FAMILY.into())
        } else {
            // Preserve the renderer's established appearance for explicit legacy
            // Courier/Courier New profiles; only the new Prime default selects
            // the embedded face. Viewport scale remains presentation-only.
            FontFamily::Monospace
        },
    )
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedSpanPaint {
    font: FontId,
    color: Color32,
    advance_mode: AdvanceMode,
}

fn resolve_span_paint(
    style: &EffectiveTextStyle,
    font_size: f32,
    palette: theme::PagePalette,
    availability: &document_font::FontAvailability,
) -> ResolvedSpanPaint {
    let resolved = super::presentation_style::resolve_effective_text_style(style);
    let face = document_font::resolve_document_face(
        resolved.family,
        style.bold,
        style.italic,
        availability,
    );
    let color = resolved.color_rgb8.map_or_else(
        || {
            if style.hidden_text {
                palette.subdued_text
            } else {
                palette.text
            }
        },
        |[red, green, blue]| Color32::from_rgb(red, green, blue),
    );
    ResolvedSpanPaint {
        font: FontId::new(font_size.max(1.0), face.family),
        color,
        advance_mode: face.advance_mode,
    }
}

#[cfg(test)]
mod imported_role_color_paint_tests {
    use super::resolve_span_paint;
    use crate::{document_font, theme};
    use eframe::egui;
    use scriptcraft_core::Role;
    use scriptcraft_core::fdx::{read_v6, write_v6};

    const BLACK_ROLE_COLOR_FIXTURE: &[u8] = include_bytes!("../../../.reference/black text.fdx");
    const FINAL_DRAFT_BLACK_FIXTURE: &[u8] =
        include_bytes!("../../../.reference/black text from final draft.fdx");
    const WHITE_TEXT_FIXTURE: &[u8] = include_bytes!("../../../.reference/white text.fdx");

    #[test]
    fn black_saved_black_and_white_controls_resolve_ordinary_and_hidden_roles_by_theme()
    -> Result<(), Box<dyn std::error::Error>> {
        let day = theme::ApplicationPalette::from_visuals(
            theme::ResolvedAppearance::Day,
            &egui::Visuals::light(),
        )
        .script_workspace
        .page;
        let night = theme::ApplicationPalette::from_visuals(
            theme::ResolvedAppearance::Night,
            &egui::Visuals::dark(),
        )
        .script_workspace
        .page;
        let availability = document_font::FontAvailability::default();

        for (fixture_name, fixture, imported_role_color) in [
            ("original black", BLACK_ROLE_COLOR_FIXTURE, Some("#000000000000")),
            ("Final Draft-saved black", FINAL_DRAFT_BLACK_FIXTURE, None),
            ("white control", WHITE_TEXT_FIXTURE, None),
        ] {
            let imported = read_v6(fixture)?;
            let output = write_v6(&imported.project)?;
            let reopened = read_v6(&output)?;

            for (phase, project, expected_role_color) in [
                ("import", &imported.project, imported_role_color),
                ("reopen", &reopened.project, None),
            ] {
                let Some(preservation) = project.preservation.as_ref() else {
                    return Err(format!("{fixture_name} {phase} retains source settings").into());
                };
                let settings = preservation.source_layout_settings();
                for role in [Role::Action, Role::Dialogue] {
                    let style = settings.text_style(role);
                    assert_eq!(
                        settings.font_spec(role).color.as_deref(),
                        expected_role_color,
                        "{fixture_name} {phase} source role color projection for {role:?}"
                    );
                    assert_eq!(
                        style.source_color, None,
                        "{fixture_name} {phase} ordinary role color stays theme-owned for {role:?}"
                    );

                    let day_paint = resolve_span_paint(&style, 12.0, day, &availability);
                    let night_paint = resolve_span_paint(&style, 12.0, night, &availability);
                    assert_eq!(day_paint.color, day.text, "{fixture_name} {phase} Day {role:?}");
                    assert_eq!(night_paint.color, night.text, "{fixture_name} {phase} Night {role:?}");
                    assert_ne!(day_paint.color, night_paint.color);
                    assert_eq!(day_paint.advance_mode, super::AdvanceMode::FixedCell);
                    assert_eq!(night_paint.advance_mode, super::AdvanceMode::FixedCell);
                    let day_selection_style = super::FixedMetricTextStyle {
                        color: day_paint.color,
                        selected_color: day.selection_text,
                        selection: Some((0, 1)),
                    };
                    let night_selection_style = super::FixedMetricTextStyle {
                        color: night_paint.color,
                        selected_color: night.selection_text,
                        selection: Some((0, 1)),
                    };
                    assert_eq!(day_selection_style.color_for(false), day_paint.color);
                    assert_eq!(day_selection_style.color_for(true), day.selection_text);
                    assert_eq!(night_selection_style.color_for(false), night_paint.color);
                    assert_eq!(night_selection_style.color_for(true), night.selection_text);
                }

                let hidden = settings.text_style(Role::Summary);
                assert!(hidden.hidden_text, "{fixture_name} {phase} retains hidden-role behavior");
                assert_eq!(hidden.source_color.as_deref(), Some("#87878F8FE0E0"));
                let day_hidden = resolve_span_paint(&hidden, 12.0, day, &availability);
                let night_hidden = resolve_span_paint(&hidden, 12.0, night, &availability);
                let canonical_hidden = egui::Color32::from_rgb(0x87, 0x8f, 0xe0);
                assert_eq!(day_hidden.color, canonical_hidden, "{fixture_name} {phase} hidden default in Day");
                assert_eq!(night_hidden.color, canonical_hidden, "{fixture_name} {phase} hidden default in Night");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod fixed_cell_atlas_lifetime_tests {
    use super::{fixed_cell_span_galley, position_galley_in_fixed_cells};
    use eframe::egui::{self, Color32, FontFamily, FontId};
    use egui::epaint::text::{Galley, LayoutJob};
    use std::sync::Arc;

    fn assert_galley_mesh_matches(actual: &Galley, expected: &Galley) {
        assert_eq!(actual.text(), expected.text());
        assert_eq!(actual.elided, expected.elided);
        assert_eq!(actual.rect, expected.rect);
        assert_eq!(actual.rows.len(), expected.rows.len());
        for (actual_placed, expected_placed) in actual.rows.iter().zip(&expected.rows) {
            assert_eq!(actual_placed.pos, expected_placed.pos);
            let actual_row = &actual_placed.row;
            let expected_row = &expected_placed.row;
            assert_eq!(actual_row.glyphs.len(), expected_row.glyphs.len());
            for (actual_glyph, expected_glyph) in
                actual_row.glyphs.iter().zip(&expected_row.glyphs)
            {
                assert_eq!(actual_glyph.chr, expected_glyph.chr);
                assert_eq!(actual_glyph.pos, expected_glyph.pos);
                assert_eq!(actual_glyph.advance_width, expected_glyph.advance_width);
                assert_eq!(actual_glyph.uv_rect, expected_glyph.uv_rect);
            }
            assert_eq!(actual_row.visuals.glyph_vertex_range, expected_row.visuals.glyph_vertex_range);
            assert_eq!(actual_row.visuals.mesh.indices, expected_row.visuals.mesh.indices);
            assert_eq!(actual_row.visuals.mesh.vertices.len(), expected_row.visuals.mesh.vertices.len());
            for (actual_vertex, expected_vertex) in actual_row
                .visuals
                .mesh
                .vertices
                .iter()
                .zip(&expected_row.visuals.mesh.vertices)
            {
                assert_eq!(actual_vertex.pos, expected_vertex.pos);
                assert_eq!(actual_vertex.uv, expected_vertex.uv);
                assert_eq!(actual_vertex.color, expected_vertex.color);
            }
        }
    }

    #[test]
    fn fixed_cell_variants_never_hit_a_previous_pass_cache_for_any_foreground() {
        let context = egui::Context::default();
        let font = FontId::new(18.0, FontFamily::Monospace);
        let colors = [Color32::WHITE, Color32::from_rgb(0x31, 0x72, 0xd0)];
        let mut previous: [Option<Arc<Galley>>; 2] = [None, None];

        // The first five passes exercise Day → Night → Day → Night → Day;
        // the final identical Day appearance proves lifetime is pass-based.
        for (pass, dark) in [false, true, false, true, false, false].into_iter().enumerate() {
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut built = 0usize;
            let _ = context.run(egui::RawInput::default(), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    for (color_index, color) in colors.into_iter().enumerate() {
                        let (galley, reused, fixed_geometry) = fixed_cell_span_galley(
                            ui.painter(),
                            "Fixed AV text",
                            &font,
                            9.0,
                            color,
                        );
                        assert!(!reused, "pass {pass} returned a custom cache hit");
                        assert!(fixed_geometry, "pass {pass} should preserve fixed-cell geometry");
                        if let Some(previous) = previous[color_index].as_ref() {
                            assert!(
                                !Arc::ptr_eq(previous, &galley),
                                "pass {pass} reused a UV-bearing fixed-cell galley from an earlier egui pass"
                            );
                        }

                        let mut fresh = ui.painter().layout_job(LayoutJob::simple_singleline(
                            "Fixed AV text".to_owned(),
                            font.clone(),
                            color,
                        )).as_ref().clone();
                        assert!(position_galley_in_fixed_cells(&mut fresh, 9.0));
                        assert_galley_mesh_matches(&galley, &fresh);
                        previous[color_index] = Some(galley);
                        built += 1;
                    }
                });
            });
            assert_eq!(built, colors.len(), "pass {pass} builds ordinary and selected variants");
        }
    }
}

fn fixed_metric_x(origin: f32, column: usize, char_width: f32) -> f32 {
    origin + column as f32 * char_width
}

fn caret_blink_visible(
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

fn caret_needs_reveal(caret_rect: egui::Rect, visible_clip: egui::Rect) -> bool {
    !visible_clip.contains_rect(caret_rect)
}

fn ordered_selection(selection: Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

pub(super) fn chrome_clear_color(
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> [f32; 4] {
    script_view_backdrop(mode, palette).to_normalized_gamma_f32()
}

pub(super) fn central_panel_frame(
    _ctx: &egui::Context,
    mode: ScriptViewMode,
    palette: theme::ScriptWorkspacePalette,
) -> egui::Frame {
    // Keep the resolved Script View surface color without a fixed viewport inset.
    egui::Frame::NONE.fill(script_view_backdrop(mode, palette))
}

fn document_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Text
}

#[cfg(test)]
mod title_surface_input_tests {
    use super::{EditorSurface, ScriptCraftApp};
    use eframe::egui;
    use scriptcraft_core::editing::{Position, Selection};

    #[test]
    fn ime_commit_in_child_targets_title_selection_and_child_ime_state() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = egui::Context::default();
        let title_index = 17;
        let original_title = app.session.project().title_page[title_index].text.clone();
        let title_caret = Position {
            element: title_index,
            offset: original_title.len(),
        };
        assert!(app
            .session
            .set_title_selection(Selection::caret(title_caret))
            .is_ok());
        let body_selection = app.session.selection();
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.swap_title_view_state();

        let events = [
            egui::Event::Ime(egui::ImeEvent::Preedit("draft".to_owned())),
            egui::Event::Ime(egui::ImeEvent::Commit("X".to_owned())),
        ];
        app.dispatch_surface_events(
            &events,
            egui::Modifiers::NONE,
            &context,
            EditorSurface::TitlePage,
        );

        assert_eq!(
            app.session.project().title_page[title_index].text,
            format!("{original_title}X"),
        );
        assert_eq!(app.session.selection(), body_selection);
        assert_eq!(app.session.title_selection().focus.offset, title_caret.offset + 1);
        assert!(!app.ime_active);
        assert!(app.ime_preedit.is_empty());
        app.swap_title_view_state();
    }

    #[test]
    fn body_paragraph_tab_shortcut_is_ignored_in_title_surface() {
        let app_result = ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        let context = egui::Context::default();
        let project_before = app.session.project().clone();
        let title_selection_before = app.session.title_selection();
        app.set_active_editor_surface(EditorSurface::TitlePage);
        app.swap_title_view_state();

        let events = [egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: Some(egui::Key::Tab),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        app.dispatch_surface_events(
            &events,
            egui::Modifiers::NONE,
            &context,
            EditorSurface::TitlePage,
        );

        assert_eq!(app.session.project(), &project_before);
        assert_eq!(app.session.title_selection(), title_selection_before);
        app.swap_title_view_state();
    }
}
