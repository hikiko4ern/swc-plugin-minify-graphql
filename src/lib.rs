#![warn(clippy::pedantic)]
#![allow(clippy::default_trait_access, clippy::module_name_repetitions)]

mod str_span;
mod visitor;

use swc_core::common::comments::Comments;
use swc_core::common::{BytePos, Spanned};
use swc_core::ecma::ast::{Program, Str, Tpl};
use swc_core::ecma::transforms::testing::test_inline;
use swc_core::ecma::visit::{VisitMut, VisitMutWith, noop_visit_mut_type};
use swc_core::plugin::plugin_transform;
use swc_core::plugin::proxies::{PluginCommentsProxy, TransformPluginProgramMetadata};

use crate::visitor::Minifier;

pub struct MinifyGraphqlVisitor<C: Comments> {
    comments: C,
    minifier: Minifier,
}

impl<C: Comments> MinifyGraphqlVisitor<C> {
    fn new(comments: C) -> Self {
        Self {
            comments,
            minifier: Minifier::default(),
        }
    }

    fn is_graphql(&self, span_lo: BytePos) -> bool {
        self.comments
            .get_leading(span_lo)
            .as_ref()
            .and_then(|c| c.first())
            .is_some_and(|c| {
                c.text
                    .to_ascii_lowercase()
                    .trim_matches(|c: char| c == '*' || c.is_whitespace())
                    == "graphql"
            })
    }
}

impl<C: Comments> VisitMut for MinifyGraphqlVisitor<C> {
    noop_visit_mut_type!();

    fn visit_mut_str(&mut self, n: &mut Str) {
        if self.is_graphql(n.span_lo()) {
            self.minifier.minify_str(n);
        }
    }

    fn visit_mut_tpl(&mut self, n: &mut Tpl) {
        if self.is_graphql(n.span_lo()) {
            self.minifier.minify_tpl(n);
        }
    }
}

#[plugin_transform]
#[must_use]
pub fn swc_plugin_minify_graphql(
    mut program: Program,
    _: TransformPluginProgramMetadata,
) -> Program {
    program.visit_mut_with(&mut MinifyGraphqlVisitor::new(PluginCommentsProxy));
    program
}

// Note: swc ignores comments in the expected result, so they are removed on purpose

test_inline!(
    #[allow(clippy::default_trait_access)]
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    without_comment,
    r"
        export const FRAGMENT = `
            id
            name
            image {
                id
                url
            }
        `;
    ",
    r"
        export const FRAGMENT = `
            id
            name
            image {
                id
                url
            }
        `;
    "
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    str_literal,
    r#"export const FRAGMENT = /** GraphQL */ "id  \n  url";"#,
    r#"export const FRAGMENT = "id url";"#
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    tpl_without_expressions,
    r"
        export const FRAGMENT = /** GraphQL */ `
            id
            name
            image {
                id
                url
            }
        `;
    ",
    r"export const FRAGMENT = `id name image{id url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    various_comments,
    r"
        export const SINGLE_LINE = /* GraphQL */ `
            id
            url
        `;
        export const MULTI_LINE = /** GraphQL */ `
            id
            url
        `;

        export const SINGLE_LINE_LOWERCASED = /* graphql */ `
            id
            url
        `;
        export const MULTI_LINE_LOWERCASED = /* graphql */ `
            id
            url
        `;

        export const UH_WHAT_IS_THAT = /* *** * gRaPhQl * *** */ `
            id
            url
        `;
    ",
    r"
        export const SINGLE_LINE = `id url`;
        export const MULTI_LINE = `id url`;

        export const SINGLE_LINE_LOWERCASED = `id url`;
        export const MULTI_LINE_LOWERCASED = `id url`;

        export const UH_WHAT_IS_THAT = `id url`;
    "
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    with_leading_expr,
    r"
        export const FRAGMENT = /** GraphQL */ `
            id
            name
            image {
                ${IMAGE}
                url
            }
        `;
    ",
    r"export const FRAGMENT = `id name image{${IMAGE} url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    with_expr_in_middle,
    r"
        export const FRAGMENT = /** GraphQL */ `
            id
            name
            image {
                id
                ${IMAGE}
                url
            }
        `;
    ",
    r"export const FRAGMENT = `id name image{id ${IMAGE} url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    with_tail_expr,
    r"
        export const FRAGMENT = /** GraphQL */ `
            id
            name
            image {
                id
                ${IMAGE}
            }
        `;
    ",
    r"export const FRAGMENT = `id name image{id ${IMAGE}}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    empty,
    r"export const FRAGMENT = /** GraphQL */ ``;",
    r"export const FRAGMENT = ``;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    whitespace_only,
    r"
        export const FRAGMENT = /** GraphQL */ `


        `;
    ",
    r"export const FRAGMENT = ``;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    whitespaces_with_expr,
    r"
        export const FRAGMENT = /** GraphQL */ `
            ${FRAGMENTS}
        `;
    ",
    r"export const FRAGMENT = `${FRAGMENTS}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    whitespaces_with_multiple_exprs,
    r"
        export const FRAGMENT = /** GraphQL */ `
            ${A}
            ${B}
        `;
    ",
    r"export const FRAGMENT = `${A} ${B}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    expr_breaks_token,
    r#"
        export const FLOAT_INVALID  = /** GraphQL */ `123.${FP}`;
        export const FLOAT_VALID  = /** GraphQL */ `123.4${FP}`;
        export const STRING = /** GraphQL */ `"Hello${PRETTY}world!"`;
        export const FIELD  = /** GraphQL */ `
        	id
        	some${LONG}FieldName
        `;
    "#,
    r#"
        export const FLOAT_INVALID  = `123.${FP}`; // left unchanged due to parsing error
        export const FLOAT_VALID  = `123.4 ${FP}`;
        export const STRING = `"Hello${PRETTY}world!"`; // left unchanged due to parsing error
        export const FIELD  = `id some ${LONG} FieldName`; // extra spaces around `${LONG}` are expected
    "#
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_basic_full,
    r"
        const QUERY = /* GraphQL */ `
            query ($id: ID!) {
                image (id: $id) {
                    ...img
                }
            }

            fragment img on Image {
                id
                url
            }
        `;
    ",
    r"const QUERY = /* GraphQL */ `query($id:ID!){image(id:$id){...img}}fragment img on Image{id url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_basic_fragment_fields,
    r"
        const IMAGE_FIELDS = /* GraphQL */ `
            id
            url
        `;
    ",
    r"const IMAGE_FIELDS = /* GraphQL */ `id url`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_basic_fragment,
    r"
        const IMAGE_FRAGMENT = /* GraphQL */ `
            fragment image on Image {
                id
                url
            }
        `;
    ",
    r"const IMAGE_FRAGMENT = /* GraphQL */ `fragment image on Image{id url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_template_literals,
    r"
        const IMAGE_FRAGMENT = /* GraphQL */ `
            fragment image on Image {
                id
                url
            }
        `;
    ",
    r"const IMAGE_FRAGMENT = /* GraphQL */ `fragment image on Image{id url}`;"
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_template_literals_with_expressions,
    r"
        const IMAGE = /* GraphQL */ `
            id
            url
        `;

        const ENTITY = /* GraphQL */ `
            id
            image {
                ${IMAGE}
                previewUrl
            }
        `;
    ",
    r"
        const IMAGE = /* GraphQL */ `id url`;

        const ENTITY = /* GraphQL */ `id image{${IMAGE} previewUrl}`;
    "
);

test_inline!(
    Default::default(),
    |tr| swc_core::ecma::visit::visit_mut_pass(MinifyGraphqlVisitor::new(tr.comments.clone())),
    readme_template_literals_with_expressions_invalid,
    r"
        const LONG = 'Long';

        const FIELD = /* GraphQL */ `
            id
            some${LONG}FieldName
        `;
    ",
    r"
        const LONG = 'Long';

        const FIELD = /* GraphQL */ `id some ${LONG} FieldName`;
    "
);
