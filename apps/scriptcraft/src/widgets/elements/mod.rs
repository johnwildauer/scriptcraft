pub(crate) mod inspector;
pub(crate) mod selector;
pub(crate) mod state;
pub(crate) mod summary;


use crate::{app::ScriptCraftApp, commands::CommandInvocation};
use eframe::egui;
use scriptcraft_commands::{builtin_template_value, Command, CommandOrigin};

/// Composes the live Elements projection and dispatches one deferred window intent.
pub(crate) fn show_elements_windows(app: &mut ScriptCraftApp, ctx: &egui::Context) {
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
        inspector::show_elements_window(
            ctx,
            &summaries,
            selector_was_open,
            elements_focus_requested && !selector_was_open,
        )
    } else {
        None
    };

    if selector_was_open {
        let selector_focus_requested = app.element_windows.take_selector_focus_request();
        let _ = selector_focus_requested;
        let selector_state = app
            .element_windows
            .selector()
            .expect("selector remains open during root composition");
        let selector_intent = selector::show_template_selector(
            ctx,
            selector_state.mode(),
            selector_state.selected_template(),
            selector_state.error(),
            selector_focus_requested,
        );
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
