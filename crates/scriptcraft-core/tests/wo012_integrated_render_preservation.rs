#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::layout::{LayoutCache, ScreenplayLayout};
use scriptcraft_core::{Project, Role, ScreenplayElement};

const BODY_PAGES: usize = 130;
const LINES_PER_PAGE: usize = 54;

fn long_source() -> Vec<u8> {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!("default project is valid"));
    project.elements = (0..BODY_PAGES)
        .map(|page| {
            let text = (0..LINES_PER_PAGE)
                .map(|line| format!("Page {page:03} action line {line:02} — café"))
                .collect::<Vec<_>>()
                .join("\n");
            ScreenplayElement::new(format!("long-action-{page}"), Role::Action, text)
                .unwrap_or_else(|_| unreachable!("bounded long-document paragraph is valid"))
        })
        .collect();

    let mut source = write_v6(&project).unwrap_or_else(|_| unreachable!("generated FDX is valid"));
    let root_close = b"</FinalDraft>";
    let root_close_at = source
        .windows(root_close.len())
        .rposition(|window| window == root_close)
        .unwrap_or_else(|| unreachable!("generated FDX has a root close tag"));
    source.splice(
        root_close_at..root_close_at,
        b"<FutureData><Opaque marker=\"keep\">source-owned payload</Opaque></FutureData>"
            .iter()
            .copied(),
    );
    source
}

fn assert_cached_matches_fresh(
    cache: &mut LayoutCache,
    session: &EditingSession,
) -> std::sync::Arc<ScreenplayLayout> {
    let (cached, _) = cache
        .get_or_build_with_inputs(session.revision(), session.project(), session.layout_inputs())
        .unwrap_or_else(|_| unreachable!("valid integrated project lays out"));
    let fresh = ScreenplayLayout::build_with_inputs(session.project(), session.layout_inputs())
        .unwrap_or_else(|_| unreachable!("valid integrated project has fresh layout"));
    assert_eq!(cached.as_ref(), &fresh);
    cached
}

#[test]
fn long_fdx_edit_history_layout_and_export_boundaries_remain_equivalent() {
    let source = long_source();
    let imported = read_v6(&source).unwrap_or_else(|_| unreachable!("generated FDX imports"));
    let mut session = EditingSession::new(imported.project)
        .unwrap_or_else(|_| unreachable!("imported long document validates"));

    let mut cache = LayoutCache::default();
    let initial_layout = assert_cached_matches_fresh(&mut cache, &session);
    assert!(initial_layout.pages.len() >= 120);
    assert_eq!(session.project().elements.len(), BODY_PAGES);

    let target = BODY_PAGES / 2;
    session
        .set_selection(Selection::caret(Position {
            element: target,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("middle-element caret is valid"));
    session
        .insert("Δ")
        .unwrap_or_else(|_| unreachable!("Unicode insertion is valid"));
    assert!(session.dirty());
    assert!(assert_cached_matches_fresh(&mut cache, &session).pages.len() >= 120);

    let edited = session.project().clone();
    session
        .undo()
        .unwrap_or_else(|_| unreachable!("integrated edit is undoable"));
    assert_ne!(session.project(), &edited);
    assert!(assert_cached_matches_fresh(&mut cache, &session).pages.len() >= 120);
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("integrated edit is redoable"));
    assert_eq!(session.project(), &edited);
    assert!(assert_cached_matches_fresh(&mut cache, &session).pages.len() >= 120);

    let before_exports = session.clone();
    let pdf = export_pdf(session.project()).unwrap_or_else(|_| unreachable!("print PDF exports"));
    let text = export_text(session.project()).unwrap_or_else(|_| unreachable!("plain text exports"));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(text.text.contains("LOSSY EXPORT"));
    assert_eq!(session, before_exports);

    let saved = write_v6(session.project()).unwrap_or_else(|_| unreachable!("edited FDX serializes"));
    let reopened = read_v6(&saved).unwrap_or_else(|_| unreachable!("serialized FDX reopens"));
    assert_eq!(
        reopened
            .project
            .elements
            .iter()
            .map(|element| (element.role, element.text.as_str()))
            .collect::<Vec<_>>(),
        session
            .project()
            .elements
            .iter()
            .map(|element| (element.role, element.text.as_str()))
            .collect::<Vec<_>>()
    );
    let saved_text = String::from_utf8(saved)
        .unwrap_or_else(|_| unreachable!("serialized FDX is UTF-8"));
    assert!(saved_text.contains("<FutureData><Opaque marker=\"keep\">source-owned payload</Opaque></FutureData>"));
    assert!(saved_text.contains("Δ"));

    let reopened_layout = ScreenplayLayout::build_with_inputs(
        &reopened.project,
        &session.layout_inputs().clone(),
    )
    .unwrap_or_else(|_| unreachable!("reopened project lays out"));
    assert_eq!(
        reopened_layout.pages.len(),
        assert_cached_matches_fresh(&mut cache, &session).pages.len()
    );
}
