//! Detached, UI-independent Story Map placement planning and publication.

use scriptcraft_model::{
    BeatCard, Project, StableId, StoryMapPlacement,
    MAX_STORY_MAP_LANES, MAX_STORY_MAP_TARGET_PAGES,
};

use super::{EditError, EditingSession};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapPlacementRequest {
    pub card: StableId,
    pub lane_uuid: String,
    pub start_eighths: u64,
    pub length_eighths: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryMapPlacementChangeKind {
    Primary,
    InitiallyCarriedFollower,
    CollisionDisplacement,
    DisplacementTriggeredFollower,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapAffectedPlacement {
    pub card: StableId,
    pub original: Option<StoryMapPlacement>,
    pub final_placement: Option<StoryMapPlacement>,
    pub change: StoryMapPlacementChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMapPlacementPlan {
    /// Fully validated detached Project; it is never installed by planning.
    pub candidate: Project,
    /// Only cards whose optional placement differs from the source Project.
    pub affected: Vec<StoryMapAffectedPlacement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryMapPlacementRejection {
    MissingCard,
    MissingLane,
    InvalidBounds,
    TargetExceeded,
    InvalidSourceProject,
    InconsistentPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryMapPlacementPlanError {
    Rejected(StoryMapPlacementRejection),
    Failed(EditError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryMapPlacementOutcome {
    Changed,
    NoOp,
    Rejected(StoryMapPlacementRejection),
}

#[derive(Clone)]
struct CardPlan {
    id: StableId,
    original: Option<StoryMapPlacement>,
    original_lane: Option<usize>,
    planned: Option<StoryMapPlacement>,
    assignment_recorded: bool,
    fixed_priority: bool,
    change: Option<StoryMapPlacementChangeKind>,
}

/// Computes the complete placement result without mutating or publishing `project`.
pub fn plan_story_map_placement(
    project: &Project,
    request: &StoryMapPlacementRequest,
) -> Result<StoryMapPlacementPlan, StoryMapPlacementPlanError> {
    if project.validate().is_err() {
        return reject(StoryMapPlacementRejection::InvalidSourceProject);
    }

    let story_map = project.story_map();
    let target_eighths = story_map
        .target_eighths()
        .ok_or(StoryMapPlacementPlanError::Rejected(
            StoryMapPlacementRejection::InvalidSourceProject,
        ))?;
    if story_map.lanes.len() > MAX_STORY_MAP_LANES
        || story_map.target_pages == 0
        || story_map.target_pages > MAX_STORY_MAP_TARGET_PAGES
        || request.length_eighths == 0
    {
        return reject(StoryMapPlacementRejection::InvalidBounds);
    }
    let primary_end = request
        .start_eighths
        .checked_add(request.length_eighths)
        .ok_or(StoryMapPlacementPlanError::Rejected(
            StoryMapPlacementRejection::InvalidBounds,
        ))?;
    if primary_end > target_eighths {
        return reject(StoryMapPlacementRejection::TargetExceeded);
    }

    let destination_lane = story_map
        .lanes
        .iter()
        .position(|lane| lane.uuid == request.lane_uuid)
        .ok_or(StoryMapPlacementPlanError::Rejected(
            StoryMapPlacementRejection::MissingLane,
        ))?;
    let board = project.beat_board();
    let primary_index = board
        .cards
        .iter()
        .position(|card| card.id == request.card)
        .ok_or(StoryMapPlacementPlanError::Rejected(
            StoryMapPlacementRejection::MissingCard,
        ))?;

    let mut cards = Vec::with_capacity(board.cards.len());
    for card in &board.cards {
        let original = card.story_map_placement.clone();
        let original_lane = match &original {
            Some(placement) => Some(
                story_map
                    .lanes
                    .iter()
                    .position(|lane| lane.uuid == placement.lane_uuid)
                    .ok_or(StoryMapPlacementPlanError::Rejected(
                        StoryMapPlacementRejection::InvalidSourceProject,
                    ))?,
            ),
            None => None,
        };
        cards.push(CardPlan {
            id: card.id.clone(),
            planned: original.clone(),
            original,
            original_lane,
            assignment_recorded: false,
            fixed_priority: false,
            change: None,
        });
    }

    let old_primary = cards[primary_index].original.clone();
    let requested = StoryMapPlacement {
        lane_uuid: request.lane_uuid.clone(),
        start_eighths: request.start_eighths,
        length_eighths: request.length_eighths,
    };
    if old_primary.as_ref() == Some(&requested) {
        return Ok(StoryMapPlacementPlan {
            candidate: project.clone(),
            affected: Vec::new(),
        });
    }

    let is_resize = old_primary.as_ref().is_some_and(|old| {
        old.lane_uuid == requested.lane_uuid && old.start_eighths == requested.start_eighths
    });
    let is_shrink = is_resize
        && old_primary
            .as_ref()
            .is_some_and(|old| requested.length_eighths < old.length_eighths);
    let primary_delta = match &old_primary {
        Some(old) if !is_resize => Some(checked_delta(requested.start_eighths, old.start_eighths)?),
        _ => None,
    };

    let mut priority_by_lane = vec![Vec::<usize>::new(); story_map.lanes.len()];
    assign_placement(
        &mut cards,
        primary_index,
        requested,
        StoryMapPlacementChangeKind::Primary,
        true,
        target_eighths,
    )?;
    push_unique(&mut priority_by_lane[destination_lane], primary_index);

    // Move/lane-transfer followers are selected from the immutable original snapshot.
    if let (Some(old), Some(delta), Some(original_lane)) = (
        old_primary.as_ref(),
        primary_delta,
        cards[primary_index].original_lane,
    ) {
        let original_end = old.checked_end_eighths().ok_or(
            StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidSourceProject),
        )?;
        let mut followers = cards
            .iter()
            .enumerate()
            .filter_map(|(index, state)| {
                let placement = state.original.as_ref()?;
                let lane = state.original_lane?;
                (index != primary_index
                    && lane > original_lane
                    && placement.start_eighths >= old.start_eighths
                    && placement.start_eighths < original_end)
                    .then_some((lane, state.id.as_str().to_owned(), index))
            })
            .collect::<Vec<_>>();
        followers.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
        for (lane, _, index) in followers {
            let original = cards[index].original.as_ref().ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
            )?;
            let mut placement = original.clone();
            placement.start_eighths = shift_start(original.start_eighths, delta).ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::TargetExceeded),
            )?;
            assign_placement(
                &mut cards,
                index,
                placement,
                StoryMapPlacementChangeKind::InitiallyCarriedFollower,
                true,
                target_eighths,
            )?;
            push_unique(&mut priority_by_lane[lane], index);
        }
    }

    // Every lane is resolved once, top-to-bottom. Propagated followers are queued
    // only into later lanes, so this pass is bounded by lane/card counts.
    for lane_index in 0..story_map.lanes.len() {
        if is_shrink {
            break;
        }
        let priorities = priority_by_lane[lane_index].clone();
        validate_priority_intervals(&cards, &priorities)?;
        resolve_lane_collisions(&mut cards, &priorities, lane_index, target_eighths)?;

        let mut triggers = cards
            .iter()
            .enumerate()
            .filter_map(|(index, state)| {
                let original = state.original.as_ref()?;
                let final_value = state.planned.as_ref()?;
                let final_lane = story_map
                    .lanes
                    .iter()
                    .position(|lane| lane.uuid == final_value.lane_uuid)?;
                (state.assignment_recorded
                    && final_lane == lane_index
                    && original.start_eighths != final_value.start_eighths)
                    .then_some((final_value.start_eighths, state.id.as_str().to_owned(), index))
            })
            .collect::<Vec<_>>();
        triggers.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
        for (_, _, trigger_index) in triggers {
            propagate_followers(
                &mut cards,
                &mut priority_by_lane,
                story_map,
                trigger_index,
                lane_index,
                target_eighths,
            )?;
        }
    }

    let mut candidate = project.clone();
    let mut updated_board = board.clone();
    for (index, state) in cards.iter().enumerate() {
        if state.original != state.planned {
            let card: &mut BeatCard = updated_board.cards.get_mut(index).ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
            )?;
            card.story_map_placement = state.planned.clone();
        }
    }
    candidate
        .replace_beat_board(updated_board)
        .map_err(|error| StoryMapPlacementPlanError::Failed(EditError::Model(error)))?;

    let affected = cards
        .into_iter()
        .filter_map(|state| {
            (state.original != state.planned).then(|| StoryMapAffectedPlacement {
                card: state.id,
                original: state.original,
                final_placement: state.planned,
                change: state.change.unwrap_or(StoryMapPlacementChangeKind::CollisionDisplacement),
            })
        })
        .collect();
    Ok(StoryMapPlacementPlan { candidate, affected })
}

impl EditingSession {
    /// Recomputes against the current Project and publishes one changed candidate
    /// through the existing Beat snapshot boundary, preserving Beat selection.
    pub fn set_story_map_placement(
        &mut self,
        request: StoryMapPlacementRequest,
    ) -> Result<StoryMapPlacementOutcome, EditError> {
        let plan = match plan_story_map_placement(self.project(), &request) {
            Ok(plan) => plan,
            Err(StoryMapPlacementPlanError::Rejected(reason)) => {
                return Ok(StoryMapPlacementOutcome::Rejected(reason));
            }
            Err(StoryMapPlacementPlanError::Failed(error)) => return Err(error),
        };
        if plan.candidate == *self.project() {
            return Ok(StoryMapPlacementOutcome::NoOp);
        }
        self.publish_beat(plan.candidate, self.beat_selection.clone())?;
        Ok(StoryMapPlacementOutcome::Changed)
    }
}

fn propagate_followers(
    cards: &mut [CardPlan],
    priority_by_lane: &mut [Vec<usize>],
    story_map: &scriptcraft_model::StoryMap,
    trigger_index: usize,
    trigger_lane: usize,
    target_eighths: u64,
) -> Result<(), StoryMapPlacementPlanError> {
    let original = cards[trigger_index].original.clone().ok_or(
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
    )?;
    let final_start = cards[trigger_index]
        .planned
        .as_ref()
        .ok_or(StoryMapPlacementPlanError::Rejected(
            StoryMapPlacementRejection::InconsistentPlan,
        ))?
        .start_eighths;
    let delta = checked_delta(final_start, original.start_eighths)?;
    let original_end = original.checked_end_eighths().ok_or(
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidSourceProject),
    )?;
    for lane in trigger_lane.saturating_add(1)..story_map.lanes.len() {
        let mut followers = cards
            .iter()
            .enumerate()
            .filter_map(|(index, state)| {
                let placement = state.original.as_ref()?;
                (state.original_lane == Some(lane)
                    && placement.start_eighths >= original.start_eighths
                    && placement.start_eighths < original_end)
                    .then_some((state.id.as_str().to_owned(), index))
            })
            .collect::<Vec<_>>();
        followers.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, index) in followers {
            let source = cards[index].original.clone().ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
            )?;
            let mut desired = source.clone();
            desired.start_eighths = shift_start(source.start_eighths, delta).ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::TargetExceeded),
            )?;
            let was_fixed_priority = cards[index].fixed_priority;
            assign_placement(
                cards,
                index,
                desired,
                StoryMapPlacementChangeKind::DisplacementTriggeredFollower,
                true,
                target_eighths,
            )?;
            if !was_fixed_priority {
                push_unique(&mut priority_by_lane[lane], index);
            }
        }
    }
    Ok(())
}

fn resolve_lane_collisions(
    cards: &mut [CardPlan],
    priorities: &[usize],
    lane_index: usize,
    target_eighths: u64,
) -> Result<(), StoryMapPlacementPlanError> {
    let mut blockers = priorities.to_vec();
    for _ in 0..=cards.len() {
        let next = cards
            .iter()
            .enumerate()
            .filter(|(_, state)| {
                !state.assignment_recorded && state.original_lane == Some(lane_index)
            })
            .filter_map(|(index, state)| {
                let placement = state.planned.as_ref()?;
                let end = placement.checked_end_eighths()?;
                blockers
                    .iter()
                    .filter_map(|blocker_index| {
                        let blocker = cards[*blocker_index].planned.as_ref()?;
                        let blocker_end = blocker.checked_end_eighths()?;
                        (placement.start_eighths < blocker_end
                            && blocker.start_eighths < end)
                            .then_some((placement.start_eighths, state.id.as_str(), index))
                    })
                    .next()
            })
            .min_by(|left, right| (left.0, left.1).cmp(&(right.0, right.1)));
        let Some((_, _, displaced_index)) = next else {
            return Ok(());
        };

        let mut displaced = cards[displaced_index].planned.clone().ok_or(
            StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
        )?;
        let mut next_start = displaced.start_eighths;
        let mut resolved = false;
        for _ in 0..=blockers.len() {
            let end = next_start
                .checked_add(displaced.length_eighths)
                .ok_or(StoryMapPlacementPlanError::Rejected(
                    StoryMapPlacementRejection::InvalidBounds,
                ))?;
            let next_blocker_end = blockers
                .iter()
                .filter_map(|blocker_index| {
                    let blocker = cards[*blocker_index].planned.as_ref()?;
                    let blocker_end = blocker.checked_end_eighths()?;
                    (next_start < blocker_end && blocker.start_eighths < end)
                        .then_some(blocker_end)
                })
                .max();
            let Some(blocker_end) = next_blocker_end else {
                resolved = true;
                break;
            };
            if blocker_end <= next_start {
                return reject(StoryMapPlacementRejection::InconsistentPlan);
            }
            next_start = blocker_end;
            if next_start
                .checked_add(displaced.length_eighths)
                .map_or(true, |candidate_end| candidate_end > target_eighths)
            {
                return reject(StoryMapPlacementRejection::TargetExceeded);
            }
        }
        if !resolved {
            return reject(StoryMapPlacementRejection::InconsistentPlan);
        }
        displaced.start_eighths = next_start;
        assign_placement(
            cards,
            displaced_index,
            displaced,
            StoryMapPlacementChangeKind::CollisionDisplacement,
            false,
            target_eighths,
        )?;
        blockers.push(displaced_index);
    }
    reject(StoryMapPlacementRejection::InconsistentPlan)
}

fn assign_placement(
    cards: &mut [CardPlan],
    index: usize,
    placement: StoryMapPlacement,
    change: StoryMapPlacementChangeKind,
    fixed_priority: bool,
    target_eighths: u64,
) -> Result<(), StoryMapPlacementPlanError> {
    let end = placement.checked_end_eighths().ok_or(
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidBounds),
    )?;
    if placement.length_eighths == 0 {
        return reject(StoryMapPlacementRejection::InvalidBounds);
    }
    if end > target_eighths {
        return reject(StoryMapPlacementRejection::TargetExceeded);
    }
    let state = cards.get_mut(index).ok_or(
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InconsistentPlan),
    )?;
    if state.assignment_recorded {
        if state.planned.as_ref() != Some(&placement) {
            return reject(StoryMapPlacementRejection::InconsistentPlan);
        }
        state.fixed_priority |= fixed_priority;
        return Ok(());
    }
    state.assignment_recorded = true;
    state.fixed_priority = fixed_priority;
    state.change = Some(change);
    state.planned = Some(placement);
    Ok(())
}

fn push_unique(values: &mut Vec<usize>, index: usize) {
    if !values.contains(&index) {
        values.push(index);
    }
}

fn validate_priority_intervals(
    cards: &[CardPlan],
    priorities: &[usize],
) -> Result<(), StoryMapPlacementPlanError> {
    let mut intervals = priorities
        .iter()
        .map(|index| {
            let placement = cards
                .get(*index)
                .and_then(|state| state.planned.as_ref())
                .ok_or(StoryMapPlacementPlanError::Rejected(
                    StoryMapPlacementRejection::InconsistentPlan,
                ))?;
            let end = placement.checked_end_eighths().ok_or(
                StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidBounds),
            )?;
            Ok((placement.start_eighths, end, cards[*index].id.as_str()))
        })
        .collect::<Result<Vec<_>, StoryMapPlacementPlanError>>()?;
    intervals.sort_by(|left, right| (left.0, left.2).cmp(&(right.0, right.2)));
    if intervals
        .windows(2)
        .any(|pair| pair[1].0 < pair[0].1)
    {
        return reject(StoryMapPlacementRejection::InconsistentPlan);
    }
    Ok(())
}

fn checked_delta(new_start: u64, old_start: u64) -> Result<i64, StoryMapPlacementPlanError> {
    let new_start = i64::try_from(new_start).map_err(|_| {
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidBounds)
    })?;
    let old_start = i64::try_from(old_start).map_err(|_| {
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidBounds)
    })?;
    new_start.checked_sub(old_start).ok_or(
        StoryMapPlacementPlanError::Rejected(StoryMapPlacementRejection::InvalidBounds),
    )
}

fn shift_start(start: u64, delta: i64) -> Option<u64> {
    if delta < 0 {
        start.checked_sub(delta.unsigned_abs())
    } else {
        start.checked_add(u64::try_from(delta).ok()?)
    }
}

fn reject<T>(reason: StoryMapPlacementRejection) -> Result<T, StoryMapPlacementPlanError> {
    Err(StoryMapPlacementPlanError::Rejected(reason))
}
