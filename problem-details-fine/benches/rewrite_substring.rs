//! A substring rule that matches each fixture type. It allocates the new URI.

mod common;

use criterion::{Criterion, criterion_group, criterion_main};
use problem_details_fine::rewrite::{RewriteConfig, RewriteRule};

fn bench(c: &mut Criterion) {
    RewriteConfig::new()
        .rule(RewriteRule::substring(
            common::AUTHORITY,
            "example.com,2026",
        ))
        .register_global()
        .unwrap();
    common::run(c, "rewrite_substring");
}

criterion_group!(benches, bench);
criterion_main!(benches);
