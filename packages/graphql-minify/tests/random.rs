use graphql_minify::{MinifyAllocator, minify};
use graphql_semantic_compare::{GraphqlSemanticEquality, cmp_documents};
use test_each_file::test_each_file;

test_each_file! { in "./packages/graphql-minify/test_data/valid" as valid => test }
test_each_file! { in "./packages/graphql-minify/test_data/random" as random => test }

fn test(file: &str) {
    let mut alloc = MinifyAllocator::default();

    let minified = minify(file, &mut alloc).expect("minification failed");

    assert_eq!(
        cmp_documents(file, &minified),
        GraphqlSemanticEquality::Equal,
        "documents are not equal"
    );
}
