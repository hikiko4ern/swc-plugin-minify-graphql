//! GraphQL minification helpers
//!
//! Logic is built on several axioms:
//! - [`Tpl`] always contains at least one [`TplElement`]
//! - [`Tpl`] always starts with [`TplElement`], which is potentially empty
//! - [`TplElement`] and [`Expr`] always alternate,
//!   i.e. [`Expr`] always follows [`TplElement`] and [`Expr`] always follows [`TplElement`]
//!
//! Since separate parts of GraphQL ([`TplElement`]) and not the whole query ([`Tpl`]) are minified,
//! it is necessary to manually surround [`Expr`] with spaces in some cases,
//! so that identifiers from the expression are not glued with identifiers from the parts
//! after expression substitution in runtime:\
//! `id image{${IMAGE} url}` minified in parts will turn into `id image{${IMAGE}url}`,
//! which in the case where `IMAGE` is an identifier `id` will turn into `id image{idurl}`
//! instead of the correct `id image{id url}`
//!
//! Surrounds with spaces if:
//! - [`Expr`] is followed by [`Expr`], and [`TplElement`] does not end with one of [`Punctuator`]s
//! - the current [`TplElement`] was preceded by [`Expr`], and [`TplElement`] does not start with one of [`Punctuator`]s
//!
//! [`Tpl`]: swc_core::ecma::ast::Tpl
//! [`TplElement`]: swc_core::ecma::ast::TplElement
//! [`Expr`]: swc_core::ecma::ast::Expr
//! [`Punctuator`]: https://spec.graphql.org/October2021/#Punctuator
// spell-checker: ignore idurl

use swc_core::atoms::{Atom, Wtf8Atom};
use swc_core::common::errors::HANDLER;
use swc_core::ecma::ast::{Str, Tpl, TplElement};

use crate::str_span::StrSpan;

/// [`Punctuator`] characters
///
/// <div class="warning">
///
/// punctuator `...` is not checked for a complete match --- any `.` is considered as a part of `...`,
/// since the only [`Token`] whose beginning or end is `.` is `...`
///
/// cases where [`Expr`] breaks [`Token`] (e.g. `some${LONG}FieldName`, `123.${FP}` or `"some. ${STR} string"`)
/// are considered invalid and are not handled properly
///
/// </div>
///
/// [`Punctuator`]: https://spec.graphql.org/October2021/#Punctuator
/// [`Token`]: https://spec.graphql.org/October2021/#Token
/// [`Expr`]: swc_core::ecma::ast::Expr
const PUNCTUATORS: &[char] = &[
    '!', '$', '&', '(', ')', '.', ':', '@', '[', ']', '{', ',', '}',
];

#[derive(Default)]
pub(crate) struct Minifier {
    alloc: graphql_minify::MinifyAllocator,
}

impl Minifier {
    /// minifies [`Str`]
    pub fn minify_str(&mut self, str: &mut Str) {
        if let Some(value) = str.value.as_str()
            && let Some(min) = self.try_minify(value, str)
        {
            str.value = Wtf8Atom::new(min);
            str.raw = None;
        }
    }

    /// minifies [`Tpl`]
    pub fn minify_tpl(&mut self, tpl: &mut Tpl) {
        // If there are no expressions, we take the shortest path and
        // minify the single `TplElement` without additional checks

        if tpl.exprs.is_empty() {
            let tpl_el = unsafe { tpl.quasis.get_unchecked_mut(0) };

            if let Some(min) = self.try_minify(tpl_el_value(tpl_el), tpl_el) {
                tpl_el.raw = Atom::new(min);
                tpl_el.cooked = Some(tpl_el.raw.clone().into());
            }

            return;
        }

        // minify all `TplElement`s, surrounding expressions with spaces if necessary

        let mut expr_it = tpl.exprs.iter();
        let mut has_prev_expr = false;
        let last_quasis_index = tpl.quasis.len() - 1;

        for (i, tpl_el) in tpl.quasis.iter_mut().enumerate() {
            let next_is_expr = expr_it.next().is_some();

            if let Some(mut min) = self.try_minify(tpl_el_value(tpl_el), tpl_el) {
                let is_empty = min.is_empty();
                let mut is_space_inserted = false;

                if has_prev_expr
                    && !(is_empty && last_quasis_index == i)
                    && !min.starts_with(PUNCTUATORS)
                {
                    min.insert(0, ' ');
                    is_space_inserted = true;
                }

                if next_is_expr
                    && !(is_empty && (is_space_inserted || i == 0))
                    && !min.ends_with(PUNCTUATORS)
                {
                    min.push(' ');
                }

                tpl_el.raw = Atom::new(min);
                tpl_el.cooked = Some(tpl_el.raw.clone().into());
            }

            has_prev_expr = next_is_expr;
        }
    }

    fn try_minify<Str>(&mut self, code: &str, str: &Str) -> Option<String>
    where
        Str: StrSpan,
    {
        if code.is_empty() {
            return None;
        }

        match graphql_minify::minify(code, &mut self.alloc) {
            Ok(min) => Some(min),
            Err(err) => HANDLER.with(|handler| {
                let err_value_span = err.span();
                let is_single_byte_err_span = (err_value_span.end - err_value_span.start) == 1;

                let err_file_span = str
                    .value_span()
                    .from_inner_byte_pos(err_value_span.start, err_value_span.end);

                handler
                    .struct_span_err(str.outer_span(), "failed to minify GraphQL")
                    .span_label(
                        err_file_span,
                        if is_single_byte_err_span {
                            format!("{} at {}", err.as_str(), err_file_span.lo.0)
                        } else {
                            format!(
                                "{} at {}-{}",
                                err.as_str(),
                                err_file_span.lo.0,
                                err_file_span.hi.0
                            )
                        },
                    )
                    .emit();
                None
            }),
        }
    }
}

fn tpl_el_value(tpl_el: &TplElement) -> &str {
    tpl_el
        .cooked
        .as_ref()
        .and_then(|cooked| cooked.as_str())
        .unwrap_or(tpl_el.raw.as_str())
}
