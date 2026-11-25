//! Central desktop appearance ownership. Neutral RGB values are visual estimates
//! from the 2026-11-05 reference images and require user side-by-side calibration.

use eframe::egui::{self, Color32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AppearanceMode {
    #[default]
    System,
    Day,
    Night,
    Midnight,
}

impl AppearanceMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Day => "Day Mode",
            Self::Night => "Night Mode",
            Self::Midnight => "Midnight Mode",
        }
    }

    fn theme_preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Day => egui::ThemePreference::Light,
            Self::Night | Self::Midnight => egui::ThemePreference::Dark,
        }
    }
}

pub(crate) type AppliedAppearance = Option<(AppearanceMode, egui::Theme)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppearanceContext {
    Root,
    TitlePage,
    Elements,
    TemplateSelector,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AppearanceApplications {
    root: AppliedAppearance,
    title_page: AppliedAppearance,
    elements: AppliedAppearance,
    template_selector: AppliedAppearance,
}

impl AppearanceApplications {
    pub(crate) fn get(self, context: AppearanceContext) -> AppliedAppearance {
        match context {
            AppearanceContext::Root => self.root,
            AppearanceContext::TitlePage => self.title_page,
            AppearanceContext::Elements => self.elements,
            AppearanceContext::TemplateSelector => self.template_selector,
        }
    }

    pub(crate) fn set(&mut self, context: AppearanceContext, applied: AppliedAppearance) {
        match context {
            AppearanceContext::Root => self.root = applied,
            AppearanceContext::TitlePage => self.title_page = applied,
            AppearanceContext::Elements => self.elements = applied,
            AppearanceContext::TemplateSelector => self.template_selector = applied,
        }
    }

    pub(crate) fn clear(&mut self, context: AppearanceContext) {
        self.set(context, None);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedAppearance {
    Day,
    Night,
    Midnight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PagePalette {
    pub(crate) paper: Color32,
    pub(crate) text: Color32,
    pub(crate) subdued_text: Color32,
    pub(crate) border: Color32,
    pub(crate) note_internal_separator: Color32,
    pub(crate) selection: Color32,
    pub(crate) selection_text: Color32,
    pub(crate) selection_boundary: Color32,
    pub(crate) dual_dialogue_boundary: Color32,
    pub(crate) caret: Color32,
    pub(crate) ime_text: Color32,
    pub(crate) scrollbar_background: Color32,
    pub(crate) scrollbar_neutral: Color32, 
    pub(crate) scrollbar_hovered: Color32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BeatPaintPalette {
    pub(crate) grid: Color32,
    pub(crate) unselected_card_border: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolbarPalette {
    pub(crate) background: Color32,
    pub(crate) border: Color32,
    pub(crate) formatting_separator: Color32,
    pub(crate) foreground: Color32,
    pub(crate) disabled_foreground: Color32,
    pub(crate) hover_fill: Color32,
    pub(crate) selected_fill: Color32,
    pub(crate) hover_foreground: Color32,
    pub(crate) pressed_fill: Color32,
    pub(crate) open_fill: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScriptWorkspacePalette {
    pub(crate) background: Color32,
    pub(crate) page: PagePalette,
    pub(crate) beat: BeatPaintPalette,
    pub(crate) editor_scrollbar: EditorScrollbarPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditorScrollbarPalette {
    pub(crate) track: Color32,
    pub(crate) resting_handle: Color32,
    pub(crate) hovered_handle: Color32,
    pub(crate) dragged_handle: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusPalette {
    pub(crate) background: Color32,
    pub(crate) border: Color32,
    pub(crate) foreground: Color32,
    pub(crate) center_foreground: Color32,
    pub(crate) hover_fill: Color32,
    pub(crate) hover_foreground: Color32,
    pub(crate) slider_rail: Color32,
    pub(crate) slider_trailing_fill: Color32,
    pub(crate) slider_knob: Color32,
    pub(crate) slider_knob_hover: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlPalette {
    pub(crate) noninteractive_fill: Color32,
    pub(crate) noninteractive_weak_fill: Color32,
    pub(crate) noninteractive_foreground: Color32,
    pub(crate) noninteractive_border: Color32,
    pub(crate) inactive_fill: Color32,
    pub(crate) inactive_weak_fill: Color32,
    pub(crate) inactive_foreground: Color32,
    pub(crate) inactive_border: Color32,
    pub(crate) hovered_fill: Color32,
    pub(crate) hovered_weak_fill: Color32,
    pub(crate) hovered_foreground: Color32,
    pub(crate) hovered_border: Color32,
    pub(crate) active_fill: Color32,
    pub(crate) active_weak_fill: Color32,
    pub(crate) active_foreground: Color32,
    pub(crate) active_border: Color32,
    pub(crate) open_fill: Color32,
    pub(crate) open_weak_fill: Color32,
    pub(crate) open_foreground: Color32,
    pub(crate) open_border: Color32,
    pub(crate) selection_fill: Color32,
    pub(crate) selection_foreground: Color32,
    pub(crate) text_cursor: Color32,
    pub(crate) weak_foreground: Color32,
    pub(crate) hyperlink: Color32,
    pub(crate) faint_background: Color32,
    pub(crate) extreme_background: Color32,
    pub(crate) text_edit_background: Color32,
    pub(crate) code_background: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverlayPalette {
    pub(crate) window_background: Color32,
    pub(crate) window_border: Color32,
    pub(crate) window_shadow: Color32,
    pub(crate) popup_shadow: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FeedbackPalette {
    pub(crate) warning: Color32,
    pub(crate) error: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ApplicationPalette {
    pub(crate) toolbar: ToolbarPalette,
    pub(crate) script_workspace: ScriptWorkspacePalette,
    pub(crate) status: StatusPalette,
    pub(crate) controls: ControlPalette,
    pub(crate) overlays: OverlayPalette,
    pub(crate) feedback: FeedbackPalette,
}

impl ApplicationPalette {
    pub(crate) fn from_visuals(appearance: ResolvedAppearance, visuals: &egui::Visuals) -> Self {
        let widgets = &visuals.widgets;
        let page = match appearance {
            ResolvedAppearance::Day => PagePalette {
                paper: Color32::WHITE,
                text: Color32::from_rgb(32, 32, 32),
                subdued_text: Color32::from_rgb(98, 98, 98),
                border: Color32::from_rgb(189, 189, 189),
                note_internal_separator: Color32::from_rgb(189, 189, 189),
                selection: Color32::from_rgb(56, 109, 177),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(0, 83, 125),
                dual_dialogue_boundary: Color32::from_rgb(142, 153, 163),
                caret: Color32::from_rgb(32, 32, 32),
                ime_text: Color32::from_rgb(76, 42, 122),
                scrollbar_background: Color32::from_rgb(246, 246, 246),
                scrollbar_neutral: Color32::from_rgb(175, 175, 175),
                scrollbar_hovered: Color32::from_rgb(125, 125, 125),
            },
            ResolvedAppearance::Night => PagePalette {
                paper: Color32::from_rgb(54, 54, 54),
                text: Color32::from_rgb(238, 238, 238),
                subdued_text: Color32::from_rgb(156, 163, 173),
                border: Color32::from_rgb(10, 10, 10),
                note_internal_separator: Color32::from_rgb(176, 176, 176),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(192, 222, 255),
                dual_dialogue_boundary: Color32::from_rgb(143, 153, 163),
                caret: Color32::from_rgb(238, 238, 238),
                ime_text: Color32::from_rgb(211, 194, 255),
                scrollbar_background: Color32::from_rgb(33, 33, 33),
                scrollbar_neutral: Color32::from_rgb(100, 100, 100),
                scrollbar_hovered: Color32::from_rgb(150, 150, 150),
            },
            ResolvedAppearance::Midnight => PagePalette {
                paper: Color32::from_rgb(27, 27, 27),
                text: Color32::from_rgb(238, 240, 244),
                subdued_text: Color32::from_rgb(156, 163, 173),
                border: Color32::from_rgb(20, 20, 20),
                note_internal_separator: Color32::from_rgb(176, 176, 176),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(192, 222, 255),
                dual_dialogue_boundary: Color32::from_rgb(124, 135, 146),
                caret: Color32::from_rgb(238, 240, 244),
                ime_text: Color32::from_rgb(211, 194, 255),
                scrollbar_background: Color32::from_rgb(33, 33, 33),
                scrollbar_neutral: Color32::from_rgb(100, 100, 100),
                scrollbar_hovered: Color32::from_rgb(150, 150, 150),
            },
        };
        let beat = match appearance {
            ResolvedAppearance::Day => BeatPaintPalette {
                grid: Color32::from_rgb(189, 189, 189),
                unselected_card_border: Color32::from_rgb(189, 189, 189),
            },
            ResolvedAppearance::Night => BeatPaintPalette {
                grid: Color32::from_rgb(80, 80, 80),
                unselected_card_border: Color32::from_rgb(105, 105, 105),
            },
            ResolvedAppearance::Midnight => BeatPaintPalette {
                grid: Color32::from_rgb(50, 50, 50),
                unselected_card_border: Color32::from_rgb(80, 80, 80),
            },
        };
        let hovered_foreground = widgets.hovered.text_color();
        let (toolbar_background, toolbar_border, toolbar_foreground, workspace_background) =
            match appearance {
                ResolvedAppearance::Day => (
                    Color32::from_rgb(236, 236, 236),
                    Color32::from_rgb(208, 208, 208),
                    Color32::from_rgb(90, 90, 90),
                    Color32::from_rgb(239, 239, 239),
                ),
                ResolvedAppearance::Night => (
                    Color32::from_rgb(54, 54, 54),
                    Color32::from_rgb(10, 10, 10),
                    Color32::from_rgb(180, 180, 180),
                    Color32::from_rgb(29, 29, 29),
                ),
                ResolvedAppearance::Midnight => (
                    Color32::from_rgb(54, 54, 54),
                    Color32::from_rgb(10, 10, 10),
                    Color32::from_rgb(180, 180, 180),
                    Color32::from_rgb(17, 17, 17),
                ),
            };
        let formatting_separator = match appearance {
            ResolvedAppearance::Day => Color32::from_rgb(220, 220, 220),
            ResolvedAppearance::Night => Color32::from_rgb(72, 72, 72),
            ResolvedAppearance::Midnight => Color32::from_rgb(68, 68, 68),
        };
        let toolbar_selected_fill = match appearance {
            ResolvedAppearance::Day => Color32::from_rgb(214, 214, 214),
            ResolvedAppearance::Night => Color32::from_rgb(80, 80, 80),
            ResolvedAppearance::Midnight => Color32::from_rgb(76, 76, 76),
        };
        let (status_background, status_border, status_foreground, center_foreground) =
            match appearance {
                ResolvedAppearance::Day => (
                    Color32::from_rgb(246, 246, 246),
                    Color32::from_rgb(208, 208, 208),
                    Color32::from_rgb(90, 90, 90),
                    Color32::from_rgb(48, 48, 48),
                ),
                ResolvedAppearance::Night => (
                    Color32::from_rgb(33, 33, 33),
                    Color32::from_rgb(35, 35, 35),
                    Color32::from_rgb(180, 180, 180),
                    Color32::from_rgb(230, 230, 230),
                ),
                ResolvedAppearance::Midnight => (
                    Color32::from_rgb(33, 33, 33),
                    Color32::from_rgb(35, 35, 35),
                    Color32::from_rgb(180, 180, 180),
                    Color32::from_rgb(230, 230, 230),
                ),
            };
        let slider_rail = match appearance {
            ResolvedAppearance::Day => Color32::from_rgb(215, 215, 215),
            ResolvedAppearance::Night => Color32::from_rgb(70, 70, 70),
            ResolvedAppearance::Midnight => Color32::from_rgb(60, 60, 60),
        };

        Self {
            toolbar: ToolbarPalette {
                background: toolbar_background,
                border: toolbar_border,
                formatting_separator,
                foreground: toolbar_foreground,
                disabled_foreground: alpha_color(toolbar_foreground, 0.45),
                hover_fill: widgets.hovered.bg_fill,
                selected_fill: toolbar_selected_fill,
                hover_foreground: hovered_foreground,
                pressed_fill: widgets.active.bg_fill,
                open_fill: widgets.open.bg_fill,
            },
            script_workspace: ScriptWorkspacePalette {
                background: workspace_background,
                page,
                beat,
                editor_scrollbar: EditorScrollbarPalette {
                    track: page.scrollbar_background,
                    resting_handle: page.scrollbar_neutral,
                    hovered_handle: page.scrollbar_hovered,
                    dragged_handle: page.scrollbar_hovered,
                },
            },
            status: StatusPalette {
                background: status_background,
                border: status_border,
                foreground: status_foreground,
                center_foreground,
                hover_fill: widgets.hovered.bg_fill,
                hover_foreground: hovered_foreground,
                slider_rail,
                slider_trailing_fill: visuals.selection.bg_fill,
                slider_knob: status_foreground,
                slider_knob_hover: hovered_foreground,
            },
            controls: ControlPalette {
                noninteractive_fill: widgets.noninteractive.bg_fill,
                noninteractive_weak_fill: widgets.noninteractive.weak_bg_fill,
                noninteractive_foreground: widgets.noninteractive.text_color(),
                noninteractive_border: widgets.noninteractive.bg_stroke.color,
                inactive_fill: widgets.inactive.bg_fill,
                inactive_weak_fill: widgets.inactive.weak_bg_fill,
                inactive_foreground: widgets.inactive.text_color(),
                inactive_border: widgets.inactive.bg_stroke.color,
                hovered_fill: widgets.hovered.bg_fill,
                hovered_weak_fill: widgets.hovered.weak_bg_fill,
                hovered_foreground,
                hovered_border: widgets.hovered.bg_stroke.color,
                active_fill: widgets.active.bg_fill,
                active_weak_fill: widgets.active.weak_bg_fill,
                active_foreground: widgets.active.text_color(),
                active_border: widgets.active.bg_stroke.color,
                open_fill: widgets.open.bg_fill,
                open_weak_fill: widgets.open.weak_bg_fill,
                open_foreground: widgets.open.text_color(),
                open_border: widgets.open.bg_stroke.color,
                selection_fill: visuals.selection.bg_fill,
                selection_foreground: visuals.selection.stroke.color,
                text_cursor: visuals.text_cursor.stroke.color,
                weak_foreground: visuals.weak_text_color(),
                hyperlink: visuals.hyperlink_color,
                faint_background: visuals.faint_bg_color,
                extreme_background: visuals.extreme_bg_color,
                text_edit_background: visuals.text_edit_bg_color(),
                code_background: visuals.code_bg_color,
            },
            overlays: OverlayPalette {
                window_background: visuals.window_fill,
                window_border: visuals.window_stroke.color,
                window_shadow: visuals.window_shadow.color,
                popup_shadow: visuals.popup_shadow.color,
            },
            feedback: FeedbackPalette {
                warning: visuals.warn_fg_color,
                error: visuals.error_fg_color,
            },
        }
    }

    fn apply_to_visuals(self, visuals: &mut egui::Visuals) {
        let controls = self.controls;
        visuals.panel_fill = controls.noninteractive_fill;
        visuals.widgets.noninteractive.bg_fill = controls.noninteractive_fill;
        visuals.widgets.noninteractive.weak_bg_fill = controls.noninteractive_weak_fill;
        visuals.widgets.noninteractive.fg_stroke.color = controls.noninteractive_foreground;
        visuals.widgets.noninteractive.bg_stroke.color = controls.noninteractive_border;
        visuals.widgets.inactive.bg_fill = controls.inactive_fill;
        visuals.widgets.inactive.weak_bg_fill = controls.inactive_weak_fill;
        visuals.widgets.inactive.fg_stroke.color = controls.inactive_foreground;
        visuals.widgets.inactive.bg_stroke.color = controls.inactive_border;
        visuals.widgets.hovered.bg_fill = controls.hovered_fill;
        visuals.widgets.hovered.weak_bg_fill = controls.hovered_weak_fill;
        visuals.widgets.hovered.fg_stroke.color = controls.hovered_foreground;
        visuals.widgets.hovered.bg_stroke.color = controls.hovered_border;
        visuals.widgets.active.bg_fill = controls.active_fill;
        visuals.widgets.active.weak_bg_fill = controls.active_weak_fill;
        visuals.widgets.active.fg_stroke.color = controls.active_foreground;
        visuals.widgets.active.bg_stroke.color = controls.active_border;
        visuals.widgets.open.bg_fill = controls.open_fill;
        visuals.widgets.open.weak_bg_fill = controls.open_weak_fill;
        visuals.widgets.open.fg_stroke.color = controls.open_foreground;
        visuals.widgets.open.bg_stroke.color = controls.open_border;
        visuals.selection.bg_fill = controls.selection_fill;
        visuals.selection.stroke.color = controls.selection_foreground;
        visuals.text_cursor.stroke.color = controls.text_cursor;
        visuals.weak_text_color = Some(controls.weak_foreground);
        visuals.hyperlink_color = controls.hyperlink;
        visuals.faint_bg_color = controls.faint_background;
        visuals.extreme_bg_color = controls.extreme_background;
        visuals.text_edit_bg_color = Some(controls.text_edit_background);
        visuals.code_bg_color = controls.code_background;
        visuals.window_fill = self.overlays.window_background;
        visuals.window_stroke.color = self.overlays.window_border;
        visuals.window_shadow.color = self.overlays.window_shadow;
        visuals.popup_shadow.color = self.overlays.popup_shadow;
        visuals.warn_fg_color = self.feedback.warning;
        visuals.error_fg_color = self.feedback.error;
    }
}

fn alpha_color(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (f32::from(color.a()) * alpha).round().clamp(0.0, 255.0) as u8,
    )
}

fn visuals_for_appearance(appearance: ResolvedAppearance) -> egui::Visuals {
    let mut visuals = match appearance {
        ResolvedAppearance::Day => egui::Visuals::light(),
        ResolvedAppearance::Night | ResolvedAppearance::Midnight => egui::Visuals::dark(),
    };
    let widgets = &mut visuals.widgets;
    match appearance {
        ResolvedAppearance::Day => {
            visuals.panel_fill = Color32::from_rgb(248, 248, 248);
            visuals.window_fill = Color32::from_rgb(248, 248, 248);
            visuals.window_stroke.color = Color32::from_rgb(190, 190, 190);
            visuals.faint_bg_color = Color32::from_rgb(245, 245, 245);
            visuals.extreme_bg_color = Color32::WHITE;
            visuals.text_edit_bg_color = Some(Color32::WHITE);
            visuals.code_bg_color = Color32::from_rgb(230, 230, 230);
            visuals.weak_text_color = Some(Color32::from_gray(140));
            widgets.noninteractive.bg_fill = Color32::from_rgb(248, 248, 248);
            widgets.noninteractive.weak_bg_fill = Color32::from_rgb(248, 248, 248);
            widgets.noninteractive.bg_stroke.color = Color32::from_rgb(190, 190, 190);
            widgets.inactive.bg_fill = Color32::from_rgb(230, 230, 230);
            widgets.inactive.weak_bg_fill = Color32::from_rgb(230, 230, 230);
            widgets.hovered.bg_fill = Color32::from_rgb(220, 220, 220);
            widgets.hovered.weak_bg_fill = Color32::from_rgb(220, 220, 220);
            widgets.hovered.bg_stroke.color = Color32::from_rgb(105, 105, 105);
            widgets.active.bg_fill = Color32::from_rgb(165, 165, 165);
            widgets.active.weak_bg_fill = Color32::from_rgb(165, 165, 165);
            widgets.open.bg_fill = Color32::from_rgb(220, 220, 220);
            widgets.open.weak_bg_fill = Color32::from_rgb(220, 220, 220);
            widgets.open.bg_stroke.color = Color32::from_rgb(160, 160, 160);
        }
        ResolvedAppearance::Night => {
            visuals.panel_fill = Color32::from_rgb(44, 44, 44);
            visuals.window_fill = Color32::from_rgb(48, 48, 48);
            visuals.window_stroke.color = Color32::from_rgb(60, 60, 60);
            visuals.faint_bg_color = Color32::from_rgb(32, 32, 32);
            visuals.extreme_bg_color = Color32::from_rgb(10, 10, 10);
            visuals.text_edit_bg_color = Some(Color32::from_rgb(32, 32, 32));
            visuals.code_bg_color = Color32::from_rgb(64, 64, 64);
            visuals.weak_text_color = Some(Color32::from_gray(140));
            widgets.noninteractive.bg_fill = Color32::from_rgb(44, 44, 44);
            widgets.noninteractive.weak_bg_fill = Color32::from_rgb(44, 44, 44);
            widgets.noninteractive.bg_stroke.color = Color32::from_rgb(60, 60, 60);
            widgets.inactive.bg_fill = Color32::from_rgb(60, 60, 60);
            widgets.inactive.weak_bg_fill = Color32::from_rgb(60, 60, 60);
            widgets.hovered.bg_fill = Color32::from_rgb(72, 72, 72);
            widgets.hovered.weak_bg_fill = Color32::from_rgb(72, 72, 72);
            widgets.hovered.bg_stroke.color = Color32::from_rgb(150, 150, 150);
            widgets.active.bg_fill = Color32::from_rgb(55, 55, 55);
            widgets.active.weak_bg_fill = Color32::from_rgb(55, 55, 55);
            widgets.open.bg_fill = Color32::from_rgb(48, 48, 48);
            widgets.open.weak_bg_fill = Color32::from_rgb(48, 48, 48);
            widgets.open.bg_stroke.color = Color32::from_rgb(60, 60, 60);
        }
        ResolvedAppearance::Midnight => {
            visuals.panel_fill = Color32::from_rgb(40, 40, 40);
            visuals.window_fill = Color32::from_rgb(40, 40, 40);
            visuals.window_stroke.color = Color32::from_rgb(55, 55, 55);
            visuals.faint_bg_color = Color32::from_rgb(28, 28, 28);
            visuals.extreme_bg_color = Color32::from_rgb(8, 8, 8);
            visuals.text_edit_bg_color = Some(Color32::from_rgb(24, 24, 24));
            visuals.code_bg_color = Color32::from_rgb(54, 54, 54);
            visuals.weak_text_color = Some(Color32::from_gray(140));
            widgets.noninteractive.bg_fill = Color32::from_rgb(40, 40, 40);
            widgets.noninteractive.weak_bg_fill = Color32::from_rgb(40, 40, 40);
            widgets.noninteractive.bg_stroke.color = Color32::from_rgb(55, 55, 55);
            widgets.inactive.bg_fill = Color32::from_rgb(56, 56, 56);
            widgets.inactive.weak_bg_fill = Color32::from_rgb(56, 56, 56);
            widgets.hovered.bg_fill = Color32::from_rgb(68, 68, 68);
            widgets.hovered.weak_bg_fill = Color32::from_rgb(68, 68, 68);
            widgets.hovered.bg_stroke.color = Color32::from_rgb(145, 145, 145);
            widgets.active.bg_fill = Color32::from_rgb(50, 50, 50);
            widgets.active.weak_bg_fill = Color32::from_rgb(50, 50, 50);
            widgets.open.bg_fill = Color32::from_rgb(44, 44, 44);
            widgets.open.weak_bg_fill = Color32::from_rgb(44, 44, 44);
            widgets.open.bg_stroke.color = Color32::from_rgb(55, 55, 55);
        }
    }
    visuals
}

fn dark_visuals(appearance: ResolvedAppearance) -> egui::Visuals {
    visuals_for_appearance(appearance)
}

fn resolved_appearance(mode: AppearanceMode, theme: egui::Theme) -> ResolvedAppearance {
    match mode {
        AppearanceMode::Day => ResolvedAppearance::Day,
        AppearanceMode::Night => ResolvedAppearance::Night,
        AppearanceMode::Midnight => ResolvedAppearance::Midnight,
        AppearanceMode::System => match theme {
            egui::Theme::Light => ResolvedAppearance::Day,
            egui::Theme::Dark => ResolvedAppearance::Night,
        },
    }
}

pub(crate) fn configure_appearance(
    ctx: &egui::Context,
    mode: AppearanceMode,
    currently_applied: AppliedAppearance,
) -> (egui::Theme, AppliedAppearance, ApplicationPalette) {
    let system_theme = ctx
        .system_theme()
        .unwrap_or_else(|| ctx.options(|options| options.fallback_theme));
    let (resolved_theme, applied, palette, _) =
        configure_appearance_for_theme(ctx, mode, currently_applied, system_theme);
    (resolved_theme, applied, palette)
}

fn configure_appearance_for_theme(
    ctx: &egui::Context,
    mode: AppearanceMode,
    currently_applied: AppliedAppearance,
    system_theme: egui::Theme,
) -> (egui::Theme, AppliedAppearance, ApplicationPalette, bool) {
    let resolved_theme = match mode {
        AppearanceMode::System => system_theme,
        AppearanceMode::Day => egui::Theme::Light,
        AppearanceMode::Night | AppearanceMode::Midnight => egui::Theme::Dark,
    };
    let applied = Some((mode, resolved_theme));
    let appearance = resolved_appearance(mode, resolved_theme);
    let mut light = visuals_for_appearance(ResolvedAppearance::Day);
    let mut dark = dark_visuals(if mode == AppearanceMode::Midnight {
        ResolvedAppearance::Midnight
    } else {
        ResolvedAppearance::Night
    });

    let mut active_visuals = match resolved_theme {
        egui::Theme::Light => light.clone(),
        egui::Theme::Dark => dark.clone(),
    };
    let palette = ApplicationPalette::from_visuals(appearance, &active_visuals);
    palette.apply_to_visuals(&mut active_visuals);
    match resolved_theme {
        egui::Theme::Light => light = active_visuals,
        egui::Theme::Dark => dark = active_visuals,
    }

    let installed = currently_applied != applied;
    if installed {
        ctx.set_visuals_of(egui::Theme::Light, light.clone());
        ctx.set_visuals_of(egui::Theme::Dark, dark.clone());
        ctx.set_theme(mode.theme_preference());
    }

    (resolved_theme, applied, palette, installed)
}
