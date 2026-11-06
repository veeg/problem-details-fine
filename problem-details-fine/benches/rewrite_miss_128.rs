//! 128 exact rules, one ignore rule and one substring rule. No rule matches a fixture type.

mod common;

use criterion::{Criterion, criterion_group, criterion_main};
use problem_details_fine::rewrite::{RewriteConfig, RewriteRule};

fn bench(c: &mut Criterion) {
    (0..128)
        .map(|i| format!("https://unrelated.example.com/problems/{i}"))
        .fold(RewriteConfig::new(), |config, from| {
            config.rule(RewriteRule::exact(from).type_uri("https://x"))
        })
        .rule(RewriteRule::ignore("unrelated-ignore-0"))
        .rule(RewriteRule::substring(
            "unrelated.example.com,2025",
            "example.com,2026",
        ))
        .register_global()
        .unwrap();
    common::run(c, "rewrite_miss_128");
}

criterion_group!(benches, bench);
criterion_main!(benches);
