//! 128 exact rules. Three of them match the fixture types.

mod common;

use criterion::{Criterion, criterion_group, criterion_main};
use problem_details_fine::rewrite::{RewriteConfig, RewriteRule};

fn bench(c: &mut Criterion) {
    (0..125)
        .map(|i| format!("https://unrelated.example.com/problems/{i}"))
        .chain(common::TYPES.iter().map(ToString::to_string))
        .fold(RewriteConfig::new(), |config, from| {
            let to = format!("https://docs.example.com/{from}");
            config.rule(RewriteRule::exact(from).type_uri(to))
        })
        .register_global()
        .unwrap();
    common::run(c, "rewrite_exact_128");
}

criterion_group!(benches, bench);
criterion_main!(benches);
