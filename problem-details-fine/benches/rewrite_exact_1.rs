//! One exact rule for each fixture type.

mod common;

use criterion::{Criterion, criterion_group, criterion_main};
use problem_details_fine::rewrite::{RewriteConfig, RewriteRule};

fn bench(c: &mut Criterion) {
    common::TYPES
        .iter()
        .fold(RewriteConfig::new(), |config, from| {
            config.rule(
                RewriteRule::exact(*from).type_uri(format!("https://docs.example.com/{from}")),
            )
        })
        .register_global()
        .unwrap();
    common::run(c, "rewrite_exact_1");
}

criterion_group!(benches, bench);
criterion_main!(benches);
