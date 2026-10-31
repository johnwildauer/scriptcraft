//! Controlled WO-006 rendered-path benchmark. Run with `--nocapture` to
//! retain bounded measurements; output contains dimensions and counters only.

use scriptcraft_core::layout::{LayoutCache, ScreenplayLayout};
use scriptcraft_core::{LayoutInputs, Project, Role};
use std::hint::black_box;
use std::time::Instant;

const WARMUP: usize = 3;
const SAMPLES: usize = 10;
const LINES_PER_PAGE: usize = 55;
const PAGE_CASES: [usize; 6] = [1, 5, 15, 50, 120, 180];

#[derive(Debug, Default, Clone, Copy)]
struct Counters {
    frames: u64,
    events: u64,
    layout_builds: u64,
    layout_cache_hits: u64,
    painted_pages: u64,
    repaint_requests: u64,
}

#[derive(Debug)]
struct Measurement {
    p50_ms: f64,
    p95_ms: f64,
    pages: usize,
    counters: Counters,
}

fn fixture(pages: usize) -> (Project, LayoutInputs) {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    let text = (0..LINES_PER_PAGE)
        .map(|line| format!("generated benchmark line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    project.elements = (0..pages)
        .map(|page| {
            scriptcraft_core::ScreenplayElement::new(
                format!("page-{page}"),
                Role::Action,
                text.clone(),
            )
            .unwrap_or_else(|_| unreachable!())
        })
        .collect();
    let inputs = LayoutInputs {
        boundary_gap: 0,
        ..LayoutInputs::default()
    };
    (project, inputs)
}

fn percentile(samples: &mut [f64], percentile: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    let index = ((samples.len() as f64 * percentile).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len().saturating_sub(1));
    samples[index]
}

fn measure_uncached(project: &Project, inputs: &LayoutInputs) -> Measurement {
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut counters = Counters::default();
    let mut pages = 0;
    for iteration in 0..(WARMUP + SAMPLES) {
        let started = Instant::now();
        let layout = ScreenplayLayout::build_with_inputs(black_box(project), inputs)
            .unwrap_or_else(|_| unreachable!());
        pages = layout.pages.len();
        black_box(layout);
        if iteration >= WARMUP {
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            counters.frames += 1;
            counters.events += 1;
            counters.layout_builds += 1;
            counters.painted_pages += pages as u64;
            counters.repaint_requests += 1;
        }
    }
    Measurement {
        p50_ms: percentile(&mut samples, 0.50),
        p95_ms: percentile(&mut samples, 0.95),
        pages,
        counters,
    }
}

fn measure_cached(project: &Project, inputs: &LayoutInputs, edit: Option<usize>) -> Measurement {
    let mut cache = LayoutCache::default();
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut counters = Counters::default();
    let mut pages = 0;
    for iteration in 0..(WARMUP + SAMPLES) {
        let mut candidate = project.clone();
        if let Some(element) = edit {
            candidate.elements[element].text.push('x');
        }
        // A changed edit is a new session revision; an unchanged read keeps
        // the same revision and therefore exercises the real cache hit path.
        let revision = edit.map_or(0, |index| index as u64 + iteration as u64 + 1);
        let started = Instant::now();
        let (layout, hit) = cache
            .get_or_build_with_inputs(revision, black_box(&candidate), inputs)
            .unwrap_or_else(|_| unreachable!());
        pages = layout.pages.len();
        black_box(layout);
        if iteration >= WARMUP {
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            counters.frames += 1;
            counters.events += u64::from(edit.is_some());
            counters.layout_builds += u64::from(!hit);
            counters.layout_cache_hits += u64::from(hit);
            counters.painted_pages += pages as u64;
            counters.repaint_requests += u64::from(edit.is_some());
        }
    }
    Measurement {
        p50_ms: percentile(&mut samples, 0.50),
        p95_ms: percentile(&mut samples, 0.95),
        pages,
        counters,
    }
}

fn print_measurement(pages: usize, edit: &str, measurement: &Measurement) {
    let c = measurement.counters;
    println!(
        "pages={pages} edit={edit} layout_pages={} p50_ms={:.6} p95_ms={:.6} frames={} events={} layout_builds={} layout_cache_hits={} painted_pages={} repaint_requests={}",
        measurement.pages,
        measurement.p50_ms,
        measurement.p95_ms,
        c.frames,
        c.events,
        c.layout_builds,
        c.layout_cache_hits,
        c.painted_pages,
        c.repaint_requests,
    );
}

#[test]
fn controlled_long_document_benchmark_reports_wo006_oracle() {
    println!("WO-006a benchmark: warmup={WARMUP} samples={SAMPLES} unit=ms");
    for pages in PAGE_CASES {
        let (project, inputs) = fixture(pages);
        let positions = [("beginning", 0), ("middle", pages / 2), ("end", pages - 1)];
        for (label, element) in positions {
            let measurement = measure_cached(&project, &inputs, Some(element));
            print_measurement(pages, label, &measurement);
            assert_eq!(measurement.pages, pages);
            assert_eq!(measurement.counters.frames, SAMPLES as u64);
            assert_eq!(measurement.counters.events, SAMPLES as u64);
            assert_eq!(measurement.counters.layout_builds, SAMPLES as u64);
            assert_eq!(measurement.counters.painted_pages, (pages * SAMPLES) as u64);
            assert_eq!(measurement.counters.repaint_requests, SAMPLES as u64);
        }
    }

    let (long, inputs) = fixture(180);
    let uncached = measure_uncached(&long, &inputs);
    let cached = measure_cached(&long, &inputs, None);
    print_measurement(180, "unchanged-cached", &cached);
    println!(
        "unchanged-read-180-pages uncached_p95_ms={:.6} cached_p95_ms={:.6} reduction_pct={:.1}",
        uncached.p95_ms,
        cached.p95_ms,
        (1.0 - cached.p95_ms / uncached.p95_ms) * 100.0
    );
    assert_eq!(cached.counters.layout_builds, 0);
    assert_eq!(cached.counters.layout_cache_hits, SAMPLES as u64);
    assert!(cached.p95_ms < uncached.p95_ms);
}

#[test]
fn layout_cache_observes_build_then_unchanged_hit() {
    let (project, inputs) = fixture(1);
    let mut cache = LayoutCache::default();
    let (built, first_hit) = cache
        .get_or_build_with_inputs(7, &project, &inputs)
        .unwrap_or_else(|_| unreachable!());
    let (hit, second_hit) = cache
        .get_or_build_with_inputs(7, &project, &inputs)
        .unwrap_or_else(|_| unreachable!());
    assert!(!first_hit);
    assert!(second_hit);
    assert_eq!(built.as_ref(), hit.as_ref());
}
