pub(crate) mod inspector;
pub(crate) mod selector;
pub(crate) mod state;
pub(crate) mod summary;
mod presentation;

use crate::{app::ScriptCraftApp, commands::CommandInvocation};
use eframe::egui;
use scriptcraft_commands::{builtin_template_value, Command, CommandOrigin};
use state::ElementsTab;

/// Focuses the currently topmost native Elements viewport without repeatedly
/// raising a viewport that already owns native focus.
pub(crate) fn focus_topmost_element_viewport(app: &ScriptCraftApp, ctx: &egui::Context) {
    focus_topmost_element_viewport_for_state(
        ctx,
        app.element_windows.selector_open(),
        app.element_windows.elements_open(),
    );
}

/// Shared target resolver used by the app boundary and native inspector callback.
pub(crate) fn focus_topmost_element_viewport_for_state(
    ctx: &egui::Context,
    selector_open: bool,
    elements_open: bool,
) {
    let target = if selector_open {
        Some(selector::viewport_id())
    } else if elements_open {
        Some(inspector::viewport_id())
    } else {
        None
    };
    if let Some(target) = target {
        if ctx.input_for(target, |input| input.viewport().focused) != Some(true) {
            ctx.send_viewport_cmd_to(target, egui::ViewportCommand::Focus);
        }
    }
}

/// Composes the live Elements projection and dispatches one deferred window intent.
pub(crate) fn show_elements_windows(app: &mut ScriptCraftApp, ctx: &egui::Context) {
    if app.element_windows.take_native_close_request() {
        selector::send_close(ctx);
        inspector::send_close(ctx);
    }

    if !app.element_windows.elements_open() && !app.element_windows.selector_open() {
        return;
    }

    let selector_was_open = app.element_windows.selector_open();
    let elements_focus_requested = app.element_windows.take_elements_focus_request();
    let inspector_intent = if app.element_windows.elements_open() {
        let summaries = summary::element_definition_summaries(
            app.session.project(),
            crate::chrome::shortcuts::ShortcutPlatform::current(),
        );
        let selected_key = app
            .element_windows
            .selected_element()
            .filter(|selected| summaries.iter().any(|item| &item.key == *selected))
            .cloned()
            .or_else(|| summaries.first().map(|summary| summary.key.clone()));
        app.element_windows.select_element(selected_key.clone());
        let selected_summary = selected_key
            .as_ref()
            .and_then(|selected| summaries.iter().find(|summary| &summary.key == selected));
        match inspector::show_elements_window(
            ctx,
            &summaries,
            selected_key.as_ref(),
            selected_summary,
            app.element_windows.active_tab(),
            selector_was_open,
            elements_focus_requested && !selector_was_open,
            app.appearance_mode,
            app.appearance_applications
                .get(crate::chrome::theme::AppearanceContext::Elements),
        ) {
            Ok((intent, applied)) => {
                app.appearance_applications.set(
                    crate::chrome::theme::AppearanceContext::Elements,
                    applied,
                );
                if elements_focus_requested && !selector_was_open {
                    focus_topmost_element_viewport(app, ctx);
                }
                intent
            }
            Err(()) => {
                app.handle_unsupported_element_viewport(false, ctx);
                return;
            }
        }
    } else {
        None
    };

    if selector_was_open {
        let selector_focus_requested = app.element_windows.take_selector_focus_request();
        let selector_state = app
            .element_windows
            .selector()
            .expect("selector remains open during root composition");
        let selector_outcome = selector::show_template_selector(
            ctx,
            selector_state.mode(),
            selector_state.selected_template(),
            selector_state.error(),
            selector_focus_requested,
            app.appearance_mode,
            app.appearance_applications
                .get(crate::chrome::theme::AppearanceContext::TemplateSelector),
        );
        let selector_intent = match selector_outcome {
            Ok((intent, applied)) => {
                app.appearance_applications.set(
                    crate::chrome::theme::AppearanceContext::TemplateSelector,
                    applied,
                );
                if selector_focus_requested {
                    focus_topmost_element_viewport(app, ctx);
                }
                intent
            }
            Err(()) => {
                app.handle_unsupported_element_viewport(true, ctx);
                return;
            }
        };
        match selector_intent {
            Some(selector::TemplateSelectorIntent::SelectionChanged(id)) => {
                app.element_windows.select_template(id);
            }
            Some(selector::TemplateSelectorIntent::Cancel) => {
                dispatch_window_command(app, Command::FormatTemplateSelectorCancel, ctx);
            }
            Some(selector::TemplateSelectorIntent::Apply(id)) => {
                app.element_windows.select_template(id);
                dispatch_window_command(
                    app,
                    Command::FormatTemplateApply {
                        template: builtin_template_value(id),
                    },
                    ctx,
                );
            }
            Some(selector::TemplateSelectorIntent::New(id)) => {
                dispatch_window_command(
                    app,
                    Command::FileNewFromTemplate {
                        template: builtin_template_value(id),
                    },
                    ctx,
                );
            }
            None => {}
        }
        // The selector owns the frame's terminal interaction. An underlying
        // Elements title close is ignored until the selector has returned.
    } else {
        let command = match inspector_intent {
            Some(inspector::ElementsWindowIntent::SelectElement(key)) => {
                app.element_windows.select_element(Some(key));
                None
            }
            Some(inspector::ElementsWindowIntent::SelectTab(tab)) => {
                app.element_windows.select_tab(tab);
                None
            }
            Some(inspector::ElementsWindowIntent::OpenTemplateSelector) => {
                Some(Command::FormatTemplateSelectorOpen)
            }
            Some(inspector::ElementsWindowIntent::CloseElements) => {
                Some(Command::FormatElementsClose)
            }
            None => None,
        };
        if let Some(command) = command {
            dispatch_window_command(app, command, ctx);
        }
    }
}

fn dispatch_window_command(app: &mut ScriptCraftApp, command: Command, ctx: &egui::Context) {
    app.dispatch_command(
        CommandInvocation::new(command, CommandOrigin::InternalFollowUp),
        ctx,
    );
}
