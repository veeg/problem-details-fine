//! Shared fixtures and benchmark bodies. The global rewrite configuration is set once per process, so every
//! configuration scenario is its own bench binary calling [`run`].
#![allow(dead_code)]

use std::{
    hint::black_box,
    thread,
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, Throughput};
use problem_details_fine::{ProblemDetails, ProblemType, problem_type};

#[problem_type(type = "tag:bench.test,2025:UNIT", status = NOT_FOUND, title = "Unit")]
#[derive(Debug, Clone)]
pub struct Unit;

#[problem_type(
    type = "tag:bench.test,2025:FLAT",
    status = CONFLICT,
    title = "A widget with that id already exists",
    detail = "widget `{widget_id}` exists {count} times",
)]
#[derive(Debug, Clone)]
pub struct Flat {
    pub widget_id: String,
    pub count: u32,
    pub scopes: Vec<String>,
}

#[problem_type(type = "tag:bench.test,2025:TAGGED", status = BAD_REQUEST, title = "Tagged")]
#[derive(Debug, Clone)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Tagged {
    #[problem(detail = "field `{field}` at {line}:{column}: {message}")]
    Syntax {
        field: String,
        message: String,
        line: usize,
        column: usize,
    },
}

/// The `type` URIs of the fixtures, for configuring rewrites.
pub const TYPES: [&str; 3] = [Unit::TYPE, Flat::TYPE, Tagged::TYPE];

/// The authority of the fixtures' tag URIs.
pub const AUTHORITY: &str = "bench.test,2025";

pub fn flat() -> Flat {
    Flat {
        widget_id: "w-123".into(),
        count: 2,
        scopes: vec!["widgets:read".into(), "widgets:write".into()],
    }
}

pub fn tagged() -> Tagged {
    Tagged::Syntax {
        field: "items[3].name".into(),
        message: "expected `,` or `}`".into(),
        line: 12,
        column: 7,
    }
}

/// Serialize each fixture directly (one pass) and through `ProblemDetails` (payload into a map, then the
/// document), plus multi-threaded throughput, labelled with `scenario`.
pub fn run(c: &mut Criterion, scenario: &str) {
    let mut group = c.benchmark_group(format!("{scenario}/serialize"));
    group.bench_function("unit/direct", |b| {
        b.iter(|| serde_json::to_vec(black_box(&Unit)))
    });
    group.bench_function("unit/details", |b| {
        b.iter(|| serde_json::to_vec(&ProblemDetails::from(black_box(Unit))))
    });
    let value = flat();
    group.bench_function("flat/direct", |b| {
        b.iter(|| serde_json::to_vec(black_box(&value)))
    });
    group.bench_function("flat/details", |b| {
        b.iter(|| serde_json::to_vec(&ProblemDetails::from(black_box(value.clone()))))
    });
    group.bench_function("flat/clone_only", |b| b.iter(|| black_box(value.clone())));
    let value = tagged();
    group.bench_function("tagged/direct", |b| {
        b.iter(|| serde_json::to_vec(black_box(&value)))
    });
    group.bench_function("tagged/details", |b| {
        b.iter(|| serde_json::to_vec(&ProblemDetails::from(black_box(value.clone()))))
    });
    group.bench_function("type_uri", |b| {
        let details = ProblemDetails::from(flat());
        b.iter(|| black_box(&details).type_uri().len())
    });
    group.finish();

    threads(c, scenario);
}

/// Documents per thread in one measured batch of the thread-scaling benchmark.
const PER_THREAD: u64 = 1_000;

/// Throughput of 1, 2, 4 and 8 threads serializing concurrently. With the rewrite configuration read lock-free,
/// throughput per thread should stay flat as threads are added.
fn threads(c: &mut Criterion, scenario: &str) {
    let mut group = c.benchmark_group(format!("{scenario}/threads"));
    for threads in [1u64, 2, 4, 8] {
        group.throughput(Throughput::Elements(threads * PER_THREAD));
        group.bench_with_input(
            BenchmarkId::from_parameter(threads),
            &threads,
            |b, &threads| {
                b.iter_custom(|iters| {
                    let mut elapsed = Duration::ZERO;
                    for _ in 0..iters {
                        let start = Instant::now();
                        thread::scope(|scope| {
                            for _ in 0..threads {
                                scope.spawn(|| {
                                    let value = flat();
                                    let mut buffer = Vec::with_capacity(256);
                                    for _ in 0..PER_THREAD {
                                        buffer.clear();
                                        serde_json::to_writer(&mut buffer, black_box(&value))
                                            .unwrap();
                                        black_box(&buffer);
                                    }
                                });
                            }
                        });
                        elapsed += start.elapsed();
                    }
                    elapsed
                })
            },
        );
    }
    group.finish();
}
