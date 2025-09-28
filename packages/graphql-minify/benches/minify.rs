use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use graphql_minify::{MinifyAllocator, minify};

macro_rules! bench {
    ($group:ident, $id:literal, $input:ident) => {
        $group.bench_with_input($id, $input, |b, input| {
            b.iter(|| minify(input, &mut MinifyAllocator::default()))
        });
    };
}

fn bench_query(c: &mut Criterion) {
    const HERO_BASIC: &str = include_str!("../test_data/valid/hero_basic.graphql");
    const HERO_COMPARISON: &str = include_str!("../test_data/valid/hero_comparison.graphql");
    const KITCHEN_SINK: &str = include_str!("../test_data/kitchen_sink_query.graphql");

    let mut g = c.benchmark_group("query");

    bench!(g, "hero (basic)", HERO_BASIC);
    bench!(g, "hero (multiple)", HERO_COMPARISON);
    bench!(g, "kitchen sink", KITCHEN_SINK);

    g.finish();
}

fn bench_schema(c: &mut Criterion) {
    const KITCHEN_SINK: &str = include_str!("../test_data/valid/kitchen_sink_schema.graphql");
    const GITHUB: &str = include_str!("../test_data/valid/github_schema.graphql");

    let mut g = c.benchmark_group("schema");

    bench!(g, "kitchen sink", KITCHEN_SINK);
    bench!(g, "github", GITHUB);

    g.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = bench_query, bench_schema
);
criterion_main!(benches);
