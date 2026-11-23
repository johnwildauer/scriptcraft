//! Title Page text and structure operations.

use super::history::Snapshot;
use super::selection::{validate_title_position, validate_title_selection, SelectionOrdering};
use super::text::{
    apply_typing_styles, coalesce_runs, effective_title_runs, next_boundary, previous_boundary,
    push_run, slice_runs, title_run_attributes_at,
};
use super::{validate_full_candidate, EditError, EditingSession, Position, Selection};
use scriptcraft_model::formatted_text::{
    project_style_at_caret, RunBoundaryAffinity, RunStyleFlags,
};
use scriptcraft_model::preservation::PreservationError;
use scriptcraft_model::{
    ElementTypeKey, ModelError, Project, TextRun, TitlePageParagraph, TitlePageParagraphFormatting,
    MAX_TEXT_BYTES,
};

impl EditingSession {
    /// Changes the selected title paragraph's optional exact document type.
    /// Type-less title paragraphs remain addressable and can be assigned a
    /// registered key without changing their text or formatting.
    pub fn change_title_element_type(
        &mut self,
        element_type: Option<ElementTypeKey>,
    ) -> Result<(), EditError> {
        if let Some(key) = &element_type {
            if self.project.element_definitions().get(key).is_none() {
                return Err(EditError::Model(ModelError::MissingElementDefinition(
                    key.clone(),
                )));
            }
        }
        let (start, end) = self.title_selection.ordered();
        if start.element != end.element {
            return Err(EditError::InvalidSelection);
        }
        let current = self
            .project
            .title_page
            .get(start.element)
            .ok_or(EditError::InvalidElement)?
            .element_type();
        if current == element_type {
            return Ok(());
        }

        self.mutate_title(|project, selection, _typing| {
            let paragraph = project
                .title_page
                .get(start.element)
                .cloned()
                .ok_or(EditError::InvalidElement)?;
            if paragraph.element_type() != current {
                return Err(EditError::Preservation(
                    PreservationError::UnsupportedRoleMapping,
                ));
            }
            if paragraph
                .source_provenance
                .as_ref()
                .and_then(|provenance| provenance.foreign_role.as_deref())
                != current.as_ref().map(ElementTypeKey::as_str)
                && paragraph.source_provenance.is_some()
            {
                return Err(EditError::Preservation(
                    PreservationError::UnsupportedRoleMapping,
                ));
            }
            if paragraph.source_provenance.is_some() {
                project
                    .preservation
                    .as_mut()
                    .ok_or(EditError::Preservation(
                        PreservationError::ManagedAnchorMissing,
                    ))?
                    .change_managed_title_element_type(
                        &paragraph,
                        current.as_ref(),
                        element_type.as_ref(),
                    )
                    .map_err(EditError::Preservation)?;
            }
            let updated = &mut project.title_page[start.element];
            updated.set_element_type(element_type.clone());
            if let Some(provenance) = &mut updated.source_provenance {
                provenance.foreign_role = element_type.as_ref().map(|key| key.as_str().to_owned());
            }
            Ok(*selection)
        })
    }

    /// Inserts or replaces canonical title text in one paragraph. Paste is an
    /// alias so one clipboard operation always creates one history entry.
    pub fn insert_title(&mut self, text: &str) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            let insertion_style = if start == end {
                *typing
            } else {
                let paragraph = project
                    .title_page
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                project_style_at_caret(
                    &paragraph.text,
                    &paragraph.runs,
                    start.offset,
                    RunBoundaryAffinity::Right,
                    *typing,
                )
                .map_err(EditError::FormattedText)?
            };
            replace_title_range(project, start, end, text, insertion_style)?;
            Ok(Selection::caret(Position {
                element: start.element,
                offset: start.offset.saturating_add(text.len()),
            }))
        })
    }

    pub fn paste_title(&mut self, text: &str) -> Result<(), EditError> {
        self.insert_title(text)
    }

    /// Deletes a title selection, or the next Unicode scalar at a collapsed
    /// caret. Structural source reconciliation is staged with the semantic edit.
    pub fn delete_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_title_range(project, start, end, "", *typing)?;
                return Ok(Selection::caret(start));
            }
            let paragraph_text = project
                .title_page
                .get(start.element)
                .ok_or(EditError::InvalidElement)?
                .text
                .clone();
            if paragraph_text.is_empty() && project.title_page.len() > 1 {
                project.title_page.remove(start.element);
                let caret = if start.element < project.title_page.len() {
                    Position {
                        element: start.element,
                        offset: 0,
                    }
                } else {
                    let previous = start.element.saturating_sub(1);
                    Position {
                        element: previous,
                        offset: project.title_page[previous].text.len(),
                    }
                };
                return Ok(Selection::caret(caret));
            }
            if start.offset == paragraph_text.len() {
                if start.element + 1 < project.title_page.len() {
                    join_title_at(project, start.element)?;
                }
                return Ok(*selection);
            }
            let next = next_boundary(&paragraph_text, start.offset);
            let finish = Position {
                offset: next,
                ..start
            };
            replace_title_range(project, start, finish, "", *typing)?;
            Ok(Selection::caret(start))
        })
    }

    pub fn backspace_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, typing| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_title_range(project, start, end, "", *typing)?;
                return Ok(Selection::caret(start));
            }
            let paragraph_text = project
                .title_page
                .get(start.element)
                .ok_or(EditError::InvalidElement)?
                .text
                .clone();
            if paragraph_text.is_empty() && project.title_page.len() > 1 {
                project.title_page.remove(start.element);
                let caret = if start.element > 0 {
                    let previous = start.element - 1;
                    Position {
                        element: previous,
                        offset: project.title_page[previous].text.len(),
                    }
                } else {
                    Position {
                        element: 0,
                        offset: 0,
                    }
                };
                return Ok(Selection::caret(caret));
            }
            if start.offset == 0 {
                if start.element > 0 {
                    let left = start.element - 1;
                    let offset = project.title_page[left].text.len();
                    join_title_at(project, left)?;
                    return Ok(Selection::caret(Position {
                        element: left,
                        offset,
                    }));
                }
                return Ok(*selection);
            }
            let previous = previous_boundary(&paragraph_text, start.offset);
            let from = Position {
                offset: previous,
                ..start
            };
            replace_title_range(project, from, start, "", *typing)?;
            Ok(Selection::caret(from))
        })
    }

    pub fn delete_forward_title(&mut self) -> Result<(), EditError> {
        self.delete_title()
    }

    /// Title Enter inserts a new role-less paragraph. Source-backed structure
    /// is reconciled transactionally against its preserved TitlePage Content.
    pub fn split_title(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, _typing| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            let original = project
                .title_page
                .get(start.element)
                .cloned()
                .ok_or(EditError::InvalidElement)?;
            let id = project.allocate_element_id()?;
            let source_runs = effective_title_runs(&original);
            let original_len = original.text.len();
            let mut left = original.clone();
            let mut right = original;
            left.text = left.text[..start.offset].to_owned();
            right.id = id;
            right.text = right.text[start.offset..].to_owned();
            left.runs = slice_runs(&source_runs, 0, start.offset);
            right.runs = slice_runs(&source_runs, start.offset, original_len);
            right.source_provenance = None;
            project.title_page[start.element] = left;
            project.title_page.insert(start.element + 1, right);
            Ok(Selection::caret(Position {
                element: start.element + 1,
                offset: 0,
            }))
        })
    }

    pub fn enter_title(&mut self) -> Result<(), EditError> {
        self.split_title()
    }

    /// Title Tab is consumed without inserting a literal tab or changing
    /// screenplay roles.
    pub fn tab_title(&self) -> Result<(), EditError> {
        validate_title_selection(&self.project, self.title_selection)
    }

    /// Joins the title paragraph before the caret into the paragraph at the
    /// caret, retaining the preceding stable identity. Source-backed structure
    /// either reconciles wholly or leaves the live session unchanged.
    pub fn join_title_with_previous(&mut self) -> Result<(), EditError> {
        self.mutate_title(|project, selection, _typing| {
            let (start, end) = selection.ordered();
            if start != end || start.offset != 0 || start.element == 0 {
                return Err(EditError::InvalidSelection);
            }
            let left_index = start.element - 1;
            let offset = project.title_page[left_index].text.len();
            join_title_at(project, left_index)?;
            Ok(Selection::caret(Position {
                element: left_index,
                offset,
            }))
        })
    }

    pub(super) fn mutate_title<F>(&mut self, operation: F) -> Result<(), EditError>
    where
        F: FnOnce(&mut Project, &Selection, &mut RunStyleFlags) -> Result<Selection, EditError>,
    {
        let mut project = self.project.clone();
        let before_title = self.project.title_page.clone();
        let mut typing = self.title_typing_style;
        let selection = operation(&mut project, &self.title_selection, &mut typing)?;
        let before_ids = before_title
            .iter()
            .map(|paragraph| &paragraph.id)
            .collect::<Vec<_>>();
        let after_ids = project
            .title_page
            .iter()
            .map(|paragraph| &paragraph.id)
            .collect::<Vec<_>>();
        if before_ids != after_ids {
            if let Some(preservation) = project.preservation.as_mut() {
                preservation
                    .reconcile_title_structure(&before_title, &mut project.title_page)
                    .map_err(EditError::Preservation)?;
            } else if before_title
                .iter()
                .any(|paragraph| paragraph.source_provenance.is_some())
                || project
                    .title_page
                    .iter()
                    .any(|paragraph| paragraph.source_provenance.is_some())
            {
                return Err(EditError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        }
        if let Some(preservation) = project.preservation.as_mut() {
            preservation.ensure_source_identity_use_index();
        }
        validate_full_candidate(&project)?;
        validate_title_selection(&project, selection)?;
        if project == self.project {
            let transient_changed =
                self.title_selection != selection || self.title_typing_style != typing;
            self.title_selection = selection;
            self.title_typing_style = typing;
            if transient_changed {
                self.mark_mutated();
            }
            return Ok(());
        }
        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            title_selection: self.title_selection,
            title_typing_style: self.title_typing_style,
            body_typing_style: self.body_typing_style,
            body_typing_formatting: self.body_typing_formatting,
            generated_parentheticals: self.generated_parentheticals.clone(),
        });
        self.project = project;
        self.title_selection = selection;
        self.title_typing_style = typing;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        self.validated_baseline = true;
        // Title-only edits cannot affect screenplay scene-derived attachments.
        self.mark_mutated();
        Ok(())
    }
}

fn unsupported_title_structure() -> EditError {
    EditError::Preservation(PreservationError::UnsupportedTextMapping)
}

fn join_title_at(project: &mut Project, left_index: usize) -> Result<(), EditError> {
    let right_index = left_index.checked_add(1).ok_or(EditError::InvalidElement)?;
    let right = project
        .title_page
        .get(right_index)
        .cloned()
        .ok_or(EditError::InvalidElement)?;
    let mut left = project
        .title_page
        .get(left_index)
        .cloned()
        .ok_or(EditError::InvalidElement)?;
    if left.text.len().saturating_add(right.text.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let mut runs = effective_title_runs(&left);
    runs.extend(effective_title_runs(&right));
    left.text.push_str(&right.text);
    coalesce_runs(&mut runs);
    left.runs = if left.text.is_empty() {
        Vec::new()
    } else {
        runs
    };
    project.title_page[left_index] = left;
    project.title_page.remove(right_index);
    Ok(())
}

fn replace_title_range(
    project: &mut Project,
    start: Position,
    end: Position,
    inserted: &str,
    typing: RunStyleFlags,
) -> Result<(), EditError> {
    validate_title_position(project, start)?;
    validate_title_position(project, end)?;
    if (start.element, start.offset) > (end.element, end.offset) {
        return Err(EditError::InvalidSelection);
    }
    if inserted.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if project.title_page.is_empty() {
        if start != end || project.preservation.is_some() {
            return Err(unsupported_title_structure());
        }
        let id = project.allocate_element_id()?;
        let mut paragraph = TitlePageParagraph::new(
            id.as_str(),
            inserted,
            TitlePageParagraphFormatting::default(),
        )?;
        if !inserted.is_empty() && typing != RunStyleFlags::default() {
            paragraph.runs.push(TextRun {
                text: inserted.to_owned(),
                attributes: apply_typing_styles(Vec::new(), typing),
            });
        }
        project.title_page.push(paragraph);
        return Ok(());
    }
    if start.element == end.element {
        let old = project.title_page[start.element].clone();
        let combined_len = old
            .text
            .len()
            .saturating_sub(end.offset - start.offset)
            .saturating_add(inserted.len());
        if combined_len > MAX_TEXT_BYTES {
            return Err(EditError::TextTooLong);
        }
        let has_runs = !old.runs.is_empty();
        let mut has_source_mapping = false;
        if let Some(preservation) = project.preservation.as_mut() {
            if preservation.has_managed_element(&old.id) {
                has_source_mapping = true;
                if has_runs {
                    preservation
                        .ensure_run_range_is_contiguous(&old.id, start.offset..end.offset)
                        .map_err(EditError::Preservation)?;
                }
                let expected = old
                    .runs
                    .iter()
                    .map(|run| run.text.clone())
                    .collect::<Vec<_>>();
                preservation
                    .replace_managed_text(
                        &old.id,
                        &old.text,
                        start.offset,
                        end.offset,
                        inserted,
                        has_runs.then_some(expected.as_slice()),
                    )
                    .map_err(EditError::Preservation)?;
            } else if old.source_provenance.is_some() {
                return Err(EditError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ));
            }
        } else if old.source_provenance.is_some() {
            return Err(EditError::Preservation(
                PreservationError::ManagedAnchorMissing,
            ));
        }
        let text = format!(
            "{}{}{}",
            &old.text[..start.offset],
            inserted,
            &old.text[end.offset..]
        );
        let mut updated = old;
        if has_runs {
            let effective = effective_title_runs(&updated);
            let mut runs = slice_runs(&effective, 0, start.offset);
            let inherited = title_run_attributes_at(&effective, start.offset, updated.text.len());
            push_run(&mut runs, inserted, apply_typing_styles(inherited, typing));
            runs.extend(slice_runs(&effective, end.offset, updated.text.len()));
            coalesce_runs(&mut runs);
            updated.runs = if text.is_empty() { Vec::new() } else { runs };
        } else if !inserted.is_empty() && typing != RunStyleFlags::default() {
            let effective = effective_title_runs(&updated);
            let mut runs = slice_runs(&effective, 0, start.offset);
            push_run(
                &mut runs,
                inserted,
                apply_typing_styles(
                    title_run_attributes_at(&effective, start.offset, updated.text.len()),
                    typing,
                ),
            );
            runs.extend(slice_runs(&effective, end.offset, updated.text.len()));
            coalesce_runs(&mut runs);
            updated.runs = runs;
        }
        updated.text = text;
        if has_source_mapping {
            let sync_text = updated.text.clone();
            let sync_runs = updated.runs.clone();
            project
                .preservation
                .as_mut()
                .ok_or(EditError::Preservation(
                    PreservationError::ManagedAnchorMissing,
                ))?
                .sync_managed_text_runs(&updated.id, &sync_text, &sync_runs)
                .map_err(EditError::Preservation)?;
        }
        project.title_page[start.element] = updated;
        return Ok(());
    }

    // Structural replacement is available for detached/new documents only.
    let left = project.title_page[start.element].clone();
    let right = project.title_page[end.element].clone();
    let text = format!(
        "{}{}{}",
        &left.text[..start.offset],
        inserted,
        &right.text[end.offset..]
    );
    if text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let mut runs = slice_runs(&effective_title_runs(&left), 0, start.offset);
    let insertion_attributes =
        title_run_attributes_at(&effective_title_runs(&left), start.offset, left.text.len());
    push_run(
        &mut runs,
        inserted,
        apply_typing_styles(insertion_attributes, typing),
    );
    runs.extend(slice_runs(
        &effective_title_runs(&right),
        end.offset,
        right.text.len(),
    ));
    coalesce_runs(&mut runs);
    let mut replacement = left;
    replacement.text = text;
    replacement.runs = runs;
    project.title_page[start.element] = replacement;
    project.title_page.drain(start.element + 1..=end.element);
    Ok(())
}
