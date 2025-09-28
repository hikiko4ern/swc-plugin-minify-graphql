#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

mod block_string;
mod lexer;
mod minify_alloc;

use logos::{Logos, Span};

use crate::lexer::{LexingError, Token, parse_block_string};
pub use crate::minify_alloc::MinifyAllocator;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum MinifyError {
    UnknownToken(Span),
    UnterminatedString(Span),
}

impl MinifyError {
    pub const fn as_str(&self) -> &str {
        match self {
            MinifyError::UnknownToken(_) => "unknown token",
            MinifyError::UnterminatedString(_) => "unterminated string",
        }
    }

    pub const fn span(&self) -> &Span {
        match self {
            MinifyError::UnknownToken(span) | MinifyError::UnterminatedString(span) => span,
        }
    }
}

/// Strips characters that are not significant to the validity or execution of a GraphQL document.
///
/// It is functionally equivalent to [`stripIgnoredCharacters`](https://graphql-js.org/api/function/stripignoredcharacters/) defined in the [GraphQL spec](https://spec.graphql.org/June2018/#sec-Source-Text.Ignored-Tokens).
///
/// This function takes a value that implements the `AsRef<str>` trait, allowing for flexible input types
/// that can be treated as a string slice. It returns a `Result` with the minified string or an error
/// if the lexing process fails.
///
/// # Examples
///
/// ```
/// use graphql_minify::{minify, MinifyAllocator};
///
/// let original = r#"
/// query SomeQuery($foo: String!, $bar: String) {
///   someField(foo: $foo, bar: $bar) {
///    ...fragmented
///  }
/// }
/// "#;
/// let mut alloc = MinifyAllocator::default();
/// let minified = minify(original, &mut alloc).unwrap();
///
/// assert_eq!(minified, "query SomeQuery($foo:String!$bar:String){someField(foo:$foo bar:$bar){...fragmented}}");
/// ```
///
/// # Errors
///
/// This function will return an error if the lexing process encounters an unexpected character.
///
/// # Panics
///
/// This function does not panic.
///
/// # Safety
///
/// This function does not use any unsafe code.
pub fn minify<T: AsRef<str>>(value: T, alloc: &mut MinifyAllocator) -> Result<String, MinifyError> {
    let value = value.as_ref();
    let mut lexer = Token::lexer(value);
    let mut result = String::with_capacity(value.len());
    let mut last_token = None;

    while let Some(token) = lexer.next() {
        let token = match token {
            Ok(token) => token,
            Err(e) => {
                return Err(match e {
                    LexingError::UnknownToken => MinifyError::UnknownToken(lexer.span()),
                    LexingError::UnterminatedString(span) => MinifyError::UnterminatedString(span),
                });
            }
        };

        if needs_space(&token, last_token.as_ref()) {
            result.push(' ');
        }

        match token {
            Token::BlockStringDelimiter => {
                result.push_str(parse_block_string(&mut lexer, &alloc.block_string).as_ref());
                alloc.block_string.reset();
            }
            _ => result.push_str(lexer.slice()),
        }
        last_token = Some(token);
    }

    Ok(result)
}

fn is_non_punctuator(token: &Token) -> bool {
    !matches!(
        token,
        Token::BraceOpen
            | Token::BraceClose
            | Token::ParenOpen
            | Token::ParenClose
            | Token::BracketOpen
            | Token::BracketClose
            | Token::Colon
            | Token::Equals
            | Token::Exclamation
            | Token::Question
            | Token::Ellipsis
            | Token::Ampersand
            | Token::Pipe
            | Token::Variable
            | Token::Directive
    )
}

fn needs_space_after_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Variable | Token::String | Token::Identifier | Token::Directive
    )
}

fn needs_space_before_token(token: &Token) -> bool {
    matches!(token, Token::Identifier | Token::BlockStringDelimiter)
}

fn needs_space(cur_token: &Token, last_token: Option<&Token>) -> bool {
    match last_token {
        Some(last) if is_non_punctuator(last) => is_non_punctuator(cur_token),
        Some(last) if needs_space_after_token(last) => needs_space_before_token(cur_token),
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use crate::MinifyError;

    fn minify<T: AsRef<str>>(value: T) -> Result<String, MinifyError> {
        super::minify(value, &mut crate::MinifyAllocator::default())
    }

    #[test]
    fn strips_ignored_characters_from_graphql_query_document() {
        let query = indoc! {r"
      query SomeQuery($foo: String!, $bar: String) {
        someField(foo: $foo, bar: $bar) {
          a
          b {
            c
            d
          }
        }
      }
    "};

        let expected =
            "query SomeQuery($foo:String!$bar:String){someField(foo:$foo bar:$bar){a b{c d}}}";

        assert_eq!(minify(query).unwrap(), expected);
    }

    #[test]
    fn strips_ignored_characters_from_graphql_schema_document() {
        let schema = indoc! {r#"
      """
      Type description
      """
      type Foo {
        """
        Field description
        """
        bar: String
      }
    "#};

        let expected = r#""""Type description""" type Foo{"""Field description""" bar:String}"#;

        assert_eq!(minify(schema).unwrap(), expected);
    }

    #[test]
    fn errs_on_invalid_token() {
        let query = "{ foo(arg: \"\n\"";

        assert!(matches!(
            minify(query),
            Err(MinifyError::UnterminatedString(_))
        ));
    }

    #[test]
    fn strips_non_parsable_document() {
        let query = r#"{ foo(arg: "str""#;
        let expected = r#"{foo(arg:"str""#;

        assert_eq!(minify(query).unwrap(), expected);
    }

    #[test]
    fn strips_documents_with_only_ignored_characters() {
        assert_eq!(minify("\n").unwrap(), "");
        assert_eq!(minify(",").unwrap(), "");
        assert_eq!(minify(",,").unwrap(), "");
        assert_eq!(minify("#comment\n, \n").unwrap(), "");
    }

    #[test]
    fn strips_leading_and_trailing_ignored_tokens() {
        assert_eq!(minify("\n1").unwrap(), "1");
        assert_eq!(minify(",1").unwrap(), "1");
        assert_eq!(minify(",,1").unwrap(), "1");
        assert_eq!(minify("#comment\n, \n1").unwrap(), "1");

        assert_eq!(minify("1\n").unwrap(), "1");
        assert_eq!(minify("1,").unwrap(), "1");
        assert_eq!(minify("1,,").unwrap(), "1");
        assert_eq!(minify("1#comment\n, \n").unwrap(), "1");
    }

    #[test]
    fn strips_ignored_tokens_between_punctuator_tokens() {
        assert_eq!(minify("[,)").unwrap(), "[)");
        assert_eq!(minify("[\r)").unwrap(), "[)");
        assert_eq!(minify("[\r\r)").unwrap(), "[)");
        assert_eq!(minify("[\r,)").unwrap(), "[)");
        assert_eq!(minify("[,\n)").unwrap(), "[)");
    }

    #[test]
    fn strips_ignored_tokens_between_punctuator_and_non_punctuator_tokens() {
        assert_eq!(minify("[,1").unwrap(), "[1");
        assert_eq!(minify("[\r1").unwrap(), "[1");
        assert_eq!(minify("[\r\r1").unwrap(), "[1");
        assert_eq!(minify("[\r,1").unwrap(), "[1");
        assert_eq!(minify("[,\n1").unwrap(), "[1");
    }

    #[test]
    fn replace_ignored_tokens_between_non_punctuator_tokens_and_spread_with_space() {
        assert_eq!(minify("a ...").unwrap(), "a...");
        assert_eq!(minify("1 ...").unwrap(), "1...");
        assert_eq!(minify("1 ... ...").unwrap(), "1......");
    }

    #[test]
    fn replace_ignored_tokens_between_non_punctuator_tokens_with_space() {
        assert_eq!(minify("1 2").unwrap(), "1 2");
        assert_eq!(minify("\"\" \"\"").unwrap(), "\"\" \"\"");
        assert_eq!(minify("a b").unwrap(), "a b");

        assert_eq!(minify("a,1").unwrap(), "a 1");
        assert_eq!(minify("a,,1").unwrap(), "a 1");
        assert_eq!(minify("a  1").unwrap(), "a 1");
        assert_eq!(minify("a \t 1").unwrap(), "a 1");
    }

    #[test]
    fn does_not_strip_ignored_tokens_embedded_in_the_string() {
        assert_eq!(minify("\" \"").unwrap(), "\" \"");
        assert_eq!(minify("\",\"").unwrap(), "\",\"");
        assert_eq!(minify("\",,\"").unwrap(), "\",,\"");
        assert_eq!(minify("\",|\"").unwrap(), "\",|\"");
    }

    #[test]
    fn does_not_strip_ignored_tokens_embedded_in_the_block_string() {
        assert_eq!(minify("\"\"\",\"\"\"").unwrap(), "\"\"\",\"\"\"");
        assert_eq!(minify("\"\"\",,\"\"\"").unwrap(), "\"\"\",,\"\"\"");
        assert_eq!(minify("\"\"\",|\"\"\"").unwrap(), "\"\"\",|\"\"\"");
    }

    #[test]
    fn strips_ignored_characters_inside_block_strings() {
        assert_eq!(minify(r#""""""""#).unwrap(), r#""""""""#);
        assert_eq!(minify(r#"""" """"#).unwrap(), r#""""""""#);

        assert_eq!(minify(r#""""a""""#).unwrap(), r#""""a""""#);
        assert_eq!(minify(r#"""" a""""#).unwrap(), r#"""" a""""#);
        assert_eq!(minify(r#"""" a """"#).unwrap(), r#"""" a """"#);

        assert_eq!(minify("\"\"\"\n\"\"\"").unwrap(), r#""""""""#);
        assert_eq!(minify("\"\"\"a\nb\"\"\"").unwrap(), "\"\"\"a\nb\"\"\"");
        assert_eq!(minify("\"\"\"a\rb\"\"\"").unwrap(), "\"\"\"a\nb\"\"\"");
        assert_eq!(minify("\"\"\"a\r\nb\"\"\"").unwrap(), "\"\"\"a\nb\"\"\"");
        assert_eq!(
            minify("\"\"\"a\r\n\nb\"\"\"").unwrap(),
            "\"\"\"a\n\nb\"\"\""
        );

        assert_eq!(minify("\"\"\"\\\n\"\"\"").unwrap(), "\"\"\"\\\n\"\"\"");
        assert_eq!(minify("\"\"\"\"\n\"\"\"").unwrap(), "\"\"\"\"\n\"\"\"");
        assert_eq!(
            minify("\"\"\"\\\"\"\"\n\"\"\"").unwrap(),
            "\"\"\"\\\"\"\"\"\"\""
        );

        assert_eq!(
            minify("\"\"\"\na\n b\"\"\"").unwrap(),
            "\"\"\"\na\n b\"\"\""
        );
        assert_eq!(minify("\"\"\"\n a\n b\"\"\"").unwrap(), "\"\"\"a\nb\"\"\"");
        assert_eq!(
            minify("\"\"\"\na\n b\nc\"\"\"").unwrap(),
            "\"\"\"a\n b\nc\"\"\""
        );
    }

    #[test]
    fn test_kitchen_sink_query() {
        let query = include_str!("../test_data/kitchen_sink_query.graphql");
        let expected = include_str!("../test_data/kitchen_sink_query_expected.graphql");

        assert_eq!(minify(query).unwrap(), expected);
    }

    #[test]
    fn test_kitchen_sink_schema() {
        let schema = include_str!("../test_data/valid/kitchen_sink_schema.graphql");
        let expected = include_str!("../test_data/valid/kitchen_sink_schema_expected.graphql");

        assert_eq!(minify(schema).unwrap(), expected);
    }

    /// [`Token::String`]'s regex causes [stack overflow]
    ///
    /// release build optimizations help to avoid at least some of the stack overflows,
    /// so this test is performed in release mode only
    ///
    /// [`Token::String`]: super::Token::String
    /// [stack overflow]: https://github.com/maciejhirsz/logos/issues/384
    #[cfg_attr(not(debug_assertions), test)]
    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn test_stack_overflow() {
        let schema = include_str!("../test_data/fuzz/stack_overflow");
        assert_eq!(
            minify(schema),
            Err(MinifyError::UnknownToken(11..schema.len()))
        );
    }
}
