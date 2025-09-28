//! Implements semantic comparing of GraphQL documents on top of [`apollo_parser::Lexer`]
//! as [`apollo_parser`] itself [does not support it yet][semantic-diff-issue]
//!
//! [semantic-diff-issue]: https://github.com/apollographql/apollo-rs/issues/356

#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

mod description;

use apollo_parser::{Error, Lexer, Token};
pub use itertools::EitherOrBoth;
use itertools::Itertools as _;

use crate::description::Description;

#[derive(Debug, PartialEq, Eq)]
pub enum GraphqlSemanticEquality<'a, 'b> {
    /// documents are equal
    Equal,
    /// documents are not equal due to different tokens
    TokensAreDifferent(Token<'a>, Token<'b>),
    /// an error occurred while parsing one or both inputs
    ParsingError(EitherOrBoth<Error, Error>),
    /// the `right` sequence has been exhausted,
    /// but there are tokens left in the `left` sequence
    LeftNotExhausted(Result<Token<'a>, Error>),
    /// the `left` sequence has been exhausted,
    /// but there are tokens left in the `right` sequence
    RightNotExhausted(Result<Token<'b>, Error>),
}

/// Semantically compare two documents
///
/// Currently, documents are considered semantically equivalent if they consist of
/// equivalent sequences of tokens, not considering comments, whitespace and trivia tokens.
/// Block strings are dedented and whitespace-trimmed before comparison.
pub fn cmp_documents<'a, 'b>(left: &'a str, right: &'b str) -> GraphqlSemanticEquality<'a, 'b> {
    let left_lexer = NonTriviaLexer(Lexer::new(left));
    let right_lexer = NonTriviaLexer(Lexer::new(right));

    for tokens in left_lexer.zip_longest(right_lexer) {
        match tokens {
            EitherOrBoth::Both(left, right) => match (left, right) {
                (Ok(left), Ok(right)) => {
                    let left_data = left.data();
                    let right_data = right.data();

                    let are_equal =
						// if both are block strings
                        if left_data.starts_with(Description::TRIPLE_QUOTES) && right_data.starts_with(Description::TRIPLE_QUOTES) {
                            Description::new_cleaned(left_data) == Description::new_cleaned(right_data)
                        } else {
                            left_data == right_data
                        };

                    if !are_equal {
                        return GraphqlSemanticEquality::TokensAreDifferent(left, right);
                    }
                }
                (Err(left), Err(right)) => {
                    return GraphqlSemanticEquality::ParsingError(EitherOrBoth::Both(left, right));
                }
                (Err(left), _) => {
                    return GraphqlSemanticEquality::ParsingError(EitherOrBoth::Left(left));
                }
                (_, Err(right)) => {
                    return GraphqlSemanticEquality::ParsingError(EitherOrBoth::Right(right));
                }
            },
            EitherOrBoth::Left(left) => return GraphqlSemanticEquality::LeftNotExhausted(left),
            EitherOrBoth::Right(right) => return GraphqlSemanticEquality::RightNotExhausted(right),
        }
    }

    GraphqlSemanticEquality::Equal
}

/// a [`Lexer`] Iterator adaptor which skips comment, whitespace and trivia tokens
struct NonTriviaLexer<'a>(Lexer<'a>);

impl<'a> Iterator for NonTriviaLexer<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        use apollo_parser::TokenKind::{Comma, Comment, Whitespace};

        loop {
            let token = self.0.next()?;

            let token = match token {
                Ok(token) => token,
                err => return Some(err),
            };

            if !matches!(token.kind(), Whitespace | Comment | Comma) {
                return Some(Ok(token));
            }
        }
    }
}
