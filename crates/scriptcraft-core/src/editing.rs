//! Headless, transactional editing over the semantic screenplay model.

use crate::{
    cast_list::{self, CastListError},
    ModelError, Project, Role, ScreenplayElement, TextRun, MAX_ELEMENTS, MAX_TEXT_BYTES,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub element: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    pub const fn caret(position: Position) -> Self {
        Self {
            anchor: position,
            focus: position,
        }
    }

    fn ordered(self) -> (Position, Position) {
        if (self.anchor.element, self.anchor.offset) <= (self.focus.element, self.focus.offset) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }
}

/// Ephemeral editing behavior. This is session/application input, not document
/// state and is intentionally absent from snapshots and native serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditPolicy {
    pub uppercase_cues: bool,
    pub auto_parenthetical: bool,
    pub strict_transition_boundaries: bool,
}

impl Default for EditPolicy {
    fn default() -> Self {
        Self {
            uppercase_cues: true,
            auto_parenthetical: true,
            strict_transition_boundaries: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidElement,
    InvalidUtf8Boundary,
    InvalidSelection,
    EmptyHistory,
    TextTooLong,
    TooManyElements,
    UnsupportedTransition { from: Role },
    Preservation(crate::preservation::PreservationError),
    CastList(CastListError),
    Model(ModelError),
}

impl From<ModelError> for EditError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    // Project includes semantic runs and the complete preservation graph, so
    // undo/redo restores both authorities rather than only the text projection.
    project: Project,
    selection: Selection,
    generated_parentheticals: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditingSession {
    project: Project,
    policy: EditPolicy,
    selection: Selection,
    dirty: bool,
    revision: u64,
    clean_project: Project,
    history: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    generated_parentheticals: BTreeSet<String>,
    history_limit: usize,
}

impl EditingSession {
    pub fn new(project: Project) -> Result<Self, EditError> {
        Self::with_policy(project, EditPolicy::default())
    }

    pub fn with_policy(project: Project, policy: EditPolicy) -> Result<Self, EditError> {
        project.validate()?;
        let selection = Selection::caret(Position {
            element: 0,
            offset: 0,
        });
        validate_selection(&project, selection)?;
        Ok(Self {
            clean_project: project.clone(),
            project,
            policy,
            selection,
            dirty: false,
            revision: 0,
            history: Vec::new(),
            redo: Vec::new(),
            generated_parentheticals: BTreeSet::new(),
            history_limit: 100,
        })
    }

    pub fn with_history_limit(project: Project, history_limit: usize) -> Result<Self, EditError> {
        let mut session = Self::new(project)?;
        session.history_limit = history_limit;
        Ok(session)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }
    /// Current ephemeral editing policy. Undo/redo and document replacement do
    /// not restore or alter it; a newly constructed session uses defaults.
    pub const fn policy(&self) -> EditPolicy {
        self.policy
    }
    /// Changes runtime policy without altering document contents, revision,
    /// history, or clean/dirty state.
    pub fn set_policy(&mut self, policy: EditPolicy) {
        self.policy = policy;
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub const fn dirty(&self) -> bool {
        self.dirty
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Revalidates the live semantic project and selection without mutating it.
    pub fn validate(&self) -> Result<(), EditError> {
        self.project.validate()?;
        validate_selection(&self.project, self.selection)
    }

    pub fn mark_saved(&mut self) {
        self.clean_project = self.project.clone();
        self.dirty = false;
    }

    /// Refreshes derived cast lists without adding a separate undo entry.
    /// Selection is unchanged; dirty state and revision follow the corrected
    /// project so cached layout and the clean baseline remain accurate.
    pub fn refresh_cast_lists(&mut self) -> Result<bool, EditError> {
        let mut project = self.project.clone();
        let changed = cast_list::correct_cast_lists(&mut project).map_err(EditError::CastList)?;
        if changed {
            self.project = project;
            self.revision = self.revision.saturating_add(1);
            self.dirty = self.project != self.clean_project;
        }
        Ok(changed)
    }

    /// Builds corrected detached data for Save or Save As preflight. This does
    /// not mutate the live session; the storage owner controls publication.
    pub fn prepare_cast_list_save_candidate(&self) -> Result<Project, EditError> {
        cast_list::prepare_save_candidate(&self.project).map_err(EditError::CastList)
    }

    pub fn replace_with_saved(&mut self, project: Project) -> Result<(), EditError> {
        let replacement = Self::with_policy(project, self.policy)?;
        *self = replacement;
        Ok(())
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        validate_selection(&self.project, selection)?;
        self.selection = selection;
        Ok(())
    }

    /// Inserts or replaces text while retaining the existing element identity.
    /// Mapped source runs and text leaves are transformed in the same candidate;
    /// unsafe inline intersections reject the whole operation.
    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        self.mutate(|project, selection, _generated, policy| {
            let (start, end) = selection.ordered();
            replace_range(project, start, end, text)?;
            let offset =
                normalize_element(project, policy, start.element, start.offset + text.len())?;
            Ok(Selection::caret(Position {
                element: start.element,
                offset,
            }))
        })
    }

    /// Inserts clipboard text as one transactional semantic edit.
    ///
    /// Newline characters remain in the active element. Layout is responsible
    /// for visual line breaks; paste does not silently invent screenplay roles
    /// or multiple undo entries.
    pub fn insert_paste(&mut self, text: &str) -> Result<(), EditError> {
        self.insert(text)
    }

    /// Deletes text without removing an element identity. Operations which
    /// would remove an anchored paragraph instead fail until its source identity
    /// and attachments have a verified transform.
    pub fn delete(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, generated, _policy| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_range(project, start, end, "")?;
                return Ok(Selection::caret(start));
            }
            let item = project
                .elements
                .get(start.element)
                .ok_or(EditError::InvalidElement)?;
            if item.role == Role::Parenthetical
                && is_delimiter_only_parenthetical(item.text.as_str())
            {
                ensure_elements_unanchored(project, [item.id.clone()])?;
                let id = item.id.as_str().to_owned();
                generated.remove(&id);
                if start.element == 0 || project.elements.len() == 1 {
                    project.elements[0].role = Role::Dialogue;
                    project.elements[0].text.clear();
                    return Ok(Selection::caret(Position {
                        element: 0,
                        offset: 0,
                    }));
                }
                let previous = start.element.saturating_sub(1);
                let offset = project.elements[previous].text.len();
                project.elements.remove(start.element);
                return Ok(Selection::caret(Position {
                    element: previous,
                    offset,
                }));
            }
            if start.offset < item.text.len() {
                let next = next_boundary(&item.text, start.offset);
                replace_range(
                    project,
                    start,
                    Position {
                        offset: next,
                        ..start
                    },
                    "",
                )?;
            }
            Ok(Selection::caret(start))
        })
    }

    pub fn backspace(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, _generated, _policy| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_range(project, start, end, "")?;
                return Ok(Selection::caret(start));
            }
            if start.offset > 0 {
                let item = project
                    .elements
                    .get(start.element)
                    .ok_or(EditError::InvalidElement)?;
                let previous = previous_boundary(&item.text, start.offset);
                let from = Position {
                    offset: previous,
                    ..start
                };
                replace_range(project, from, start, "")?;
                return Ok(Selection::caret(from));
            }
            if start.element > 0 {
                let previous = project
                    .elements
                    .get(start.element - 1)
                    .ok_or(EditError::InvalidElement)?;
                let offset = previous.text.len();
                join_at(project, start.element - 1)?;
                return Ok(Selection::caret(Position {
                    element: start.element - 1,
                    offset,
                }));
            }
            Ok(Selection::caret(start))
        })
    }

    /// Splits an unanchored semantic element. A source-owned paragraph is not
    /// split until its foreign ID, anchor placement and nested data can be
    /// transformed without duplication or loss.
    pub fn split(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, _generated, policy| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            split_at(project, policy, start, project.elements[start.element].role)
        })
    }

    pub fn enter(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, generated, policy| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            let item = project
                .elements
                .get(start.element)
                .ok_or(EditError::InvalidElement)?;
            if strict_boundary_rejects(
                project,
                policy,
                start.element,
                item.role,
                item.text.len(),
                start.offset,
            ) {
                return Ok(*selection);
            }
            let item_role = item.role;
            let item_text = item.text.clone();
            let mut position = start;
            if item_role == Role::Parenthetical {
                if position.offset == 0 && item_text.starts_with('(') {
                    let id = split_id(project)?;
                    let new = ScreenplayElement::new(
                        id.clone(),
                        Role::Parenthetical,
                        if policy.auto_parenthetical { "()" } else { "" },
                    )?;
                    project.elements.insert(position.element, new);
                    if policy.auto_parenthetical {
                        generated.insert(id);
                        return Ok(Selection::caret(Position {
                            element: position.element,
                            offset: 1,
                        }));
                    }
                    return Ok(Selection::caret(Position {
                        element: position.element,
                        offset: 0,
                    }));
                }
                if empty_parenthetical_at(item_text.as_str(), position.offset)
                    && generated.contains(project.elements[position.element].id.as_str())
                {
                    project.elements[position.element].text.clear();
                    position.offset = 0;
                    generated.remove(project.elements[position.element].id.as_str());
                } else if let Some(boundary) =
                    parenthetical_repair_boundary(item_text.as_str(), position.offset)
                {
                    project.elements[position.element].text =
                        repair_parenthetical(item_text.as_str());
                    position.offset = boundary;
                    return insert_dialogue_below(project, position.element);
                } else if !item_text.is_empty() {
                    return Ok(*selection);
                }
            }
            let role = project.elements[position.element].role;
            let next_role = if role == Role::Dialogue && position.offset < item_text.len() {
                Role::Dialogue
            } else {
                enter_transition(role).ok_or(EditError::UnsupportedTransition { from: role })?
            };
            split_at(project, policy, position, next_role)
        })
    }

    /// Applies the explicit Tab policy. Unlisted roles are intentionally consumed
    /// without changing the project, selection, or history.
    pub fn tab(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, generated, policy| {
            let (start, end) = selection.ordered();
            if start != end {
                return Ok(*selection);
            }
            let item = project
                .elements
                .get(start.element)
                .ok_or(EditError::InvalidElement)?;
            if strict_boundary_rejects(
                project,
                policy,
                start.element,
                item.role,
                item.text.len(),
                start.offset,
            ) {
                return Ok(*selection);
            }
            let item_role = item.role;
            let item_text = item.text.clone();
            let mut position = start;
            if policy.auto_parenthetical && item_role == Role::Parenthetical {
                if is_delimiter_only_parenthetical(item_text.as_str()) {
                    project.elements[position.element].role = Role::Dialogue;
                    project.elements[position.element].text.clear();
                    generated.remove(project.elements[position.element].id.as_str());
                    position.offset = 0;
                    return Ok(Selection::caret(position));
                }
                if empty_parenthetical_at(item_text.as_str(), position.offset)
                    && generated.contains(project.elements[position.element].id.as_str())
                {
                    project.elements[position.element].text.clear();
                    position.offset = 0;
                    generated.remove(project.elements[position.element].id.as_str());
                } else if let Some(boundary) =
                    parenthetical_repair_boundary(item_text.as_str(), position.offset)
                {
                    project.elements[position.element].text =
                        repair_parenthetical(item_text.as_str());
                    position.offset = boundary;
                    return insert_dialogue_below(project, position.element);
                } else if !item_text.is_empty() {
                    return Ok(*selection);
                }
            }
            let element = &project.elements[position.element];
            let next_role = tab_transition(element.role, element.text.is_empty());
            if let Some(role) = next_role {
                project.elements[position.element].role = role;
                if policy.auto_parenthetical
                    && role == Role::Parenthetical
                    && project.elements[position.element].text.is_empty()
                {
                    project.elements[position.element].text = "()".into();
                    generated.insert(project.elements[position.element].id.as_str().to_owned());
                    return Ok(Selection::caret(Position {
                        element: position.element,
                        offset: 1,
                    }));
                }
                return normalize_selection_in_element(
                    project,
                    policy,
                    position.element,
                    Selection::caret(position),
                );
            }
            Ok(Selection::caret(position))
        })
    }

    /// Joins unanchored semantic elements. Mapped source identities are retained
    /// rather than merged or recycled; joining any anchored participant is a
    /// transactional safe-stop.
    pub fn join_with_next(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection, _generated, _policy| {
            let (start, end) = selection.ordered();
            if start != end || start.offset != project.elements[start.element].text.len() {
                return Err(EditError::InvalidSelection);
            }
            let offset = project.elements[start.element].text.len();
            join_at(project, start.element)?;
            Ok(Selection::caret(Position {
                element: start.element,
                offset,
            }))
        })
    }

    /// Changes a known role and its exact mapped source Type attribute together.
    /// A role change that would invalidate attached scene data is rejected.
    pub fn change_role(&mut self, role: Role) -> Result<(), EditError> {
        self.mutate(|project, selection, generated, policy| {
            let (start, end) = selection.ordered();
            if start.element != end.element {
                return Err(EditError::InvalidSelection);
            }
            let old_role = project.elements[start.element].role;
            if old_role != role {
                update_preserved_role(project, start.element, old_role, role)?;
            }
            project.elements[start.element].role = role;
            if role != Role::Parenthetical {
                generated.remove(project.elements[start.element].id.as_str());
            } else if policy.auto_parenthetical && project.elements[start.element].text.is_empty() {
                project.elements[start.element].text = "()".into();
                generated.insert(project.elements[start.element].id.as_str().to_owned());
                return Ok(Selection::caret(Position {
                    element: start.element,
                    offset: 1,
                }));
            }
            normalize_selection_in_element(project, policy, start.element, *selection)
        })
    }

    pub fn undo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self
            .history
            .last()
            .cloned()
            .ok_or(EditError::EmptyHistory)?;
        // Undo must remain available even when a derived list's source mapping
        // cannot be changed safely; retain that snapshot unchanged and let the
        // scheduled refresh/save preflight report the mapping limitation.
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.history.pop();
        self.redo.push(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), EditError> {
        let mut snapshot = self.redo.last().cloned().ok_or(EditError::EmptyHistory)?;
        let _ = cast_list::correct_cast_lists(&mut snapshot.project);
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        };
        self.redo.pop();
        self.push_history(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.generated_parentheticals = snapshot.generated_parentheticals;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        Ok(())
    }

    fn mutate<F>(&mut self, operation: F) -> Result<(), EditError>
    where
        F: FnOnce(
            &mut Project,
            &Selection,
            &mut BTreeSet<String>,
            &EditPolicy,
        ) -> Result<Selection, EditError>,
    {
        let mut project = self.project.clone();
        let mut generated = self.generated_parentheticals.clone();
        let prior_generated = generated.clone();
        let selection = operation(&mut project, &self.selection, &mut generated, &self.policy)?;
        invalidate_changed_generated_parentheticals(
            &self.project,
            &project,
            &prior_generated,
            &mut generated,
        );
        generated.retain(|id| {
            project
                .elements
                .iter()
                .any(|element| element.id.as_str() == id && element.role == Role::Parenthetical)
        });
        project.validate()?;
        validate_selection(&project, selection)?;
        if project == self.project && selection == self.selection {
            return Ok(());
        }
        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
            generated_parentheticals: self.generated_parentheticals.clone(),
        });
        self.project = project;
        self.selection = selection;
        self.generated_parentheticals = generated;
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        Ok(())
    }

    fn push_history(&mut self, snapshot: Snapshot) {
        if self.history_limit == 0 {
            return;
        }
        self.history.push(snapshot);
        if self.history.len() > self.history_limit {
            self.history.remove(0);
        }
    }
}

fn invalidate_changed_generated_parentheticals(
    before: &Project,
    after: &Project,
    prior_generated: &BTreeSet<String>,
    generated: &mut BTreeSet<String>,
) {
    for id in prior_generated {
        if !generated.contains(id) {
            continue;
        }
        let Some(before_element) = before
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        let Some(after_element) = after
            .elements
            .iter()
            .find(|element| element.id.as_str() == id)
        else {
            continue;
        };
        if before_element != after_element {
            generated.remove(id);
        }
    }
}

fn validate_position(project: &Project, position: Position) -> Result<(), EditError> {
    let element = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?;
    if element.is_boundary(position.offset) {
        Ok(())
    } else {
        Err(EditError::InvalidUtf8Boundary)
    }
}

fn validate_selection(project: &Project, selection: Selection) -> Result<(), EditError> {
    validate_position(project, selection.anchor)?;
    validate_position(project, selection.focus)
}

fn replace_range(
    project: &mut Project,
    start: Position,
    end: Position,
    inserted: &str,
) -> Result<(), EditError> {
    validate_position(project, start)?;
    validate_position(project, end)?;
    if (start.element, start.offset) > (end.element, end.offset) {
        return Err(EditError::InvalidSelection);
    }
    if inserted.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if start.element == end.element {
        replace_element_text(project, start.element, start.offset, end.offset, inserted)?;
        return Ok(());
    }
    // Cross-element replacement retains the first internal ID but removes the
    // other selected IDs. Until source anchors and foreign references can be
    // reconciled, every mapped participant must remain untouched.
    ensure_elements_unanchored(
        project,
        (start.element..=end.element).map(|index| project.elements[index].id.clone()),
    )?;
    let prefix = project.elements[start.element].text[..start.offset].to_owned();
    let suffix = project.elements[end.element].text[end.offset..].to_owned();
    let combined = format!("{prefix}{inserted}{suffix}");
    if combined.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let runs = replace_cross_element_runs(project, start, end, inserted, &combined)?;
    project.elements[start.element].text = combined;
    project.elements[start.element].runs = runs;
    project.elements.drain(start.element + 1..=end.element);
    Ok(())
}

fn replace_element_text(
    project: &mut Project,
    element_index: usize,
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<(), EditError> {
    let item = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let old_text = item.text.clone();
    let old_runs = item.runs.clone();
    let segments = if old_runs.is_empty() {
        vec![old_text.clone()]
    } else {
        old_runs.iter().map(|run| run.text.clone()).collect()
    };
    let updated_segments = replace_text_segments(&segments, start, end, inserted)?;
    let new_text = updated_segments.concat();
    if new_text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    let id = item.id.clone();
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        let expected_run_text = old_runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<_>>();
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ))?
            .replace_managed_text(
                &id,
                &old_text,
                start,
                end,
                inserted,
                (!old_runs.is_empty()).then_some(expected_run_text.as_slice()),
            )
            .map_err(EditError::Preservation)?;
    } else if item.source_provenance.is_some() {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::ManagedAnchorMissing,
        ));
    }
    let item = &mut project.elements[element_index];
    item.text = new_text;
    if !old_runs.is_empty() {
        for (run, text) in item.runs.iter_mut().zip(updated_segments) {
            run.text = text;
        }
    }
    Ok(())
}

fn replace_text_segments(
    segments: &[String],
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<Vec<String>, EditError> {
    let total = segments.iter().map(String::len).sum::<usize>();
    if start > end || end > total {
        return Err(EditError::InvalidSelection);
    }
    let mut insertion_segment = None;
    let mut base = 0usize;
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        if start < limit || (start == total && limit == total) {
            insertion_segment = Some(index);
            break;
        }
        base = limit;
    }
    let insertion_segment = insertion_segment
        .or_else(|| segments.len().checked_sub(1))
        .ok_or(EditError::InvalidSelection)?;
    let mut base = 0usize;
    let mut updated = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        let before_len = start.saturating_sub(base).min(segment.len());
        let after_start = end.saturating_sub(base).min(segment.len());
        let mut value = segment[..before_len].to_owned();
        if index == insertion_segment {
            value.push_str(inserted);
        }
        if end < limit || end <= base {
            value.push_str(&segment[after_start..]);
        }
        updated.push(value);
        base = limit;
    }
    Ok(updated)
}

fn ensure_elements_unanchored(
    project: &Project,
    ids: impl IntoIterator<Item = crate::StableId>,
) -> Result<(), EditError> {
    let ids = ids.into_iter().collect::<Vec<_>>();
    if project.elements.iter().any(|element| {
        ids.iter().any(|id| id == &element.id) && element.source_provenance.is_some()
    }) {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::StructuralAttachmentConflict,
        ));
    }
    if let Some(preservation) = &project.preservation {
        preservation
            .ensure_unanchored_elements(ids)
            .map_err(EditError::Preservation)?;
    }
    Ok(())
}

fn replace_cross_element_runs(
    project: &Project,
    start: Position,
    end: Position,
    inserted: &str,
    combined: &str,
) -> Result<Vec<TextRun>, EditError> {
    let left = &project.elements[start.element];
    let right = &project.elements[end.element];
    if left.runs.is_empty() && right.runs.is_empty() {
        return Ok(Vec::new());
    }
    let left_runs = effective_runs(left);
    let right_runs = effective_runs(right);
    let mut result = slice_runs(&left_runs, 0, start.offset);
    let insertion_attributes = run_attributes_at(&left_runs, start.offset);
    push_run(&mut result, inserted, insertion_attributes);
    let suffix_runs = if start.element == end.element {
        slice_runs(&left_runs, end.offset, left.text.len())
    } else {
        slice_runs(&right_runs, end.offset, right.text.len())
    };
    result.extend(suffix_runs);
    coalesce_runs(&mut result);
    let actual = result
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    if actual != combined {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::UnsupportedTextMapping,
        ));
    }
    Ok(result)
}

fn effective_runs(element: &ScreenplayElement) -> Vec<TextRun> {
    if element.runs.is_empty() {
        vec![TextRun {
            text: element.text.clone(),
            attributes: Vec::new(),
        }]
    } else {
        element.runs.clone()
    }
}

fn slice_runs(runs: &[TextRun], start: usize, end: usize) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        let from = start.saturating_sub(base).min(run.text.len());
        let to = end.saturating_sub(base).min(run.text.len());
        if from < to {
            result.push(TextRun {
                text: run.text[from..to].to_owned(),
                attributes: run.attributes.clone(),
            });
        }
        base = limit;
        if base >= end {
            break;
        }
    }
    result
}

fn run_attributes_at(runs: &[TextRun], offset: usize) -> Vec<crate::RunAttribute> {
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        if offset < limit || (offset == limit && limit == runs.iter().map(|r| r.text.len()).sum()) {
            return run.attributes.clone();
        }
        base = limit;
    }
    Vec::new()
}

fn push_run(runs: &mut Vec<TextRun>, text: &str, attributes: Vec<crate::RunAttribute>) {
    if text.is_empty() {
        return;
    }
    runs.push(TextRun {
        text: text.to_owned(),
        attributes,
    });
}

fn coalesce_runs(runs: &mut Vec<TextRun>) {
    let mut coalesced: Vec<TextRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(previous) = coalesced.last_mut() {
            if previous.attributes == run.attributes {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        coalesced.push(run);
    }
    *runs = coalesced;
}

fn update_preserved_role(
    project: &mut Project,
    element_index: usize,
    old_role: Role,
    new_role: Role,
) -> Result<(), EditError> {
    let element = project
        .elements
        .get(element_index)
        .ok_or(EditError::InvalidElement)?;
    let id = element.id.clone();
    let old_fdx_role = fdx_role_name(old_role);
    if element
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_role.as_deref())
        .is_some_and(|source_role| source_role != old_fdx_role)
    {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::UnsupportedRoleMapping,
        ));
    }
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ))?
            .change_managed_role(&id, old_fdx_role, fdx_role_name(new_role))
            .map_err(EditError::Preservation)?;
    } else if element.source_provenance.is_some() {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::ManagedAnchorMissing,
        ));
    }
    if let Some(provenance) = &mut project.elements[element_index].source_provenance {
        provenance.foreign_role = Some(fdx_role_name(new_role).to_owned());
    }
    Ok(())
}

fn fdx_role_name(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

fn strict_boundary_rejects(
    project: &Project,
    policy: &EditPolicy,
    element: usize,
    role: Role,
    text_len: usize,
    offset: usize,
) -> bool {
    policy.strict_transition_boundaries
        && matches!(
            role,
            Role::SceneHeading | Role::Character | Role::Transition
        )
        && project.elements.get(element).is_some()
        && offset != text_len
}

fn empty_parenthetical_at(text: &str, offset: usize) -> bool {
    text == "()" && offset <= 2
}

fn is_delimiter_only_parenthetical(text: &str) -> bool {
    matches!(text, "(" | "()" | ")")
}

fn parenthetical_repair_boundary(text: &str, offset: usize) -> Option<usize> {
    let closed_pair_inner_end = text.starts_with('(')
        && text.ends_with(')')
        && text.len() > 2
        && offset == text.len().saturating_sub(1);
    let accepted = closed_pair_inner_end
        || (text.starts_with('(') && !text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && text.ends_with(')') && text.len() > 1)
        || (!text.starts_with('(') && !text.ends_with(')') && !text.is_empty());
    let at_supported_boundary = closed_pair_inner_end || offset == text.len();
    if !accepted || !at_supported_boundary {
        return None;
    }
    Some(match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => text.len() + 1,
        (false, true) => text.len(),
        (false, false) => text.len() + 1,
        (true, true) => text.len(),
    })
}

fn repair_parenthetical(text: &str) -> String {
    match (text.starts_with('('), text.ends_with(')')) {
        (true, false) => format!("{text})"),
        (false, true) => format!("({text}"),
        (false, false) => format!("({text})"),
        (true, true) => text.to_owned(),
    }
}

fn insert_dialogue_below(project: &mut Project, element: usize) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let new = ScreenplayElement::new(split_id(project)?, Role::Dialogue, "")?;
    project.elements.insert(element + 1, new);
    Ok(Selection::caret(Position {
        element: element + 1,
        offset: 0,
    }))
}

fn is_uppercase_cue(role: Role) -> bool {
    matches!(role, Role::SceneHeading | Role::Character)
}

fn uppercase_with_mapping(text: &str, offset: usize) -> Result<(String, usize), EditError> {
    if !text.is_char_boundary(offset) {
        return Err(EditError::InvalidUtf8Boundary);
    }
    let mut output = String::with_capacity(text.len());
    let mut mapped = 0;
    for (index, character) in text.char_indices() {
        let uppercase = character.to_uppercase().collect::<String>();
        if index < offset {
            mapped += uppercase.len();
        }
        output.push_str(&uppercase);
    }
    Ok((output, mapped))
}

fn normalize_element(
    project: &mut Project,
    policy: &EditPolicy,
    element: usize,
    offset: usize,
) -> Result<usize, EditError> {
    let item = project
        .elements
        .get(element)
        .ok_or(EditError::InvalidElement)?;
    if !policy.uppercase_cues || !is_uppercase_cue(item.role) {
        return Ok(offset);
    }
    let id = item.id.clone();
    let old_text = item.text.clone();
    let old_runs = item.runs.clone();
    let (text, mapped) = uppercase_with_mapping(&old_text, offset)?;
    if text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if text == old_text {
        return Ok(mapped);
    }
    let has_source_mapping = project
        .preservation
        .as_ref()
        .is_some_and(|state| state.has_managed_element(&id));
    if has_source_mapping {
        let expected_runs = old_runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<_>>();
        project
            .preservation
            .as_mut()
            .ok_or(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ))?
            .uppercase_managed_text(
                &id,
                &old_text,
                (!old_runs.is_empty()).then_some(expected_runs.as_slice()),
                &text,
            )
            .map_err(EditError::Preservation)?;
    } else if project.elements[element].source_provenance.is_some() {
        return Err(EditError::Preservation(
            crate::preservation::PreservationError::ManagedAnchorMissing,
        ));
    }
    let item = &mut project.elements[element];
    item.text = text;
    if !old_runs.is_empty() {
        for run in &mut item.runs {
            run.text = run.text.to_uppercase();
        }
    }
    Ok(mapped)
}

fn normalize_selection_in_element(
    project: &mut Project,
    policy: &EditPolicy,
    element: usize,
    selection: Selection,
) -> Result<Selection, EditError> {
    let item = project
        .elements
        .get(element)
        .ok_or(EditError::InvalidElement)?;
    if !policy.uppercase_cues || !is_uppercase_cue(item.role) {
        return Ok(selection);
    }
    let id = item.id.clone();
    let old_text = item.text.clone();
    let old_runs = item.runs.clone();
    let (text, anchor) = uppercase_with_mapping(&old_text, selection.anchor.offset)?;
    let (_, focus) = uppercase_with_mapping(&old_text, selection.focus.offset)?;
    if text.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    if text != old_text {
        let has_source_mapping = project
            .preservation
            .as_ref()
            .is_some_and(|state| state.has_managed_element(&id));
        if has_source_mapping {
            let expected_runs = old_runs
                .iter()
                .map(|run| run.text.clone())
                .collect::<Vec<_>>();
            project
                .preservation
                .as_mut()
                .ok_or(EditError::Preservation(
                    crate::preservation::PreservationError::ManagedAnchorMissing,
                ))?
                .uppercase_managed_text(
                    &id,
                    &old_text,
                    (!old_runs.is_empty()).then_some(expected_runs.as_slice()),
                    &text,
                )
                .map_err(EditError::Preservation)?;
        } else if project.elements[element].source_provenance.is_some() {
            return Err(EditError::Preservation(
                crate::preservation::PreservationError::ManagedAnchorMissing,
            ));
        }
        let item = &mut project.elements[element];
        item.text = text;
        if !old_runs.is_empty() {
            for run in &mut item.runs {
                run.text = run.text.to_uppercase();
            }
        }
    }
    Ok(Selection {
        anchor: Position {
            element,
            offset: anchor,
        },
        focus: Position {
            element,
            offset: focus,
        },
    })
}

fn split_at(
    project: &mut Project,
    policy: &EditPolicy,
    position: Position,
    role: Role,
) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let item = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?
        .clone();
    ensure_elements_unanchored(project, [item.id.clone()])?;
    let left = item.text[..position.offset].to_owned();
    let right = item.text[position.offset..].to_owned();
    let old_runs = effective_runs(&item);
    let left_runs = slice_runs(&old_runs, 0, position.offset);
    let right_runs = slice_runs(&old_runs, position.offset, item.text.len());
    let id = split_id(project)?;
    let new_item = ScreenplayElement::new(id, role, right)?;
    project.elements[position.element].text = left;
    if !item.runs.is_empty() {
        project.elements[position.element].runs = left_runs;
    }
    let mut new_item = new_item;
    if !item.runs.is_empty() {
        new_item.runs = right_runs;
    }
    project.elements.insert(position.element + 1, new_item);
    normalize_element(project, policy, position.element + 1, 0)?;
    Ok(Selection::caret(Position {
        element: position.element + 1,
        offset: 0,
    }))
}

fn split_id(project: &Project) -> Result<String, ModelError> {
    Ok(project.allocate_element_id()?.as_str().to_owned())
}

fn join_at(project: &mut Project, index: usize) -> Result<(), EditError> {
    if index + 1 >= project.elements.len() {
        return Err(EditError::InvalidElement);
    }
    ensure_elements_unanchored(
        project,
        [
            project.elements[index].id.clone(),
            project.elements[index + 1].id.clone(),
        ],
    )?;
    let right = project.elements[index + 1].text.clone();
    let joined_runs =
        if project.elements[index].runs.is_empty() && project.elements[index + 1].runs.is_empty() {
            Vec::new()
        } else {
            let mut runs = effective_runs(&project.elements[index]);
            runs.extend(effective_runs(&project.elements[index + 1]));
            coalesce_runs(&mut runs);
            runs
        };
    let item = project
        .elements
        .get_mut(index)
        .ok_or(EditError::InvalidElement)?;
    if item.text.len().saturating_add(right.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    item.text.push_str(&right);
    item.runs = joined_runs;
    project.elements.remove(index + 1);
    Ok(())
}

fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |character| offset + character.len_utf8())
}

/// The supported Enter policy is intentionally explicit and finite.
pub fn transition(role: Role) -> Option<Role> {
    enter_transition(role)
}

pub fn enter_transition(role: Role) -> Option<Role> {
    Some(match role {
        Role::SceneHeading => Role::Action,
        Role::Action => Role::Action,
        Role::Character => Role::Dialogue,
        Role::Dialogue => Role::Action,
        Role::Parenthetical => Role::Dialogue,
        Role::Transition => Role::SceneHeading,
        Role::Note => Role::Note,
        Role::NewAct
        | Role::EndOfAct
        | Role::CastList
        | Role::General
        | Role::Shot
        | Role::Sequence
        | Role::Outline1
        | Role::Outline2
        | Role::Outline3
        | Role::Summary => role,
    })
}

/// Tab changes only the established legacy roles. Added roles are explicit
/// no-ops until a role-specific cycling policy is evidenced and implemented.
pub fn tab_transition(role: Role, current_text_is_empty: bool) -> Option<Role> {
    match role {
        Role::SceneHeading => Some(Role::Action),
        Role::Action => Some(Role::Character),
        Role::Character if current_text_is_empty => Some(Role::Transition),
        Role::Transition => Some(Role::SceneHeading),
        Role::Dialogue => Some(Role::Parenthetical),
        Role::Parenthetical => Some(Role::Dialogue),
        Role::Character | Role::Note => None,
        Role::NewAct
        | Role::EndOfAct
        | Role::CastList
        | Role::General
        | Role::Shot
        | Role::Sequence
        | Role::Outline1
        | Role::Outline2
        | Role::Outline3
        | Role::Summary => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preservation::{
        AnchorRelation, AnchorTarget, BoardEntity, BoardEntityKind, BoardGraph,
        DependencyConfidence, ForeignIdDomain, ForeignIdentity, ForeignReference, NamespaceBinding,
        PreservationState, PreservedAttribute, PreservedElement, PreservedNode, PreservedSection,
        ReferencePolicy, SectionKind, SourceAnchor, SourceAnchorId, SourceEncoding, SourceName,
        SourceOwnership, SourceSnapshot,
    };
    use crate::{Project, RunAttribute, SourceFormat, SourceProvenance, SourceSection};

    fn session(text: &str, role: Role) -> EditingSession {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements[0].role = role;
        project.elements[0].text = text.into();
        EditingSession::new(project).unwrap_or_else(|_| unreachable!())
    }
    fn set(session: &mut EditingSession, element: usize, offset: usize) {
        session
            .set_selection(Selection::caret(Position { element, offset }))
            .unwrap_or_else(|_| unreachable!());
    }

    fn source_name(local_name: &str) -> SourceName {
        SourceName {
            namespace_uri: None,
            local_name: local_name.into(),
            qualified_name: None,
        }
    }

    fn preserved_project(
        role: Role,
        text: &str,
        runs: Vec<TextRun>,
        source_children: Vec<PreservedNode>,
        nested_scene_attachment: bool,
    ) -> Project {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        let element_id = project.elements[0].id.clone();
        project.elements[0].role = role;
        project.elements[0].text = text.into();
        project.elements[0].runs = runs;
        project.elements[0].source_provenance = Some(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: 0,
            foreign_id: Some("foreign-paragraph".into()),
            foreign_role: Some(fdx_role_name(role).into()),
        });
        let paragraph_anchor =
            SourceAnchorId::new("anchor-paragraph").unwrap_or_else(|_| unreachable!());
        let mut children = source_children;
        let mut anchors = vec![SourceAnchor {
            id: paragraph_anchor.clone(),
            target: AnchorTarget::ProjectElement(element_id.clone()),
            relation: AnchorRelation::Represents,
        }];
        let mut identities = Vec::new();
        if nested_scene_attachment {
            let scene = ForeignIdentity::new(ForeignIdDomain::Scene, "scene-1")
                .unwrap_or_else(|_| unreachable!());
            let attachment_anchor =
                SourceAnchorId::new("anchor-scene-attachment").unwrap_or_else(|_| unreachable!());
            children.push(PreservedNode::Element(PreservedElement {
                name: source_name("SceneProperties"),
                attributes: Vec::new(),
                namespace_bindings: Vec::new(),
                source_type: None,
                ownership: SourceOwnership::Attached(scene.clone()),
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: Some(attachment_anchor.clone()),
                children: vec![PreservedNode::Element(PreservedElement {
                    name: source_name("Beat"),
                    attributes: Vec::new(),
                    namespace_bindings: Vec::new(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: vec![PreservedNode::Text("authored beat".into())],
                })],
            }));
            anchors.push(SourceAnchor {
                id: attachment_anchor,
                target: AnchorTarget::Foreign(scene.clone()),
                relation: AnchorRelation::Attached,
            });
            identities.push(scene);
        }
        let paragraph = PreservedElement {
            name: source_name("Paragraph"),
            attributes: vec![PreservedAttribute {
                name: source_name("Type"),
                value: fdx_role_name(role).into(),
            }],
            namespace_bindings: Vec::<NamespaceBinding>::new(),
            source_type: Some(fdx_role_name(role).into()),
            ownership: SourceOwnership::Managed(element_id.clone()),
            dependency_confidence: DependencyConfidence::SomeReferencesMapped,
            anchor: Some(paragraph_anchor),
            children,
        };
        project.preservation = Some(PreservationState {
            source: SourceSnapshot {
                format: SourceFormat::Fdx,
                original_bytes: b"<Script/>".to_vec(),
                encoding: SourceEncoding::Utf8,
            },
            sections: vec![PreservedSection {
                name: source_name("Content"),
                kind: SectionKind::Content,
                ordinal: 0,
                children: vec![PreservedNode::Element(paragraph)],
            }],
            anchors,
            identities,
            references: Vec::new(),
            board_graph: BoardGraph::default(),
        });
        project
    }

    fn text_children(project: &Project) -> Vec<String> {
        let state = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let PreservedNode::Element(paragraph) = &state.sections[0].children[0] else {
            unreachable!();
        };
        paragraph
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.clone()),
                PreservedNode::Element(_) => None,
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => None,
            })
            .collect()
    }

    #[test]
    fn unicode_boundaries_and_backspace_are_safe() {
        let mut s = session("a🙂b", Role::Action);
        set(&mut s, 0, 5);
        assert_eq!(s.backspace(), Ok(()));
        assert_eq!(s.project().elements[0].text, "ab");
        let mut unicode = session("a🙂b", Role::Action);
        assert_eq!(
            unicode.set_selection(Selection::caret(Position {
                element: 0,
                offset: 2
            })),
            Err(EditError::InvalidUtf8Boundary)
        );
    }

    #[test]
    fn enter_uses_explicit_role_transition_and_is_undoable() {
        let mut s = session("slug", Role::Character);
        set(&mut s, 0, 4);
        assert_eq!(s.enter(), Ok(()));
        assert_eq!(s.project().elements[1].role, Role::Dialogue);
        assert_eq!(s.undo(), Ok(()));
        assert_eq!(s.project().elements.len(), 1);
        assert_eq!(s.redo(), Ok(()));
        assert_eq!(s.project().elements[1].text, "");
    }

    #[test]
    fn enter_matrix_covers_every_role() {
        let expected = [
            (Role::SceneHeading, Role::Action),
            (Role::Action, Role::Action),
            (Role::Character, Role::Dialogue),
            (Role::Dialogue, Role::Action),
            (Role::Parenthetical, Role::Dialogue),
            (Role::Transition, Role::SceneHeading),
            (Role::Note, Role::Note),
        ];
        for (from, to) in expected {
            let mut s = session("text", from);
            let offset = s.project().elements[0].text.len();
            set(&mut s, 0, offset);
            s.enter().unwrap_or_else(|_| unreachable!());
            assert_eq!(s.project().elements[1].role, to);
        }
    }

    #[test]
    fn repeated_paragraph_splits_keep_ids_bounded_and_model_valid() {
        let mut s = session("paragraph", Role::Action);
        for _ in 0..64 {
            let element = s.project().elements.len().saturating_sub(1);
            let offset = s.project().elements[element].text.len();
            set(&mut s, element, offset);
            s.enter().unwrap_or_else(|_| unreachable!());
            assert!(s.validate().is_ok());
            assert_eq!(s.project().to_json(), Err(ModelError::UnsupportedJsonState));
        }
        assert_eq!(s.project().elements.len(), 65);
        assert!(s
            .project()
            .elements
            .iter()
            .all(|element| element.id.as_str().len() <= 128));
    }

    #[test]
    fn allocated_split_ids_avoid_project_and_title_page_ids() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements[0].text = "abcd".into();
        project.title_page.push(
            ScreenplayElement::new("element-2", Role::Action, "title")
                .unwrap_or_else(|_| unreachable!()),
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 2);
        assert_eq!(session.split(), Ok(()));
        assert_eq!(session.project().elements[1].id.as_str(), "element-3");
        assert!(session.validate().is_ok());
    }

    #[test]
    fn added_roles_split_without_changing_role_on_enter() {
        for role in Role::ALL[7..].iter().copied() {
            assert_eq!(enter_transition(role), Some(role));
            let mut session = session("unchanged", role);
            let offset = session.project().elements[0].text.len();
            set(&mut session, 0, offset);
            session.enter().unwrap_or_else(|_| unreachable!());
            assert_eq!(session.project().elements[1].role, role);
            assert_eq!(session.project().elements[0].role, role);
        }
    }

    #[test]
    fn added_roles_have_explicit_consumed_tab_noop_policy() {
        for role in Role::ALL[7..].iter().copied() {
            assert_eq!(tab_transition(role, false), None);
            let mut session = session("preserve this role", role);
            let before = session.clone();
            session.tab().unwrap_or_else(|_| unreachable!());
            assert_eq!(session, before);
            assert!(!session.can_undo());
        }
    }

    #[test]
    fn tab_matrix_is_undoable_and_unspecified_roles_are_consumed_noops() {
        let expected = [
            (Role::SceneHeading, Role::Action, "text"),
            (Role::Action, Role::Character, "text"),
            (Role::Character, Role::Transition, ""),
            (Role::Transition, Role::SceneHeading, "text"),
            (Role::Dialogue, Role::Parenthetical, "text"),
            (Role::Parenthetical, Role::Dialogue, "text"),
        ];
        for (from, to, text) in expected {
            let mut s = session(text, from);
            if from == Role::Parenthetical {
                s.set_policy(EditPolicy {
                    auto_parenthetical: false,
                    ..s.policy()
                });
            }
            let offset = s.project().elements[0].text.len();
            set(&mut s, 0, offset);
            s.tab().unwrap_or_else(|_| unreachable!());
            assert_eq!(s.project().elements[0].role, to);
            assert!(s.undo().is_ok());
            assert_eq!(s.project().elements[0].role, from);
        }

        let mut nonblank = session("BOB", Role::Character);
        let before = nonblank.clone();
        nonblank.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(nonblank, before);
        assert!(!nonblank.can_undo());

        let mut note = session("note", Role::Note);
        note.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(note.project().elements[0].text, "note");
        assert!(!note.can_undo());
    }

    #[test]
    fn cross_element_backspace_joins_without_losing_text() {
        let mut s = session("one", Role::Action);
        set(&mut s, 0, 3);
        assert_eq!(s.split(), Ok(()));
        assert_eq!(s.project().elements[1].text, "");
        set(&mut s, 1, 0);
        assert_eq!(s.backspace(), Ok(()));
        assert_eq!(s.project().elements.len(), 1);
        assert_eq!(s.project().elements[0].text, "one");
    }

    #[test]
    fn failed_edit_rolls_back_project_selection_history_and_dirty() {
        let mut s = session("🙂", Role::Action);
        let before = s.clone();
        assert_eq!(
            s.set_selection(Selection::caret(Position {
                element: 0,
                offset: 1
            })),
            Err(EditError::InvalidUtf8Boundary)
        );
        assert_eq!(s, before);
        assert_eq!(
            s.insert(&"x".repeat(MAX_TEXT_BYTES + 1)),
            Err(EditError::TextTooLong)
        );
        assert_eq!(s, before);
    }

    #[test]
    fn dirty_and_revision_follow_successful_edits_and_history() {
        let mut s = session("a", Role::Action);
        assert!(!s.dirty());
        assert_eq!(s.revision(), 0);
        set(&mut s, 0, 1);
        assert_eq!(s.insert("b"), Ok(()));
        assert!(s.dirty());
        assert_eq!(s.revision(), 1);
        assert_eq!(s.undo(), Ok(()));
        assert!(!s.dirty());
        assert_eq!(s.revision(), 2);
        assert_eq!(s.redo(), Ok(()));
        assert!(s.dirty());
        assert_eq!(s.revision(), 3);
    }

    #[test]
    fn text_edit_keeps_runs_and_source_children_transactional_through_history() {
        let run_a = RunAttribute {
            name: "Style".into(),
            value: "Bold".into(),
        };
        let run_b = RunAttribute {
            name: "Style".into(),
            value: "Italic".into(),
        };
        let project = preserved_project(
            Role::Action,
            "abcdef",
            vec![
                TextRun {
                    text: "abc".into(),
                    attributes: vec![run_a.clone()],
                },
                TextRun {
                    text: "def".into(),
                    attributes: vec![run_b.clone()],
                },
            ],
            vec![
                PreservedNode::Text("abc".into()),
                PreservedNode::CData("def".into()),
            ],
            true,
        );
        assert!(project.validate().is_ok());
        let mut session = EditingSession::new(project.clone()).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 2);
        session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 2,
                },
                focus: Position {
                    element: 0,
                    offset: 5,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(session.insert("Z"), Ok(()));
        let element = &session.project().elements[0];
        assert_eq!(element.text, "abZf");
        assert_eq!(element.runs[0].text, "abZ");
        assert_eq!(element.runs[0].attributes, vec![run_a]);
        assert_eq!(element.runs[1].text, "f");
        assert_eq!(element.runs[1].attributes, vec![run_b]);
        assert_eq!(text_children(session.project()), vec!["abZ", "f"]);
        assert!(session.dirty());
        assert_eq!(session.undo(), Ok(()));
        assert_eq!(session.project(), &project);
        assert!(!session.dirty());
        assert_eq!(session.redo(), Ok(()));
        assert_eq!(session.project().elements[0].text, "abZf");
        assert_eq!(text_children(session.project()), vec!["abZ", "f"]);
    }

    #[test]
    fn text_mapping_mismatch_fails_without_changing_any_session_state() {
        let mut project = preserved_project(
            Role::Action,
            "actual",
            Vec::new(),
            vec![PreservedNode::Text("different".into())],
            false,
        );
        project.elements[0]
            .source_provenance
            .as_mut()
            .unwrap_or_else(|| unreachable!())
            .foreign_role = Some(Role::Action.as_str().into());
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 2);
        let before = session.clone();
        assert_eq!(
            session.insert("X"),
            Err(EditError::Preservation(
                crate::preservation::PreservationError::ManagedTextMismatch
            ))
        );
        assert_eq!(session, before);
        assert!(!session.dirty());
        assert!(!session.can_undo());
    }

    #[test]
    fn deleting_or_splitting_an_anchored_scene_with_authored_data_safe_stops() {
        let project = preserved_project(
            Role::SceneHeading,
            "INT. ROOM",
            Vec::new(),
            vec![PreservedNode::Text("INT. ROOM".into())],
            true,
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 4);
        let before = session.clone();
        assert_eq!(
            session.split(),
            Err(EditError::Preservation(
                crate::preservation::PreservationError::StructuralAttachmentConflict
            ))
        );
        assert_eq!(session, before);
        assert!(!session.can_undo());
    }

    #[test]
    fn removing_an_anchored_scene_by_join_rolls_back_with_its_authored_attachment() {
        let mut project = preserved_project(
            Role::SceneHeading,
            "INT. ROOM",
            Vec::new(),
            vec![PreservedNode::Text("INT. ROOM".into())],
            true,
        );
        project.elements.push(
            ScreenplayElement::new("next", Role::Action, "action")
                .unwrap_or_else(|_| unreachable!()),
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 1, 0);
        let before = session.clone();
        assert_eq!(
            session.backspace(),
            Err(EditError::Preservation(
                crate::preservation::PreservationError::StructuralAttachmentConflict
            ))
        );
        assert_eq!(session, before);
        assert!(!session.dirty());
        assert!(!session.can_undo());
    }

    #[test]
    fn role_change_updates_only_the_mapped_type_and_is_undoable() {
        let project = preserved_project(
            Role::Dialogue,
            "hello",
            Vec::new(),
            vec![PreservedNode::Text("hello".into())],
            true,
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 0);
        assert_eq!(session.change_role(Role::Action), Ok(()));
        let PreservedNode::Element(paragraph) = &session
            .project()
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .sections[0]
            .children[0]
        else {
            unreachable!();
        };
        assert_eq!(paragraph.source_type.as_deref(), Some("Action"));
        assert_eq!(paragraph.attributes[0].value, "Action");
        assert_eq!(
            session.project().elements[0]
                .source_provenance
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .foreign_role
                .as_deref(),
            Some("Action")
        );
        assert_eq!(session.undo(), Ok(()));
        assert_eq!(session.project().elements[0].role, Role::Dialogue);
        assert_eq!(
            session
                .project()
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .sections[0]
                .children[0],
            {
                let original = preserved_project(
                    Role::Dialogue,
                    "hello",
                    Vec::new(),
                    vec![PreservedNode::Text("hello".into())],
                    true,
                );
                original
                    .preservation
                    .unwrap_or_else(|| unreachable!())
                    .sections[0]
                    .children[0]
                    .clone()
            }
        );
    }

    #[test]
    fn opaque_inline_intersections_stop_but_boundary_insertions_keep_order() {
        let project = preserved_project(
            Role::Action,
            "abcdef",
            Vec::new(),
            vec![
                PreservedNode::Text("abc".into()),
                PreservedNode::Element(PreservedElement {
                    name: source_name("OpaqueInline"),
                    attributes: Vec::new(),
                    namespace_bindings: Vec::new(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: vec![PreservedNode::Text("unmodeled".into())],
                }),
                PreservedNode::CData("def".into()),
            ],
            false,
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        session
            .set_selection(Selection {
                anchor: Position {
                    element: 0,
                    offset: 2,
                },
                focus: Position {
                    element: 0,
                    offset: 4,
                },
            })
            .unwrap_or_else(|_| unreachable!());
        let before = session.clone();
        assert_eq!(
            session.insert("X"),
            Err(EditError::Preservation(
                crate::preservation::PreservationError::UnsupportedTextMapping
            ))
        );
        assert_eq!(session, before);

        set(&mut session, 0, 3);
        assert_eq!(session.insert("X"), Ok(()));
        assert_eq!(session.project().elements[0].text, "abcXdef");
        assert_eq!(text_children(session.project()), vec!["abc", "Xdef"]);
        let PreservedNode::Element(paragraph) = &session
            .project()
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .sections[0]
            .children[0]
        else {
            unreachable!();
        };
        assert!(matches!(paragraph.children[1], PreservedNode::Element(_)));
    }

    #[test]
    fn scene_heading_role_change_with_nested_scene_data_safe_stops() {
        let project = preserved_project(
            Role::SceneHeading,
            "INT. ROOM",
            Vec::new(),
            vec![PreservedNode::Text("INT. ROOM".into())],
            true,
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 0);
        let before = session.clone();
        assert_eq!(
            session.change_role(Role::Action),
            Err(EditError::Preservation(
                crate::preservation::PreservationError::StructuralAttachmentConflict
            ))
        );
        assert_eq!(session, before);
        assert!(!session.can_undo());
    }

    #[test]
    fn unicode_cue_expansion_updates_source_and_run_offsets_together() {
        let project = preserved_project(
            Role::SceneHeading,
            "straße",
            vec![TextRun {
                text: "straße".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold".into(),
                }],
            }],
            vec![PreservedNode::Text("straße".into())],
            false,
        );
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, "straße".len());
        assert_eq!(session.change_role(Role::SceneHeading), Ok(()));
        assert_eq!(session.project().elements[0].text, "STRASSE");
        assert_eq!(session.project().elements[0].runs[0].text, "STRASSE");
        assert_eq!(text_children(session.project()), vec!["STRASSE"]);
        assert_eq!(session.selection().focus.offset, "STRASSE".len());
        assert!(session.validate().is_ok());
    }

    #[test]
    fn ordinary_script_edits_leave_board_graph_and_cross_domain_links_untouched() {
        let mut project = preserved_project(
            Role::Action,
            "action",
            Vec::new(),
            vec![PreservedNode::Text("action".into())],
            false,
        );
        let state = project
            .preservation
            .as_mut()
            .unwrap_or_else(|| unreachable!());
        let script_paragraph = ForeignIdentity::new(ForeignIdDomain::Paragraph, "para-foreign")
            .unwrap_or_else(|_| unreachable!());
        let list_item = ForeignIdentity::new(ForeignIdDomain::ListItem, "list-1")
            .unwrap_or_else(|_| unreachable!());
        let board = ForeignIdentity::new(ForeignIdDomain::Board, "board-1")
            .unwrap_or_else(|_| unreachable!());
        state.identities.push(script_paragraph.clone());
        state.board_graph.entities = vec![
            BoardEntity {
                identity: list_item.clone(),
                kind: BoardEntityKind::ListItem,
                source_order: 1,
                anchor: None,
                image_payload: None,
                declared_image_length: None,
                source_path: None,
            },
            BoardEntity {
                identity: board.clone(),
                kind: BoardEntityKind::DisplayBoard,
                source_order: 2,
                anchor: None,
                image_payload: None,
                declared_image_length: None,
                source_path: None,
            },
        ];
        state.board_graph.references.push(ForeignReference {
            source: board,
            target: list_item,
            policy: ReferencePolicy::RequiredInternal,
        });
        state.references.push(ForeignReference {
            source: script_paragraph,
            target: ForeignIdentity::new(ForeignIdDomain::Board, "orphanable-board-link")
                .unwrap_or_else(|_| unreachable!()),
            policy: ReferencePolicy::CrossDomainMayOrphan,
        });
        assert!(project.validate().is_ok());
        let state = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let board_graph = state.board_graph.clone();
        let references = state.references.clone();
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 6);
        assert_eq!(session.insert(" "), Ok(()));
        let state = session
            .project()
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        assert_eq!(state.board_graph, board_graph);
        assert_eq!(state.references, references);
        assert!(session.validate().is_ok());
    }

    #[test]
    fn non_preserved_split_and_join_keep_rich_runs_consistent() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements[0].role = Role::Action;
        project.elements[0].text = "abcdef".into();
        project.elements[0].runs = vec![
            TextRun {
                text: "abc".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold".into(),
                }],
            },
            TextRun {
                text: "def".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Italic".into(),
                }],
            },
        ];
        let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
        set(&mut session, 0, 4);
        assert_eq!(session.split(), Ok(()));
        assert_eq!(session.project().elements[0].runs[0].text, "abc");
        assert_eq!(session.project().elements[0].runs[1].text, "d");
        assert_eq!(session.project().elements[1].runs[0].text, "ef");
        assert_eq!(session.join_with_next(), Err(EditError::InvalidSelection));
        set(&mut session, 0, 4);
        assert_eq!(session.join_with_next(), Ok(()));
        assert_eq!(session.project().elements[0].text, "abcdef");
        assert_eq!(
            session.project().elements[0]
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "abcdef"
        );
        assert!(session.validate().is_ok());
    }
}
