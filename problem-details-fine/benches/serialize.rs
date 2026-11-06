//! Serialization without rewrite rules: `baseline` without the `rewrite` feature, `no_rules` with it.

mod common;

use criterion::{Criterion, criterion_group, criterion_main};

fn bench(c: &mut Criterion) {
    let scenario = if cfg!(feature = "rewrite") {
        "no_rules"
    } else {
        "baseline"
    };
    common::run(c, scenario);
}

criterion_group!(benches, bench);
criterion_main!(benches);
