//! Central desktop appearance ownership. Neutral RGB values are visual estimates
//! from the 2026-11-05 reference images and require user side-by-side calibration.

use eframe::egui::{self, Color32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AppearanceMode {
    #[default]
    System,
    Day,
    Night,
    Midnight,
}

impl AppearanceMode {
    pub(super) fn label(self) -> &'static str {
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

pub(super) type AppliedAppearance = Option<(AppearanceMode, egui::Theme)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResolvedAppearance {
    Day,
    Night,
    Midnight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PagePalette {
    pub(super) paper: Color32,
    pub(super) text: Color32,
    pub(super) subdued_text: Color32,
    pub(super) border: Color32,
    pub(super) selection: Color32,
    pub(super) selection_text: Color32,
    pub(super) selection_boundary: Color32,
    pub(super) dual_dialogue_boundary: Color32,
    pub(super) caret: Color32,
    pub(super) ime_text: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ToolbarPalette {
    pub(super) background: Color32,
    pub(super) border: Color32,
    pub(super) foreground: Color32,
    pub(super) disabled_foreground: Color32,
    pub(super) hover_fill: Color32,
    pub(super) selected_fill: Color32,
    pub(super) hover_foreground: Color32,
    pub(super) pressed_fill: Color32,
    pub(super) open_fill: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScriptWorkspacePalette {
    pub(super) background: Color32,
    pub(super) page: PagePalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StatusPalette {
    pub(super) background: Color32,
    pub(super) border: Color32,
    pub(super) foreground: Color32,
    pub(super) center_foreground: Color32,
    pub(super) hover_fill: Color32,
    pub(super) hover_foreground: Color32,
    pub(super) slider_rail: Color32,
    pub(super) slider_trailing_fill: Color32,
    pub(super) slider_knob: Color32,
    pub(super) slider_knob_hover: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ControlPalette {
    pub(super) noninteractive_fill: Color32,
    pub(super) noninteractive_weak_fill: Color32,
    pub(super) noninteractive_foreground: Color32,
    pub(super) noninteractive_border: Color32,
    pub(super) inactive_fill: Color32,
    pub(super) inactive_weak_fill: Color32,
    pub(super) inactive_foreground: Color32,
    pub(super) inactive_border: Color32,
    pub(super) hovered_fill: Color32,
    pub(super) hovered_weak_fill: Color32,
    pub(super) hovered_foreground: Color32,
    pub(super) hovered_border: Color32,
    pub(super) active_fill: Color32,
    pub(super) active_weak_fill: Color32,
    pub(super) active_foreground: Color32,
    pub(super) active_border: Color32,
    pub(super) open_fill: Color32,
    pub(super) open_weak_fill: Color32,
    pub(super) open_foreground: Color32,
    pub(super) open_border: Color32,
    pub(super) selection_fill: Color32,
    pub(super) selection_foreground: Color32,
    pub(super) text_cursor: Color32,
    pub(super) weak_foreground: Color32,
    pub(super) hyperlink: Color32,
    pub(super) faint_background: Color32,
    pub(super) extreme_background: Color32,
    pub(super) text_edit_background: Color32,
    pub(super) code_background: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OverlayPalette {
    pub(super) window_background: Color32,
    pub(super) window_border: Color32,
    pub(super) window_shadow: Color32,
    pub(super) popup_shadow: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FeedbackPalette {
    pub(super) warning: Color32,
    pub(super) error: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ApplicationPalette {
    pub(super) toolbar: ToolbarPalette,
    pub(super) script_workspace: ScriptWorkspacePalette,
    pub(super) status: StatusPalette,
    pub(super) controls: ControlPalette,
    pub(super) overlays: OverlayPalette,
    pub(super) feedback: FeedbackPalette,
}

impl ApplicationPalette {
    pub(super) fn from_visuals(
        appearance: ResolvedAppearance,
        visuals: &egui::Visuals,
    ) -> Self {
        let widgets = &visuals.widgets;
        let page = match appearance {
            ResolvedAppearance::Day => PagePalette {
                paper: Color32::WHITE,
                text: Color32::from_rgb(32, 32, 32),
                subdued_text: Color32::from_rgb(98, 98, 98),
                border: Color32::from_rgb(189, 189, 189),
                selection: Color32::from_rgb(56, 109, 177),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(0, 83, 125),
                dual_dialogue_boundary: Color32::from_rgb(142, 153, 163),
                caret: Color32::from_rgb(20, 80, 150),
                ime_text: Color32::from_rgb(76, 42, 122),
            },
            ResolvedAppearance::Night => PagePalette {
                paper: Color32::from_rgb(54, 54, 54),
                text: Color32::from_rgb(238, 238, 238),
                subdued_text: Color32::from_rgb(156, 163, 173),
                border: Color32::from_rgb(10, 10, 10),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(192, 222, 255),
                dual_dialogue_boundary: Color32::from_rgb(143, 153, 163),
                caret: Color32::from_rgb(85, 213, 255),
                ime_text: Color32::from_rgb(211, 194, 255),
            },
            ResolvedAppearance::Midnight => PagePalette {
                paper: Color32::from_rgb(27, 27, 27),
                text: Color32::from_rgb(238, 240, 244),
                subdued_text: Color32::from_rgb(156, 163, 173),
                border: Color32::from_rgb(20, 20, 20),
                selection: Color32::from_rgb(75, 117, 163),
                selection_text: Color32::WHITE,
                selection_boundary: Color32::from_rgb(192, 222, 255),
                dual_dialogue_boundary: Color32::from_rgb(124, 135, 146),
                caret: Color32::from_rgb(85, 213, 255),
                ime_text: Color32::from_rgb(211, 194, 255),
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
        let toolbar_selected_fill = match appearance {
            ResolvedAppearance::Day => Color32::from_rgb(214, 214, 214),
            ResolvedAppearance::Night | ResolvedAppearance::Midnight => {
                Color32::from_rgb(66, 66, 66)
            }
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

pub(super) fn configure_appearance(
    ctx: &egui::Context,
    mode: AppearanceMode,
    currently_applied: AppliedAppearance,
) -> (egui::Theme, AppliedAppearance, ApplicationPalette) {
    let system_theme = ctx
        .system_theme()
        .unwrap_or_else(|| ctx.options(|options| options.fallback_theme));
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

    if currently_applied != applied {
        ctx.set_visuals_of(egui::Theme::Light, light.clone());
        ctx.set_visuals_of(egui::Theme::Dark, dark.clone());
        ctx.set_theme(mode.theme_preference());
    }

    (resolved_theme, applied, palette)
}

#[cfg(test)]
mod tests {
    use super::{ApplicationPalette, ResolvedAppearance};
    use eframe::egui::{self, Color32};

    #[test]
    fn dual_dialogue_boundary_color_is_theme_owned_and_subdued_per_appearance() {
        for (appearance, expected) in [
            (ResolvedAppearance::Day, Color32::from_rgb(142, 153, 163)),
            (ResolvedAppearance::Night, Color32::from_rgb(143, 153, 163)),
            (ResolvedAppearance::Midnight, Color32::from_rgb(124, 135, 146)),
        ] {
            let palette = ApplicationPalette::from_visuals(
                appearance,
                &super::visuals_for_appearance(appearance),
            );
            assert_eq!(palette.script_workspace.page.dual_dialogue_boundary, expected);
            assert_ne!(
                palette.script_workspace.page.dual_dialogue_boundary,
                palette.script_workspace.page.selection_boundary,
            );
        }
    }

    #[test]
    fn toolbar_selected_fill_has_explicit_values_and_appearance_polarity() {
        for (appearance, expected_selected, expected_hover) in [
            (
                ResolvedAppearance::Day,
                Color32::from_rgb(214, 214, 214),
                Color32::from_rgb(220, 220, 220),
            ),
            (
                ResolvedAppearance::Night,
                Color32::from_rgb(66, 66, 66),
                Color32::from_rgb(72, 72, 72),
            ),
            (
                ResolvedAppearance::Midnight,
                Color32::from_rgb(66, 66, 66),
                Color32::from_rgb(68, 68, 68),
            ),
        ] {
            let visuals = super::visuals_for_appearance(appearance);
            let palette = ApplicationPalette::from_visuals(appearance, &visuals);
            let toolbar = palette.toolbar;
            assert_eq!(toolbar.selected_fill, expected_selected);
            assert_eq!(toolbar.hover_fill, expected_hover);
            assert_ne!(toolbar.selected_fill, toolbar.hover_fill);
            assert_eq!(
                palette.controls.selection_fill,
                visuals.selection.bg_fill,
                "the toolbar-only token does not replace generic control selection",
            );
            assert_eq!(
                palette.status.slider_trailing_fill,
                visuals.selection.bg_fill,
                "the toolbar-only token does not recolor the status slider",
            );
            match appearance {
                ResolvedAppearance::Day => {
                    assert!(toolbar.selected_fill.r() < toolbar.background.r());
                }
                ResolvedAppearance::Night | ResolvedAppearance::Midnight => {
                    assert!(toolbar.selected_fill.r() > toolbar.background.r());
                }
            }
        }
    }
}
