//! Beat edits publish semantic board and identity graph together; XML is projected on save.
use super::{history::Snapshot, EditError, EditingSession};
use scriptcraft_model::{BeatCard, BeatParagraph, BeatPoint, BeatRect, Project, StableId,
    DEFAULT_CARD_WIDTH, DEFAULT_CARD_HEIGHT, MIN_CARD_WIDTH, MIN_CARD_HEIGHT,
    MAX_CARD_WIDTH, MAX_CARD_HEIGHT, BOARD_SNAP_INCREMENT};
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{BoardEntity, BoardEntityKind, ForeignIdDomain,
    AnchorTarget, ForeignIdentity, ForeignReference, PreservedNode, ReferencePolicy};
use scriptcraft_model::formatting::{FixedPoint, ParagraphAlignment, ParagraphLeading, ParagraphSpacing, SourceParagraphOverrides};
use scriptcraft_model::formatted_text::{clear_supported_styles_over_ranges, normalize_runs, parse_style_attribute, project_style_at_caret, set_inline_run_formatting_for_range, toggle_style_over_ranges, InlineRunFormattingPatch, RunBoundaryAffinity, RunStyle, RunStyleFlags, RunStyleRange};
use scriptcraft_model::{RunAttribute, TextRun, MAX_TEXT_BYTES};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BeatTextField { Title, Body }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BeatTextPosition { pub paragraph: usize, pub offset: usize }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BeatSelection {
    #[default]
    None,
    Card { card: StableId },
    Text { card: StableId, field: BeatTextField, anchor: BeatTextPosition, focus: BeatTextPosition },
}

impl BeatSelection {
    pub fn ordered(&self) -> Option<(BeatTextPosition, BeatTextPosition)> {
        match self {
            Self::Text { anchor, focus, .. } => Some(if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeatTextEditOperation { Insert { text: String }, Paste { text: String }, DeleteBackward, DeleteForward, Enter }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatHorizontalDirection { Left, Right }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeatLineEdge { Start, End }

pub(super) fn validate_beat_selection(project: &Project, selection: &BeatSelection) -> Result<(), EditError> {
    match selection {
        BeatSelection::None => Ok(()),
        BeatSelection::Card { card } if project.beat_board().cards.iter().any(|c| &c.id == card) => Ok(()),
        BeatSelection::Text { card, field, anchor, focus } => {
            let card = project.beat_board().cards.iter().find(|c| &c.id == card).ok_or(EditError::InvalidSelection)?;
            let valid = |position: BeatTextPosition| {
                let text = match field {
                    BeatTextField::Title if position.paragraph == 0 => Some(card.title.as_str()),
                    BeatTextField::Body => card.paragraphs.get(position.paragraph).map(|p| p.text.as_str()),
                    _ => None,
                }.ok_or(EditError::InvalidSelection)?;
                if position.offset > text.len() || !text.is_char_boundary(position.offset) { return Err(EditError::InvalidUtf8Boundary); }
                Ok(())
            };
            valid(*anchor)?;
            valid(*focus)
        }
        _ => Err(EditError::InvalidSelection),
    }
}

fn reserve_nodes(nodes: &[PreservedNode], ids: &mut BTreeSet<String>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            ids.extend(element.attributes.iter().filter(|a| a.name.namespace_uri.is_none() && a.name.local_name.eq_ignore_ascii_case("id")).map(|a| a.value.clone()));
            reserve_nodes(&element.children, ids);
        }
    }
}

fn durable_allocator(project: &Project) -> ParagraphIdAllocator {
    let mut ids = BTreeSet::new();
    for provenance in project.elements.iter().filter_map(|e| e.source_provenance.as_ref()).chain(project.title_page.iter().filter_map(|e| e.source_provenance.as_ref())) {
        ids.extend(provenance.foreign_id.iter().cloned());
    }
    ids.extend(project.dual_dialogue_groups.iter().filter_map(|g| g.wrapper.outer_foreign_id.clone()));
    if let Some(state) = &project.preservation {
        ids.extend(state.identities.iter().map(|i| i.value.clone()));
        ids.extend(state.board_graph.entities.iter().map(|e| e.identity.value.clone()));
        for r in state.references.iter().chain(&state.board_graph.references) { ids.insert(r.source.value.clone()); ids.insert(r.target.value.clone()); }
        for section in &state.sections { reserve_nodes(&section.children, &mut ids); }
    }
    let board = project.beat_board();
    ids.extend(board.foreign_id.iter().cloned());
    for card in &board.cards {
        ids.insert(card.foreign_id.clone());
        ids.extend(card.paragraphs.iter().filter_map(|p| p.foreign_id.clone()));
    }
    ParagraphIdAllocator::new(ids)
}

fn valid_rect(rect: BeatRect, board: &scriptcraft_model::BeatBoard) -> bool {
    [rect.left, rect.top, rect.width, rect.height].iter().all(|v| v.raw() >= 0 && v.raw() % BOARD_SNAP_INCREMENT.raw() == 0)
        && rect.width >= MIN_CARD_WIDTH && rect.width <= MAX_CARD_WIDTH
        && rect.height >= MIN_CARD_HEIGHT && rect.height <= MAX_CARD_HEIGHT
        && rect.contained_in(board.width, board.height)
}

// The graph contains semantic identities, not live XML nodes. Preserve all unrelated entities.
fn register_new_graph(project: &mut Project, board_id: &str, card_id: &str, paragraph_id: &str) -> Result<(), EditError> {
    let Some(state) = project.preservation.as_mut() else { return Ok(()) };
    let board = ForeignIdentity::new(ForeignIdDomain::Board, board_id).map_err(EditError::Preservation)?;
    let card = ForeignIdentity::new(ForeignIdDomain::ListItem, card_id).map_err(EditError::Preservation)?;
    let paragraph = ForeignIdentity::new(ForeignIdDomain::Paragraph, paragraph_id).map_err(EditError::Preservation)?;
    for (identity, kind) in [(board.clone(), BoardEntityKind::DisplayBoard), (card.clone(), BoardEntityKind::ListItem)] {
        if !state.board_graph.entities.iter().any(|e| e.identity == identity) {
            let order = match state.board_graph.entities.last() { Some(e) => e.source_order.checked_add(1).ok_or(EditError::InvalidElement)?, None => 0 };
            state.board_graph.entities.push(BoardEntity { identity, kind, source_order: order, anchor: None, image_payload: None, declared_image_length: None, source_path: None });
        }
    }
    if !state.board_graph.references.iter().any(|r| r.source == board && r.target == card && r.policy == ReferencePolicy::RequiredInternal) {
        state.board_graph.references.push(ForeignReference { source: board, target: card, policy: ReferencePolicy::RequiredInternal });
    }
    if !state.identities.contains(&paragraph) { state.identities.push(paragraph); }
    Ok(())
}

fn order_graph(project: &mut Project, board: &scriptcraft_model::BeatBoard) {
    let (Some(state), Some(board_id)) = (project.preservation.as_mut(), board.foreign_id.as_deref()) else { return };
    let managed = board.cards.iter().map(|card| card.foreign_id.as_str()).collect::<BTreeSet<_>>();
    let mut ordered = board.cards.iter().filter_map(|card| state.board_graph.references.iter().find(|r|
        r.source.domain == ForeignIdDomain::Board && r.source.value == board_id
            && r.target.domain == ForeignIdDomain::ListItem && r.target.value == card.foreign_id
            && r.policy == ReferencePolicy::RequiredInternal).cloned()).collect::<Vec<_>>().into_iter();
    for reference in &mut state.board_graph.references {
        if reference.source.domain == ForeignIdDomain::Board && reference.source.value == board_id
            && reference.target.domain == ForeignIdDomain::ListItem && managed.contains(reference.target.value.as_str())
            && reference.policy == ReferencePolicy::RequiredInternal {
            if let Some(next) = ordered.next() { *reference = next; }
        }
    }
    state.rebuild_indexes();
}

fn paragraph_defaults() -> SourceParagraphOverrides {
    let mut settings = SourceParagraphOverrides::default();
    settings.paragraph.alignment = Some(ParagraphAlignment::Left);
    settings.paragraph.first_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.leading = Some(ParagraphLeading::Regular);
    settings.paragraph.left_indent = Some(FixedPoint::from_raw(0));
    settings.paragraph.right_indent = Some(FixedPoint::from_raw(21_672));
    settings.paragraph.space_before = Some(FixedPoint::from_raw(0));
    settings.paragraph.spacing = Some(ParagraphSpacing::Single);
    settings.starts_new_page = Some(false);
    settings
}

fn effective_beat_runs(paragraph: &BeatParagraph) -> Vec<TextRun> {
    if paragraph.runs.is_empty() {
        if paragraph.text.is_empty() { Vec::new() } else { vec![TextRun { text: paragraph.text.clone(), attributes: Vec::new() }] }
    } else { paragraph.runs.clone() }
}

fn slice_beat_runs(runs: &[TextRun], start: usize, end: usize) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        let from = start.saturating_sub(base).min(run.text.len());
        let to = end.saturating_sub(base).min(run.text.len());
        if from < to { result.push(TextRun { text: run.text[from..to].to_owned(), attributes: run.attributes.clone() }); }
        base = limit;
        if base >= end { break; }
    }
    result
}

fn attributes_at(runs: &[TextRun], offset: usize, total: usize) -> Vec<RunAttribute> {
    let mut base = 0usize;
    for run in runs {
        let limit = base.saturating_add(run.text.len());
        if offset < limit || (offset == total && limit == total) { return run.attributes.clone(); }
        base = limit;
    }
    Vec::new()
}

fn insert_attribute(attributes: &mut Vec<RunAttribute>, name: &str, value: Option<String>) {
    if let Some(index) = attributes.iter().position(|attribute| attribute.name == name) {
        if let Some(value) = value { attributes[index].value = value; } else { attributes.remove(index); }
    } else if let Some(value) = value { attributes.push(RunAttribute { name: name.to_owned(), value }); }
}

fn styled_attributes(mut attributes: Vec<RunAttribute>, style: RunStyleFlags, patch: InlineRunFormattingPatch, apply_typing: bool) -> Result<Vec<RunAttribute>, EditError> {
    if apply_typing {
        let projection = parse_style_attribute(&attributes).map_err(EditError::FormattedText)?;
        let tokens = [
            (RunStyle::Bold, "Bold"), (RunStyle::Italics, "Italic"), (RunStyle::Underline, "Underline"),
            (RunStyle::Strikethrough, "Strikeout"), (RunStyle::AllCaps, "AllCaps"),
        ];
        let mut style_values = tokens.iter().filter_map(|(key, token)| style.contains(*key).then_some(*token).map(str::to_owned)).collect::<Vec<_>>();
        style_values.extend(projection.unknown_tokens);
        let style_value = (!style_values.is_empty()).then(|| style_values.join("+"));
        insert_attribute(&mut attributes, "Style", style_value);
    }
    if let Some(value) = patch.font { insert_attribute(&mut attributes, "Font", value.map(|v| v.fdx_value().to_owned())); }
    if let Some(value) = patch.size { insert_attribute(&mut attributes, "Size", value.map(|v| v.points().to_string())); }
    if let Some(value) = patch.color { insert_attribute(&mut attributes, "Color", value.map(|v| v.fdx_value().to_owned())); }
    if let Some(value) = patch.background { insert_attribute(&mut attributes, "Background", value.map(|v| v.fdx_value().to_owned())); }
    Ok(attributes)
}

fn push_run(runs: &mut Vec<TextRun>, text: &str, attributes: &[RunAttribute]) {
    if !text.is_empty() { runs.push(TextRun { text: text.to_owned(), attributes: attributes.to_vec() }); }
}

fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset].char_indices().next_back().map_or(0, |(index, _)| index)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..].char_indices().nth(1).map_or(text.len(), |(index, _)| offset + index)
}

fn register_paragraph(project: &mut Project, id: &str) -> Result<(), EditError> {
    if let Some(state) = project.preservation.as_mut() {
        let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, id).map_err(EditError::Preservation)?;
        if !state.identities.contains(&identity) { state.identities.push(identity); }
        state.rebuild_indexes();
    }
    Ok(())
}

fn unregister_paragraph(project: &mut Project, id: Option<&str>) {
    let Some(id) = id else { return };
    let Some(state) = project.preservation.as_mut() else { return };
    if let Ok(identity) = ForeignIdentity::new(ForeignIdDomain::Paragraph, id) {
        state.identities.retain(|item| item != &identity);
        state.anchors.retain(|anchor| !matches!(&anchor.target,
            scriptcraft_model::preservation::AnchorTarget::Foreign(target) if target == &identity));
        state.references.retain(|reference| reference.source != identity && reference.target != identity);
        state.board_graph.references.retain(|reference| reference.source != identity && reference.target != identity);
        state.rebuild_indexes();
    }
}

fn remove_deleted_card_graph(project: &mut Project, board_id: Option<&str>, card: &BeatCard) -> Result<(), EditError> {
    let Some(state) = project.preservation.as_mut() else { return Ok(()) };
    let board = board_id.map(|id| ForeignIdentity::new(ForeignIdDomain::Board, id.to_owned())
        .map_err(EditError::Preservation)).transpose()?;
    let item = ForeignIdentity::new(ForeignIdDomain::ListItem, card.foreign_id.clone())
        .map_err(EditError::Preservation)?;
    let paragraph_ids = card.paragraphs.iter().filter_map(|paragraph| paragraph.foreign_id.as_ref())
        .map(|id| ForeignIdentity::new(ForeignIdDomain::Paragraph, id.clone()).map_err(EditError::Preservation))
        .collect::<Result<Vec<_>, _>>()?;
    let deleted = std::iter::once(&item).chain(paragraph_ids.iter()).cloned().collect::<BTreeSet<_>>();
    let expected_reference = |reference: &ForeignReference| board.as_ref().is_some_and(|board| {
        reference.source == *board && reference.target == item
            && reference.policy == ReferencePolicy::RequiredInternal
    });
    let board_item_reference = |reference: &ForeignReference| {
        reference.source.domain == ForeignIdDomain::Board && reference.target == item
            && reference.policy == ReferencePolicy::RequiredInternal
    };

    // Semantic card identity was resolved before this helper. Graph registrations
    // are managed metadata and may be absent or repeated; only unsafe ownership
    // and surviving non-board reference shapes block the detached deletion. A
    // board Item is a managed reference to its ListItem, so deleting that owner
    // removes matching Item edges from every board without removing the boards.
    if state.board_graph.entities.iter().any(|entity| entity.identity == item
        && (entity.kind != BoardEntityKind::ListItem || entity.anchor.is_some()))
        || state.references.iter().any(|reference| {
            if deleted.contains(&reference.source) { return false; }
            if reference.target == item {
                return reference.source.domain == ForeignIdDomain::Board
                    || reference.policy == ReferencePolicy::RequiredInternal;
            }
            paragraph_ids.contains(&reference.target)
                && reference.policy == ReferencePolicy::RequiredInternal
        })
        || state.board_graph.references.iter().any(|reference| {
            if deleted.contains(&reference.source) { return false; }
            if reference.target == item {
                return !expected_reference(reference) && !board_item_reference(reference);
            }
            paragraph_ids.contains(&reference.target)
                && reference.policy == ReferencePolicy::RequiredInternal
        })
        || state.anchors.iter().any(|anchor| matches!(&anchor.target,
            AnchorTarget::Foreign(target) if deleted.contains(target))) {
        return Err(EditError::Preservation(scriptcraft_model::preservation::PreservationError::DanglingReference));
    }

    state.board_graph.entities.retain(|entity| entity.identity != item || entity.kind != BoardEntityKind::ListItem);
    state.board_graph.references.retain(|reference| !deleted.contains(&reference.source)
        && !expected_reference(reference) && !board_item_reference(reference));
    state.references.retain(|reference| !deleted.contains(&reference.source) && !expected_reference(reference));
    state.identities.retain(|identity| !paragraph_ids.contains(identity));
    state.rebuild_indexes();
    Ok(())
}

struct BeatParagraphIdAllocators {
    stable: scriptcraft_model::StableIdAllocator,
    durable: ParagraphIdAllocator,
}

impl BeatParagraphIdAllocators {
    fn new(project: &Project) -> Result<Self, EditError> {
        Ok(Self { stable: project.stable_id_allocator()?, durable: durable_allocator(project) })
    }
    fn allocate(&mut self) -> Result<(StableId, String), EditError> {
        let stable = self.stable.allocate("beat-paragraph")?;
        let durable = self.durable.allocate().map_err(|_| EditError::InvalidElement)?;
        Ok((stable, durable))
    }
}

fn set_paragraph_text(paragraph: &mut BeatParagraph, text: String, runs: Vec<TextRun>) -> Result<(), EditError> {
    if text.len() > MAX_TEXT_BYTES { return Err(EditError::TextTooLong); }
    paragraph.runs = normalize_runs(&text, &runs).map_err(EditError::FormattedText)?;
    paragraph.text = text;
    Ok(())
}

impl EditingSession {
    pub fn beat_selection(&self) -> &BeatSelection { &self.beat_selection }
    pub fn beat_selection_is_valid(&self) -> bool {
        validate_beat_selection(&self.project, &self.beat_selection).is_ok()
    }

    fn update_beat_typing_from_selection(&mut self, previous: &BeatSelection, selection: &BeatSelection) -> Result<(), EditError> {
        let old_style = self.beat_typing_style;
        let old_formatting = self.beat_typing_formatting;
        let next = match selection {
            BeatSelection::Text { card, field: BeatTextField::Body, anchor, focus } if anchor == focus => {
                let paragraph = self.project.beat_board().cards.iter().find(|item| &item.id == card)
                    .and_then(|item| item.paragraphs.get(focus.paragraph)).ok_or(EditError::InvalidSelection)?;
                project_style_at_caret(&paragraph.text, &paragraph.runs, focus.offset, RunBoundaryAffinity::Right, old_style)
                    .map_err(EditError::FormattedText)?
            }
            _ => RunStyleFlags::default(),
        };
        self.beat_typing_style = next;
        let same_body_caret = matches!((previous, selection),
            (BeatSelection::Text { card: old_card, field: BeatTextField::Body, anchor: old_anchor, focus: old_focus },
             BeatSelection::Text { card, field: BeatTextField::Body, anchor, focus })
             if old_card == card && old_anchor == old_focus && anchor == focus && old_focus == focus);
        if !same_body_caret {
            self.beat_typing_formatting = InlineRunFormattingPatch::default();
        }
        if self.beat_typing_style != old_style || self.beat_typing_formatting != old_formatting { self.mark_mutated(); }
        Ok(())
    }

    fn publish_beat(&mut self, project: Project, selection: BeatSelection) -> Result<(), EditError> {
        let previous_selection = self.beat_selection.clone();
        validate_beat_selection(&project, &selection)?;
        let changed = project != self.project;
        if !changed && selection == self.beat_selection { return Ok(()); }
        if changed {
            let snapshot = Snapshot { project: self.project.clone(), selection: self.selection,
                title_selection: self.title_selection, beat_selection: self.beat_selection.clone(),
                title_typing_style: self.title_typing_style, body_typing_style: self.body_typing_style,
                body_typing_formatting: self.body_typing_formatting, beat_typing_style: self.beat_typing_style,
                beat_typing_formatting: self.beat_typing_formatting, generated_parentheticals: self.generated_parentheticals.clone() };
            self.push_history(snapshot);
            self.project = project;
            self.redo.clear();
            self.revision = self.revision.saturating_add(1);
            self.dirty = self.project != self.clean_project;
        }
        self.beat_selection = selection;
        self.mark_mutated();
        self.update_beat_typing_from_selection(&previous_selection, &self.beat_selection.clone())?;
        Ok(())
    }

    pub fn set_beat_selection(&mut self, selection: BeatSelection) -> Result<(), EditError> {
        self.publish_beat(self.project.clone(), selection)
    }

    /// Toggle one supported inline style in Beat Body. A caret changes only
    /// session typing state; a range publishes one semantic candidate across
    /// every intersected paragraph.
    pub fn toggle_beat_run_style(&mut self, style: RunStyle) -> Result<(), EditError> {
        let BeatSelection::Text { card, field: BeatTextField::Body, anchor, focus } = self.beat_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if anchor == focus {
            let paragraph = self.project.beat_board().cards.iter().find(|item| item.id == card)
                .and_then(|item| item.paragraphs.get(focus.paragraph)).ok_or(EditError::InvalidSelection)?;
            let current = project_style_at_caret(&paragraph.text, &paragraph.runs, focus.offset,
                RunBoundaryAffinity::Right, self.beat_typing_style).map_err(EditError::FormattedText)?;
            self.beat_typing_style = current.toggled(style);
            self.mark_mutated();
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.beat_board().cards.iter().find(|item| item.id == card)
            .ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let ranges = (start.paragraph..=end.paragraph).map(|index| {
            let paragraph = &source.paragraphs[index];
            RunStyleRange { canonical_text: &paragraph.text, runs: &paragraph.runs,
                range: (if index == start.paragraph { start.offset } else { 0 })..
                    (if index == end.paragraph { end.offset } else { paragraph.text.len() }) }
        }).collect::<Vec<_>>();
        let edits = toggle_style_over_ranges(&ranges, self.beat_typing_style, style)
            .map_err(EditError::FormattedText)?;
        let mut candidate = self.project.clone();
        let mut board = candidate.beat_board().clone();
        let card_index = board.cards.iter().position(|item| item.id == card).ok_or(EditError::InvalidSelection)?;
        for (offset, edit) in edits.into_iter().enumerate() {
            board.cards[card_index].paragraphs[start.paragraph + offset].runs = edit.runs;
        }
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, self.beat_selection.clone())
    }

    /// Clear the supported style flags only, retaining unknown Style tokens and
    /// all non-Style run attributes.
    pub fn clear_beat_run_styles(&mut self) -> Result<(), EditError> {
        let BeatSelection::Text { card, field: BeatTextField::Body, anchor, focus } = self.beat_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if anchor == focus {
            if self.beat_typing_style != RunStyleFlags::default() {
                self.beat_typing_style = RunStyleFlags::default();
                self.mark_mutated();
            }
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.beat_board().cards.iter().find(|item| item.id == card)
            .ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let ranges = (start.paragraph..=end.paragraph).map(|index| {
            let paragraph = &source.paragraphs[index];
            RunStyleRange { canonical_text: &paragraph.text, runs: &paragraph.runs,
                range: (if index == start.paragraph { start.offset } else { 0 })..
                    (if index == end.paragraph { end.offset } else { paragraph.text.len() }) }
        }).collect::<Vec<_>>();
        let edits = clear_supported_styles_over_ranges(&ranges).map_err(EditError::FormattedText)?;
        let mut candidate = self.project.clone();
        let mut board = candidate.beat_board().clone();
        let card_index = board.cards.iter().position(|item| item.id == card).ok_or(EditError::InvalidSelection)?;
        for (offset, edit) in edits.into_iter().enumerate() {
            board.cards[card_index].paragraphs[start.paragraph + offset].runs = edit.runs;
        }
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, self.beat_selection.clone())
    }

    /// Set bounded Font/Size/Color/Highlight fields on the active Beat Body
    /// range, or merge them into the transient typing patch at a caret.
    pub fn set_beat_run_formatting(&mut self, patch: InlineRunFormattingPatch) -> Result<(), EditError> {
        if patch.is_empty() { return Ok(()); }
        let BeatSelection::Text { card, field: BeatTextField::Body, anchor, focus } = self.beat_selection.clone()
            else { return Err(EditError::InvalidSelection) };
        if anchor == focus {
            let mut next = self.beat_typing_formatting;
            if let Some(value) = patch.font { next.font = Some(value); }
            if let Some(value) = patch.size { next.size = Some(value); }
            if let Some(value) = patch.color { next.color = Some(value); }
            if let Some(value) = patch.background { next.background = Some(value); }
            if next != self.beat_typing_formatting {
                self.beat_typing_formatting = next;
                self.mark_mutated();
            }
            return Ok(());
        }
        let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let source = self.project.beat_board().cards.iter().find(|item| item.id == card)
            .ok_or(EditError::InvalidSelection)?;
        if end.paragraph >= source.paragraphs.len() { return Err(EditError::InvalidSelection); }
        let mut candidate = self.project.clone();
        let mut board = candidate.beat_board().clone();
        let card_index = board.cards.iter().position(|item| item.id == card).ok_or(EditError::InvalidSelection)?;
        let mut changed = false;
        for index in start.paragraph..=end.paragraph {
            let paragraph = &mut board.cards[card_index].paragraphs[index];
            let range = (if index == start.paragraph { start.offset } else { 0 })..
                (if index == end.paragraph { end.offset } else { paragraph.text.len() });
            if range.start > range.end || range.end > paragraph.text.len()
                || !paragraph.text.is_char_boundary(range.start) || !paragraph.text.is_char_boundary(range.end) {
                return Err(EditError::InvalidUtf8Boundary);
            }
            if range.is_empty() { continue; }
            let source_runs = effective_beat_runs(paragraph);
            let normalized = set_inline_run_formatting_for_range(&paragraph.text, &source_runs, range, patch)
                .map_err(EditError::FormattedText)?;
            changed |= normalized != source_runs;
            paragraph.runs = normalized;
        }
        if !changed { return Ok(()); }
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, self.beat_selection.clone())
    }

    pub fn create_beat_card(&mut self, position: BeatPoint) -> Result<(), EditError> {
        let mut board = self.project.beat_board().clone();
        let rect = BeatRect { left: position.x, top: position.y, width: DEFAULT_CARD_WIDTH, height: DEFAULT_CARD_HEIGHT };
        if !valid_rect(rect, &board) { return Err(EditError::InvalidElement); }
        let mut stable = self.project.stable_id_allocator()?;
        let card_id = stable.allocate("beat-card")?;
        let paragraph_id = stable.allocate("beat-paragraph")?;
        let mut durable = durable_allocator(&self.project);
        if board.foreign_id.is_none() { board.foreign_id = Some(durable.allocate().map_err(|_| EditError::InvalidElement)?); }
        let foreign_id = durable.allocate().map_err(|_| EditError::InvalidElement)?;
        let foreign_paragraph = durable.allocate().map_err(|_| EditError::InvalidElement)?;
        let mut settings = SourceParagraphOverrides::default();
        settings.paragraph.alignment = Some(ParagraphAlignment::Left);
        settings.paragraph.first_indent = Some(FixedPoint::from_raw(0));
        settings.paragraph.leading = Some(ParagraphLeading::Regular);
        settings.paragraph.left_indent = Some(FixedPoint::from_raw(0));
        settings.paragraph.right_indent = Some(FixedPoint::from_raw(21_672));
        settings.paragraph.space_before = Some(FixedPoint::from_raw(0));
        settings.paragraph.spacing = Some(ParagraphSpacing::Single);
        settings.starts_new_page = Some(false);
        board.cards.push(BeatCard { id: card_id.clone(), foreign_id: foreign_id.clone(), title: String::new(),
            paragraphs: vec![BeatParagraph { id: paragraph_id, foreign_id: Some(foreign_paragraph.clone()), source_paragraph_ordinal: None, text: String::new(), runs: Vec::new(), direct_settings: settings }], rect });
        board.card_creation_order.push(card_id.clone());
        let mut candidate = self.project.clone();
        register_new_graph(&mut candidate, board.foreign_id.as_deref().ok_or(EditError::InvalidElement)?, &foreign_id, &foreign_paragraph)?;
        order_graph(&mut candidate, &board);
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, BeatSelection::Card { card: card_id })
    }

    pub fn activate_beat_card(&mut self, card: StableId) -> Result<(), EditError> {
        let mut board = self.project.beat_board().clone();
        let index = board.cards.iter().position(|c| c.id == card).ok_or(EditError::InvalidSelection)?;
        let item = board.cards.remove(index);
        board.cards.push(item);
        let mut candidate = self.project.clone();
        order_graph(&mut candidate, &board);
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, BeatSelection::Card { card })
    }

    /// Delete exactly one semantic Beat card and its managed graph registrations.
    /// All graph checks and candidate validation happen on a detached Project.
    pub fn delete_beat_card(&mut self, card: StableId) -> Result<(), EditError> {
        let mut board = self.project.beat_board().clone();
        let index = board.cards.iter().position(|item| item.id == card).ok_or(EditError::InvalidSelection)?;
        let deleted = board.cards.remove(index);
        board.card_creation_order.retain(|id| id != &card);
        let mut candidate = self.project.clone();
        remove_deleted_card_graph(&mut candidate, board.foreign_id.as_deref(), &deleted)?;
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, BeatSelection::None)
    }

    pub fn set_beat_card_geometry(&mut self, card: StableId, rect: BeatRect, activate: bool) -> Result<(), EditError> {
        let mut board = self.project.beat_board().clone();
        if !valid_rect(rect, &board) { return Err(EditError::InvalidElement); }
        let index = board.cards.iter().position(|c| c.id == card).ok_or(EditError::InvalidSelection)?;
        board.cards[index].rect = rect;
        if activate { let item = board.cards.remove(index); board.cards.push(item); }
        let mut candidate = self.project.clone();
        order_graph(&mut candidate, &board);
        candidate.replace_beat_board(board)?;
        self.publish_beat(candidate, BeatSelection::Card { card })
    }

    /// Returns the selected canonical text in document order, with LF between body paragraphs.
    pub fn selected_beat_text(&self) -> Result<Option<String>, EditError> {
        let BeatSelection::Text { card, field, anchor, focus } = &self.beat_selection else { return Ok(None) };
        let card = self.project.beat_board().cards.iter().find(|item| &item.id == card).ok_or(EditError::InvalidSelection)?;
        let (start, end) = if anchor <= focus { (*anchor, *focus) } else { (*focus, *anchor) };
        match field {
            BeatTextField::Title => Ok(Some(card.title[start.offset..end.offset].to_owned())),
            BeatTextField::Body => {
                let mut value = String::new();
                for index in start.paragraph..=end.paragraph {
                    if index > start.paragraph { value.push('\n'); }
                    let paragraph = card.paragraphs.get(index).ok_or(EditError::InvalidSelection)?;
                    let from = if index == start.paragraph { start.offset } else { 0 };
                    let to = if index == end.paragraph { end.offset } else { paragraph.text.len() };
                    value.push_str(&paragraph.text[from..to]);
                }
                Ok(Some(value))
            }
        }
    }

    /// Select the complete title or ordered body for the currently selected card.
    pub fn select_all_beat_text(&mut self, field: BeatTextField) -> Result<(), EditError> {
        let card_id = match &self.beat_selection {
            BeatSelection::Card { card } | BeatSelection::Text { card, .. } => card.clone(),
            BeatSelection::None => return Err(EditError::InvalidSelection),
        };
        let card = self.project.beat_board().cards.iter().find(|item| item.id == card_id).ok_or(EditError::InvalidSelection)?;
        let (anchor, focus) = match field {
            BeatTextField::Title => (BeatTextPosition { paragraph: 0, offset: 0 }, BeatTextPosition { paragraph: 0, offset: card.title.len() }),
            BeatTextField::Body => (BeatTextPosition { paragraph: 0, offset: 0 }, {
                let last = card.paragraphs.last().ok_or(EditError::InvalidSelection)?;
                BeatTextPosition { paragraph: card.paragraphs.len() - 1, offset: last.text.len() }
            }),
        };
        self.set_beat_selection(BeatSelection::Text { card: card_id, field, anchor, focus })
    }

    /// Execute one model-authoritative Beat edit against the active Text selection.
    pub fn edit_beat_text(&mut self, operation: BeatTextEditOperation) -> Result<(), EditError> {
        let BeatSelection::Text { card, field, anchor, focus } = self.beat_selection.clone() else { return Err(EditError::InvalidSelection) };
        let (mut start, mut end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        if operation == BeatTextEditOperation::Enter && field == BeatTextField::Title {
            return self.set_beat_selection(BeatSelection::Text { card, field: BeatTextField::Body,
                anchor: BeatTextPosition { paragraph: 0, offset: 0 }, focus: BeatTextPosition { paragraph: 0, offset: 0 } });
        }
        let mut text = match &operation {
            BeatTextEditOperation::Insert { text } => { if text.contains('\r') || text.contains('\n') { return Err(EditError::InvalidSelection); } text.clone() }
            BeatTextEditOperation::Paste { text } if field == BeatTextField::Title => text.chars().map(|ch| if matches!(ch, '\r' | '\n') { ' ' } else { ch }).collect(),
            BeatTextEditOperation::Paste { text } => text.replace("\r\n", "\n").replace('\r', "\n"),
            BeatTextEditOperation::DeleteBackward | BeatTextEditOperation::DeleteForward | BeatTextEditOperation::Enter => String::new(),
        };
        if text.is_empty() && matches!(operation, BeatTextEditOperation::Insert { .. } | BeatTextEditOperation::Paste { .. }) { return Ok(()); }
        let mut project = self.project.clone();
        let mut board = project.beat_board().clone();
        let card_index = board.cards.iter().position(|item| item.id == card).ok_or(EditError::InvalidSelection)?;
        let mut final_position;
        match field {
            BeatTextField::Title => {
                let current = &board.cards[card_index].title;
                if start == end && text.is_empty() {
                    if operation == BeatTextEditOperation::DeleteBackward && start.offset > 0 { start.offset = previous_boundary(current, start.offset); }
                    else if operation == BeatTextEditOperation::DeleteForward && start.offset < current.len() { end.offset = next_boundary(current, end.offset); }
                    else { return Ok(()); }
                }
                let updated = format!("{}{}{}", &current[..start.offset], text, &current[end.offset..]);
                if updated.len() > MAX_TEXT_BYTES { return Err(EditError::TextTooLong); }
                if updated == current.as_str() { return Ok(()); }
                board.cards[card_index].title = updated;
                final_position = BeatTextPosition { paragraph: 0, offset: start.offset + text.len() };
            }
            BeatTextField::Body => {
                let paragraphs = &board.cards[card_index].paragraphs;
                if start == end && text.is_empty() {
                    let paragraph = paragraphs.get(start.paragraph).ok_or(EditError::InvalidSelection)?;
                    if operation == BeatTextEditOperation::DeleteBackward {
                        if start.offset > 0 { start.offset = previous_boundary(&paragraph.text, start.offset); }
                        else if start.paragraph > 0 { start.paragraph -= 1; start.offset = paragraphs[start.paragraph].text.len(); }
                        else { return Ok(()); }
                    } else if operation == BeatTextEditOperation::DeleteForward {
                        if start.offset < paragraph.text.len() { end.offset = next_boundary(&paragraph.text, start.offset); }
                        else if start.paragraph + 1 < paragraphs.len() { end.paragraph += 1; end.offset = 0; }
                        else { return Ok(()); }
                    }
                }
                let typing = if anchor == focus { self.beat_typing_style } else { RunStyleFlags::default() };
                let patch = if anchor == focus { self.beat_typing_formatting } else { InlineRunFormattingPatch::default() };
                let pieces = if operation == BeatTextEditOperation::Enter { vec![String::new()] } else { text.split('\n').map(str::to_owned).collect() };
                final_position = replace_beat_body_range(&mut project, &mut board, card_index, start, end, &pieces, typing, patch, anchor == focus)?;
                if operation == BeatTextEditOperation::Enter {
                    final_position = split_beat_paragraph(&mut project, &mut board, card_index, final_position)?;
                }
            }
        }
        project.replace_beat_board(board)?;
        let next = BeatSelection::Text { card, field, anchor: final_position, focus: final_position };
        self.publish_beat(project, next)
    }

    /// Move one Unicode scalar horizontally, crossing body paragraph boundaries.
    pub fn navigate_beat_horizontal(&mut self, direction: BeatHorizontalDirection, extend: bool) -> Result<(), EditError> {
        let BeatSelection::Text { card, field, anchor, focus } = self.beat_selection.clone() else { return Err(EditError::InvalidSelection) };
        let (ordered_start, ordered_end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
        let mut target = focus;
        if !extend && anchor != focus {
            target = match direction { BeatHorizontalDirection::Left => ordered_start, BeatHorizontalDirection::Right => ordered_end };
        } else {
            let text = self.beat_text_at(&card, field, target.paragraph)?;
            match direction {
                BeatHorizontalDirection::Left if target.offset > 0 => target.offset = previous_boundary(text, target.offset),
                BeatHorizontalDirection::Right if target.offset < text.len() => target.offset = next_boundary(text, target.offset),
                BeatHorizontalDirection::Left if field == BeatTextField::Body && target.paragraph > 0 => {
                    target.paragraph -= 1; target.offset = self.beat_text_at(&card, field, target.paragraph)?.len();
                }
                BeatHorizontalDirection::Right if field == BeatTextField::Body => {
                    let count = self.project.beat_board().cards.iter().find(|item| item.id == card).ok_or(EditError::InvalidSelection)?.paragraphs.len();
                    if target.paragraph + 1 < count { target.paragraph += 1; target.offset = 0; }
                }
                _ => {}
            }
        }
        self.set_beat_selection(BeatSelection::Text { card, field, anchor: if extend { anchor } else { target }, focus: target })
    }

    /// Move to the logical paragraph start/end. Desktop visual Home/End resolves its own layout target.
    pub fn navigate_beat_line_edge(&mut self, edge: BeatLineEdge, extend: bool) -> Result<(), EditError> {
        let BeatSelection::Text { card, field, anchor, focus } = self.beat_selection.clone() else { return Err(EditError::InvalidSelection) };
        let text = self.beat_text_at(&card, field, focus.paragraph)?;
        let target = BeatTextPosition { paragraph: focus.paragraph, offset: if edge == BeatLineEdge::Start { 0 } else { text.len() } };
        self.set_beat_selection(BeatSelection::Text { card, field, anchor: if extend { anchor } else { target }, focus: target })
    }

    fn beat_text_at(&self, card: &StableId, field: BeatTextField, paragraph: usize) -> Result<&str, EditError> {
        let card = self.project.beat_board().cards.iter().find(|item| &item.id == card).ok_or(EditError::InvalidSelection)?;
        match field { BeatTextField::Title if paragraph == 0 => Ok(&card.title), BeatTextField::Body => card.paragraphs.get(paragraph).map(|p| p.text.as_str()).ok_or(EditError::InvalidSelection), _ => Err(EditError::InvalidSelection) }
    }
}

fn replace_beat_body_range(project: &mut Project, board: &mut scriptcraft_model::BeatBoard, card_index: usize,
    start: BeatTextPosition, end: BeatTextPosition, inserted: &[String], typing: RunStyleFlags,
    patch: InlineRunFormattingPatch, apply_typing: bool) -> Result<BeatTextPosition, EditError> {
    let card = board.cards.get(card_index).ok_or(EditError::InvalidSelection)?;
    if start.paragraph > end.paragraph || end.paragraph >= card.paragraphs.len() { return Err(EditError::InvalidSelection); }
    let left = card.paragraphs[start.paragraph].clone();
    let right = card.paragraphs[end.paragraph].clone();
    if start.offset > left.text.len() || !left.text.is_char_boundary(start.offset) || end.offset > right.text.len() || !right.text.is_char_boundary(end.offset) { return Err(EditError::InvalidUtf8Boundary); }
    let left_runs = effective_beat_runs(&left);
    let right_runs = effective_beat_runs(&right);
    let attrs = styled_attributes(attributes_at(&left_runs, start.offset, left.text.len()), typing, patch, apply_typing)?;
    let chunks = if inserted.is_empty() { vec![String::new()] } else { inserted.to_vec() };
    let multiline = chunks.len() > 1;
    let prefix = left.text[..start.offset].to_owned();
    let suffix = right.text[end.offset..].to_owned();
    let mut new_paragraphs = Vec::with_capacity(chunks.len());
    let mut allocated = Vec::new();
    let mut ids = BeatParagraphIdAllocators::new(project)?;
    for (i, chunk) in chunks.iter().enumerate() {
        let preserve_right = multiline && i + 1 == chunks.len() && end.paragraph > start.paragraph;
        let mut paragraph = if i == 0 { left.clone() } else if preserve_right { right.clone() } else {
            let (id, foreign_id) = ids.allocate()?;
            allocated.push(foreign_id.clone());
            BeatParagraph { id, foreign_id: Some(foreign_id), source_paragraph_ordinal: None,
                text: String::new(), runs: Vec::new(), direct_settings: paragraph_defaults() }
        };
        let mut text = String::new();
        let mut runs = Vec::new();
        if i == 0 { text.push_str(&prefix); runs.extend(slice_beat_runs(&left_runs, 0, start.offset)); }
        text.push_str(chunk); push_run(&mut runs, chunk, &attrs);
        if i + 1 == chunks.len() { text.push_str(&suffix); runs.extend(slice_beat_runs(&right_runs, end.offset, right.text.len())); }
        if i == 0 && chunks.len() == 1 && start.paragraph == end.paragraph {
            runs = slice_beat_runs(&left_runs, 0, start.offset);
            push_run(&mut runs, chunk, &attrs);
            runs.extend(slice_beat_runs(&left_runs, end.offset, left.text.len()));
        }
        set_paragraph_text(&mut paragraph, text, runs)?;
        new_paragraphs.push(paragraph);
    }
    for old in &card.paragraphs[start.paragraph..=end.paragraph] {
        if !new_paragraphs.iter().any(|paragraph| paragraph.id == old.id) { unregister_paragraph(project, old.foreign_id.as_deref()); }
    }
    let mut updated = card.paragraphs.clone();
    updated.splice(start.paragraph..=end.paragraph, new_paragraphs);
    board.cards[card_index].paragraphs = updated;
    for id in allocated { register_paragraph(project, &id)?; }
    let offset = chunks.last().map_or(0, String::len) + if chunks.len() == 1 { prefix.len() } else { 0 };
    Ok(BeatTextPosition { paragraph: start.paragraph + chunks.len() - 1, offset })
}

fn split_beat_paragraph(project: &mut Project, board: &mut scriptcraft_model::BeatBoard, card_index: usize,
    position: BeatTextPosition) -> Result<BeatTextPosition, EditError> {
    let card = board.cards.get(card_index).ok_or(EditError::InvalidSelection)?;
    let source = card.paragraphs.get(position.paragraph).ok_or(EditError::InvalidSelection)?.clone();
    if position.offset > source.text.len() || !source.text.is_char_boundary(position.offset) { return Err(EditError::InvalidUtf8Boundary); }
    let (id, foreign_id) = BeatParagraphIdAllocators::new(project)?.allocate()?;
    let runs = effective_beat_runs(&source);
    let mut left = source.clone();
    let mut right = BeatParagraph { id, foreign_id: Some(foreign_id.clone()), source_paragraph_ordinal: None,
        text: String::new(), runs: Vec::new(), direct_settings: paragraph_defaults() };
    set_paragraph_text(&mut left, source.text[..position.offset].to_owned(), slice_beat_runs(&runs, 0, position.offset))?;
    set_paragraph_text(&mut right, source.text[position.offset..].to_owned(), slice_beat_runs(&runs, position.offset, source.text.len()))?;
    board.cards[card_index].paragraphs[position.paragraph] = left;
    board.cards[card_index].paragraphs.insert(position.paragraph + 1, right);
    register_paragraph(project, &foreign_id)?;
    Ok(BeatTextPosition { paragraph: position.paragraph + 1, offset: 0 })
}
