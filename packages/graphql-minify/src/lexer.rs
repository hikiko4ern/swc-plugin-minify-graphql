use bumpalo::Bump;
use bumpalo::collections::String as BumpaloString;
use logos::{Lexer, Logos, Span};

use super::block_string::{BlockStringToken, dedent_block_lines_mut, print_block_string};
use crate::block_string::{BlockStringLines, PrintedBlockString};

#[derive(Debug, PartialEq, Clone, Default)]
/// An enumeration of errors that can occur during the lexing process.
pub enum LexingError {
    #[default]
    UnknownToken,
    /// First value is the index of the first character of the unterminated string
    UnterminatedString(Span),
}

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[\s,]+")]
#[logos(error = LexingError)]
pub(crate) enum Token {
    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token("=")]
    Equals,

    #[token("!")]
    Exclamation,

    #[token("?")]
    Question,

    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token("...")]
    Ellipsis,

    #[token(r#"""""#)]
    BlockStringDelimiter,

    #[regex(r#""([^"\\]+|\\.)*""#, validate_string)]
    String,

    #[regex("-?[0-9]+")]
    Int,

    #[regex("-?[0-9]+\\.[0-9]+(e-?[0-9]+)?")]
    Float,

    #[regex("true|false")]
    Bool,

    #[regex("@[a-zA-Z_][a-zA-Z0-9_]*")]
    Directive,

    #[regex("\\$[a-zA-Z_][a-zA-Z0-9_]*")]
    Variable,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    #[regex(r"#[^\r\n]*", logos::skip)]
    Comment,
}

pub(crate) fn parse_block_string<'bump>(
    lexer: &mut Lexer<Token>,
    alloc: &'bump Bump,
) -> PrintedBlockString<'bump> {
    let remainder = lexer.remainder();

    let mut block_string_lines = BlockStringLines::with_capacity_in(5, alloc);

    {
        let mut block_lexer = BlockStringToken::lexer(remainder);
        let mut current_line = BumpaloString::new_in(alloc);
        let mut max_line_length = 0;

        while let Some(Ok(token)) = block_lexer.next() {
            match token {
                BlockStringToken::NewLine => {
                    max_line_length = max_line_length.max(current_line.len());
                    block_string_lines.push(current_line);
                    current_line = BumpaloString::with_capacity_in(max_line_length, alloc);
                }
                BlockStringToken::Text
                | BlockStringToken::Quote
                | BlockStringToken::EscapeSeq
                | BlockStringToken::EscapedTripleQuote => {
                    current_line.push_str(block_lexer.slice());
                }
                BlockStringToken::TripleQuote => break,
            }
        }

        if !current_line.is_empty() {
            block_string_lines.push(current_line);
        }

        lexer.bump(remainder.len() - block_lexer.remainder().len());
    }

    dedent_block_lines_mut(&mut block_string_lines);
    print_block_string(&block_string_lines, alloc)
}

#[inline]
fn validate_string(lexer: &Lexer<Token>) -> Result<(), LexingError> {
    let str = lexer.slice().as_bytes();

    if have_newline(str) {
        Err(LexingError::UnterminatedString(lexer.span()))
    } else {
        Ok(())
    }
}

#[inline]
fn have_newline(text: &[u8]) -> bool {
    const USIZE_BYTES: usize = size_of::<usize>();

    // Fast path for small slices.
    if text.len() < 2 * USIZE_BYTES {
        return have_newline_naive(text);
    }

    memchr::memchr2(b'\n', b'\r', text).is_some()
}

#[inline]
fn have_newline_naive(text: &[u8]) -> bool {
    text.iter().any(|&b| b == b'\n' || b == b'\r')
}
