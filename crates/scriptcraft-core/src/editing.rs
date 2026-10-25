//! Headless, transactional editing over the semantic screenplay model.

use crate::{ModelError, Project, Role, ScreenplayElement, MAX_ELEMENTS, MAX_TEXT_BYTES};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidElement,
    InvalidUtf8Boundary,
    InvalidSelection,
    EmptyHistory,
    TextTooLong,
    TooManyElements,
    UnsupportedTransition { from: Role },
    Model(ModelError),
}

impl From<ModelError> for EditError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    project: Project,
    selection: Selection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditingSession {
    project: Project,
    selection: Selection,
    dirty: bool,
    revision: u64,
    clean_project: Project,
    history: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    history_limit: usize,
}

impl EditingSession {
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate()?;
        let selection = Selection::caret(Position {
            element: 0,
            offset: 0,
        });
        validate_selection(&project, selection)?;
        Ok(Self {
            clean_project: project.clone(),
            project,
            selection,
            dirty: false,
            revision: 0,
            history: Vec::new(),
            redo: Vec::new(),
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
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub const fn dirty(&self) -> bool {
        self.dirty
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn mark_saved(&mut self) {
        self.clean_project = self.project.clone();
        self.dirty = false;
    }

    pub fn replace_with_saved(&mut self, project: Project) -> Result<(), EditError> {
        let replacement = Self::new(project)?;
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

    pub fn insert(&mut self, text: &str) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            replace_range(project, start, end, text)?;
            Ok(Selection::caret(Position {
                element: start.element,
                offset: start.offset + text.len(),
            }))
        })
    }

    pub fn delete(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            if start != end {
                replace_range(project, start, end, "")?;
                return Ok(Selection::caret(start));
            }
            let item = project
                .elements
                .get(start.element)
                .ok_or(EditError::InvalidElement)?;
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
        self.mutate(|project, selection| {
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

    pub fn split(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            split_at(project, start, project.elements[start.element].role)
        })
    }

    pub fn enter(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            if start != end {
                return Err(EditError::InvalidSelection);
            }
            let role = project.elements[start.element].role;
            let next_role =
                enter_transition(role).ok_or(EditError::UnsupportedTransition { from: role })?;
            split_at(project, start, next_role)
        })
    }

    /// Applies the explicit Tab policy. Unlisted roles are intentionally consumed
    /// without changing the project, selection, or history.
    pub fn tab(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            if start != end {
                return Ok(*selection);
            }
            let element = &project.elements[start.element];
            let next_role = match element.role {
                Role::SceneHeading => Some(Role::Action),
                Role::Action => Some(Role::Character),
                Role::Character if element.text.is_empty() => Some(Role::Transition),
                Role::Transition => Some(Role::SceneHeading),
                Role::Dialogue => Some(Role::Parenthetical),
                Role::Parenthetical => Some(Role::Dialogue),
                _ => None,
            };
            if let Some(role) = next_role {
                project.elements[start.element].role = role;
            }
            Ok(*selection)
        })
    }

    pub fn join_with_next(&mut self) -> Result<(), EditError> {
        self.mutate(|project, selection| {
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

    pub fn change_role(&mut self, role: Role) -> Result<(), EditError> {
        self.mutate(|project, selection| {
            let (start, end) = selection.ordered();
            if start.element != end.element {
                return Err(EditError::InvalidSelection);
            }
            project.elements[start.element].role = role;
            Ok(*selection)
        })
    }

    pub fn undo(&mut self) -> Result<(), EditError> {
        let snapshot = self.history.pop().ok_or(EditError::EmptyHistory)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
        };
        self.redo.push(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), EditError> {
        let snapshot = self.redo.pop().ok_or(EditError::EmptyHistory)?;
        let current = Snapshot {
            project: self.project.clone(),
            selection: self.selection,
        };
        self.push_history(current);
        self.project = snapshot.project;
        self.selection = snapshot.selection;
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.project != self.clean_project;
        Ok(())
    }

    fn mutate<F>(&mut self, operation: F) -> Result<(), EditError>
    where
        F: FnOnce(&mut Project, &Selection) -> Result<Selection, EditError>,
    {
        let mut project = self.project.clone();
        let selection = operation(&mut project, &self.selection)?;
        project.validate()?;
        validate_selection(&project, selection)?;
        if project == self.project && selection == self.selection {
            return Ok(());
        }
        self.push_history(Snapshot {
            project: self.project.clone(),
            selection: self.selection,
        });
        self.project = project;
        self.selection = selection;
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
        let item = project
            .elements
            .get_mut(start.element)
            .ok_or(EditError::InvalidElement)?;
        let mut text = String::with_capacity(item.text.len().saturating_add(inserted.len()));
        text.push_str(&item.text[..start.offset]);
        text.push_str(inserted);
        text.push_str(&item.text[end.offset..]);
        if text.len() > MAX_TEXT_BYTES {
            return Err(EditError::TextTooLong);
        }
        item.text = text;
        return Ok(());
    }
    let prefix = project.elements[start.element].text[..start.offset].to_owned();
    let suffix = project.elements[end.element].text[end.offset..].to_owned();
    let combined = format!("{prefix}{inserted}{suffix}");
    if combined.len() > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    project.elements[start.element].text = combined;
    project.elements.drain(start.element + 1..=end.element);
    Ok(())
}

fn split_at(project: &mut Project, position: Position, role: Role) -> Result<Selection, EditError> {
    if project.elements.len() >= MAX_ELEMENTS {
        return Err(EditError::TooManyElements);
    }
    let item = project
        .elements
        .get(position.element)
        .ok_or(EditError::InvalidElement)?
        .clone();
    let left = item.text[..position.offset].to_owned();
    let right = item.text[position.offset..].to_owned();
    let id = format!("{}-split-{}", item.id.as_str(), project.elements.len());
    let new_item = ScreenplayElement::new(id, role, right)?;
    project.elements[position.element].text = left;
    project.elements.insert(position.element + 1, new_item);
    Ok(Selection::caret(Position {
        element: position.element + 1,
        offset: 0,
    }))
}

fn join_at(project: &mut Project, index: usize) -> Result<(), EditError> {
    if index + 1 >= project.elements.len() {
        return Err(EditError::InvalidElement);
    }
    let right = project.elements[index + 1].text.clone();
    let item = project
        .elements
        .get_mut(index)
        .ok_or(EditError::InvalidElement)?;
    if item.text.len().saturating_add(right.len()) > MAX_TEXT_BYTES {
        return Err(EditError::TextTooLong);
    }
    item.text.push_str(&right);
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Project;

    fn session(text: &str, role: Role) -> EditingSession {
        let mut project = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        project.elements[0].role = role;
        project.elements[0].text = text.into();
        EditingSession::new(project).unwrap_or_else(|_| unreachable!())
    }
    fn set(session: &mut EditingSession, element: usize, offset: usize) {
        session
            .set_selection(Selection::caret(Position { element, offset }))
            .unwrap_or_else(|_| unreachable!());
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
        set(&mut s, 0, 2);
        assert_eq!(s.enter(), Ok(()));
        assert_eq!(s.project().elements[1].role, Role::Dialogue);
        assert_eq!(s.undo(), Ok(()));
        assert_eq!(s.project().elements.len(), 1);
        assert_eq!(s.redo(), Ok(()));
        assert_eq!(s.project().elements[1].text, "ug");
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
            set(&mut s, 0, 2);
            s.enter().unwrap_or_else(|_| unreachable!());
            assert_eq!(s.project().elements[1].role, to);
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
}
