use std::hint::black_box;

const SCHEMA: &str = include_str!("../test_data/github_schema.graphql");

pub fn main() {
    let _ = graphql_minify::minify(black_box(SCHEMA), &mut bumpalo::Bump::new());
}
