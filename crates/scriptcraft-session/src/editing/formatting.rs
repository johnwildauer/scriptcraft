//! Surface-aware run formatting and paragraph-alignment operations.

use super::candidate::MutationValidationScope;
use super::selection::{validate_selection, validate_title_selection, SelectionOrdering};
use super::{EditError, EditingSession, Selection};
use scriptcraft_model::formatted_text::{
    clear_paragraph_run_styles, project_style_at_caret, set_paragraph_alignments,
    toggle_paragraph_run_styles, ParagraphTarget, RunBoundaryAffinity, RunStyle, RunStyleFlags,
};
use scriptcraft_model::formatting::ParagraphAlignment;
use scriptcraft_model::Project;

impl EditingSession {
    /// Applies a run-style toggle to the body selection through the same
    /// canonical range engine used by title text.
    pub fn toggle_body_run_style(&mut self, style: RunStyle) -> Result<(), EditError> {
        self.toggle_surface_run_style(false, style)
    }

    pub fn toggle_title_run_style(&mut self, style: RunStyle) -> Result<(), EditError> {
        self.toggle_surface_run_style(true, style)
    }

    /// Clears all supported run formatting from the current Body selection in
    /// one transactional edit. Alignment and unknown source style tokens remain.
    pub fn clear_body_run_styles(&mut self) -> Result<(), EditError> {
        self.clear_surface_run_styles(false)
    }

    /// Clears all supported run formatting from the current Title Page selection
    /// in one transactional edit. Alignment and unknown source style tokens remain.
    pub fn clear_title_run_styles(&mut self) -> Result<(), EditError> {
        self.clear_surface_run_styles(true)
    }

    pub fn set_body_alignment(&mut self, alignment: ParagraphAlignment) -> Result<(), EditError> {
        self.set_surface_alignment(false, alignment)
    }

    pub fn set_title_alignment(&mut self, alignment: ParagraphAlignment) -> Result<(), EditError> {
        self.set_surface_alignment(true, alignment)
    }

    fn toggle_surface_run_style(
        &mut self,
        title_surface: bool,
        style: RunStyle,
    ) -> Result<(), EditError> {
        let selection = if title_surface {
            self.title_selection
        } else {
            self.selection
        };
        if selection.anchor == selection.focus {
            let position = selection.focus;
            let (text, runs, active) = if title_surface {
                let paragraph = self
                    .project
                    .title_page
                    .get(position.element)
                    .ok_or(EditError::InvalidElement)?;
                (
                    paragraph.text.as_str(),
                    paragraph.runs.as_slice(),
                    self.title_typing_style,
                )
            } else {
                let paragraph = self
                    .project
                    .elements
                    .get(position.element)
                    .ok_or(EditError::InvalidElement)?;
                (
                    paragraph.text.as_str(),
                    paragraph.runs.as_slice(),
                    self.body_typing_style,
                )
            };
            let current = project_style_at_caret(
                text,
                runs,
                position.offset,
                RunBoundaryAffinity::Right,
                active,
            )
            .map_err(EditError::FormattedText)?;
            let next = current.toggled(style);
            if title_surface {
                self.title_typing_style = next;
            } else {
                self.body_typing_style = next;
            }
            self.mark_mutated();
            return Ok(());
        }

        let active = if title_surface {
            self.title_typing_style
        } else {
            self.body_typing_style
        };
        let ranges = selection_run_ranges(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, _| {
                toggle_paragraph_run_styles(project, &ranges, active, style)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                toggle_paragraph_run_styles(project, &ranges, active, style)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        }
    }

    fn clear_surface_run_styles(&mut self, title_surface: bool) -> Result<(), EditError> {
        let selection = if title_surface {
            self.title_selection
        } else {
            self.selection
        };
        if selection.anchor == selection.focus {
            if title_surface {
                self.project
                    .title_page
                    .get(selection.focus.element)
                    .ok_or(EditError::InvalidElement)?;
                let changed = self.title_typing_style != RunStyleFlags::default();
                self.title_typing_style = RunStyleFlags::default();
                if changed {
                    self.mark_mutated();
                }
            } else {
                self.project
                    .elements
                    .get(selection.focus.element)
                    .ok_or(EditError::InvalidElement)?;
                let changed = self.body_typing_style != RunStyleFlags::default();
                self.body_typing_style = RunStyleFlags::default();
                if changed {
                    self.mark_mutated();
                }
            }
            return Ok(());
        }

        let ranges = selection_run_ranges(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, typing| {
                clear_paragraph_run_styles(project, &ranges).map_err(EditError::FormattedText)?;
                *typing = RunStyleFlags::default();
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                clear_paragraph_run_styles(project, &ranges).map_err(EditError::FormattedText)?;
                Ok(selection)
            })?;
            let typing_changed = self.body_typing_style != RunStyleFlags::default();
            self.body_typing_style = RunStyleFlags::default();
            if typing_changed {
                self.mark_mutated();
            }
            Ok(())
        }
    }

    fn set_surface_alignment(
        &mut self,
        title_surface: bool,
        alignment: ParagraphAlignment,
    ) -> Result<(), EditError> {
        let selection = if title_surface {
            self.title_selection
        } else {
            self.selection
        };
        let targets = selection_paragraph_targets(&self.project, selection, title_surface)?;
        if title_surface {
            self.mutate_title(|project, _, _| {
                set_paragraph_alignments(project, &targets, alignment)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        } else {
            self.mutate(MutationValidationScope::Complete, |project, _, _, _| {
                set_paragraph_alignments(project, &targets, alignment)
                    .map_err(EditError::FormattedText)?;
                Ok(selection)
            })
        }
    }
}

fn selection_run_ranges(
    project: &Project,
    selection: Selection,
    title_surface: bool,
) -> Result<Vec<(ParagraphTarget, std::ops::Range<usize>)>, EditError> {
    if title_surface {
        validate_title_selection(project, selection)?;
    } else {
        validate_selection(project, selection)?;
    }
    let (start, end) = selection.ordered();
    let mut ranges = Vec::new();
    for index in start.element..=end.element {
        let (text_len, target) = if title_surface {
            let paragraph = project
                .title_page
                .get(index)
                .ok_or(EditError::InvalidElement)?;
            (paragraph.text.len(), ParagraphTarget::TitlePage(index))
        } else {
            let paragraph = project
                .elements
                .get(index)
                .ok_or(EditError::InvalidElement)?;
            (paragraph.text.len(), ParagraphTarget::Body(index))
        };
        let from = if index == start.element {
            start.offset
        } else {
            0
        };
        let to = if index == end.element {
            end.offset
        } else {
            text_len
        };
        ranges.push((target, from..to));
    }
    Ok(ranges)
}

fn selection_paragraph_targets(
    project: &Project,
    selection: Selection,
    title_surface: bool,
) -> Result<Vec<ParagraphTarget>, EditError> {
    if title_surface {
        validate_title_selection(project, selection)?;
    } else {
        validate_selection(project, selection)?;
    }
    let (start, end) = selection.ordered();
    let last = if start.element < end.element && end.offset == 0 {
        end.element - 1
    } else {
        end.element
    };
    (start.element..=last)
        .map(|index| {
            if title_surface {
                project
                    .title_page
                    .get(index)
                    .ok_or(EditError::InvalidElement)?;
                Ok(ParagraphTarget::TitlePage(index))
            } else {
                project
                    .elements
                    .get(index)
                    .ok_or(EditError::InvalidElement)?;
                Ok(ParagraphTarget::Body(index))
            }
        })
        .collect()
}
