use std::ops::Deref;

use bumpalo::{
    Bump,
    collections::{String as BumpaloString, Vec as BumpaloVec},
};
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub(crate) enum BlockStringToken {
    #[token("\"")]
    Quote,

    #[regex(r#"\n|\r\n|\r"#)]
    NewLine,

    #[regex(r#"\\""""#)]
    EscapedTripleQuote,

    #[regex(r#"\\"#)]
    EscapeSeq,

    #[regex(r#"[^"\r\n\\]+"#)]
    Text,

    #[token(r#"""""#)]
    TripleQuote,
}

pub(crate) struct BlockStringLines<'bump> {
    lines: BumpaloVec<'bump, BumpaloString<'bump>>,
    total_len: usize,
}

impl<'bump> BlockStringLines<'bump> {
    #[cfg(test)]
    pub fn new_in(alloc: &'bump Bump) -> Self {
        Self {
            lines: BumpaloVec::new_in(alloc),
            total_len: 0,
        }
    }

    pub fn with_capacity_in(capacity: usize, alloc: &'bump Bump) -> Self {
        Self {
            lines: BumpaloVec::with_capacity_in(capacity, alloc),
            total_len: 0,
        }
    }

    pub fn push(&mut self, line: BumpaloString<'bump>) {
        self.total_len += line.len();
        self.lines.push(line);
    }
}

impl<'bump> Deref for BlockStringLines<'bump> {
    type Target = [BumpaloString<'bump>];

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

pub(crate) enum PrintedBlockString<'bump> {
    Empty,
    String(BumpaloString<'bump>),
}

impl AsRef<str> for PrintedBlockString<'_> {
    fn as_ref(&self) -> &str {
        match self {
            PrintedBlockString::Empty => r#""""""""#,
            PrintedBlockString::String(str) => str.as_str(),
        }
    }
}

pub(crate) fn print_block_string<'bump>(
    lines: &BlockStringLines<'bump>,
    alloc: &'bump Bump,
) -> PrintedBlockString<'bump> {
    const TRIPLE_QUOTES: &str = r#"""""#;

    let [start_lines @ .., last_line] = lines.lines.as_slice() else {
        return PrintedBlockString::Empty;
    };

    let with_leading_new_line = lines.len() > 1
        && lines[1..].iter().all(|line| {
            line.as_bytes()
                .first()
                .copied()
                .is_none_or(is_graphql_whitespace)
        });

    let with_trailing_newline = last_line.ends_with(['"', '\\']) && !last_line.ends_with(r#"\""""#);

    let mut result = BumpaloString::with_capacity_in(
        lines.total_len
            + (TRIPLE_QUOTES.len() * 2)
            + usize::from(with_leading_new_line)
            + (lines.len() - 1)
            + usize::from(with_trailing_newline),
        alloc,
    );

    result.push_str(TRIPLE_QUOTES);

    if with_leading_new_line {
        result.push('\n');
    }

    for line in start_lines {
        result.push_str(line);
        result.push('\n');
    }

    result.push_str(last_line);

    if with_trailing_newline {
        result.push('\n');
    }

    result.push_str(TRIPLE_QUOTES);

    PrintedBlockString::String(result)
}

pub(crate) fn dedent_block_lines_mut(lines: &mut BlockStringLines) {
    let mut common_indent = usize::MAX;
    let mut first_non_empty_line = None;
    let mut last_non_empty_line = None;

    for (i, line) in lines.iter().enumerate() {
        let indent = leading_whitespace(line);

        if indent < line.len() {
            first_non_empty_line.get_or_insert(i);
            last_non_empty_line = Some(i);

            if i != 0 && indent < common_indent {
                common_indent = common_indent.min(indent);
            }
        }
    }

    let buf = &mut lines.lines;

    match (first_non_empty_line, last_non_empty_line) {
        (Some(start), Some(end)) => {
            for line in buf.iter_mut().skip(1) {
                if line.len() > common_indent {
                    lines.total_len -= common_indent;
                    line.drain(0..common_indent);
                } else {
                    lines.total_len -= line.len();
                    line.clear();
                }
            }

            buf.drain(..start);
            buf.drain((end + 1 - start)..);
        }
        _ => buf.clear(),
    }
}

fn is_graphql_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

fn leading_whitespace(s: &str) -> usize {
    s.as_bytes()
        .iter()
        .position(|&b| !is_graphql_whitespace(b))
        .unwrap_or(s.len())
}

#[cfg(test)]
mod test_dedent {
    use super::{BlockStringLines, dedent_block_lines_mut};

    fn get_dedented_vec(lines: &[&str]) -> Vec<String> {
        use std::cell::RefCell;

        use bumpalo::{Bump, collections::String as BumpaloString};

        thread_local! {
            static BUMP: RefCell<Bump> = RefCell::new(Bump::new());
        }

        BUMP.with_borrow_mut(|bump| {
            let mut bsl = BlockStringLines::with_capacity_in(lines.len(), bump);

            for line in lines {
                bsl.push(BumpaloString::from_str_in(line, bump));
            }

            dedent_block_lines_mut(&mut bsl);

            let lines = bsl
                .lines
                .iter()
                .map(|str| String::from(str.as_str()))
                .collect::<Vec<_>>();

            drop(bsl);
            bump.reset();

            lines
        })
    }

    #[test]
    fn does_not_dedent_first_line() {
        assert_eq!(get_dedented_vec(&["  a"]), &["  a"]);
        assert_eq!(get_dedented_vec(&[" a", "  b"]), &[" a", "b"]);
    }

    #[test]
    fn removes_minimal_indentation_length() {
        assert_eq!(get_dedented_vec(&["", " a", "  b"]), &["a", " b"]);
        assert_eq!(get_dedented_vec(&["", "  a", " b"]), &[" a", "b"]);
        assert_eq!(
            get_dedented_vec(&["", "  a", " b", "c"]),
            &["  a", " b", "c"]
        );
    }

    #[test]
    fn dedent_both_tab_and_space_as_single_character() {
        assert_eq!(
            get_dedented_vec(&["", "\ta", "          b"]),
            &["a", "         b"]
        );
        assert_eq!(
            get_dedented_vec(&["", "\t a", "          b"]),
            &["a", "        b"]
        );
        assert_eq!(
            get_dedented_vec(&["", " \t a", "          b"]),
            &["a", "       b"]
        );
    }

    #[test]
    fn dedent_do_not_take_empty_lines_into_account() {
        assert_eq!(get_dedented_vec(&["a", "", " b"]), &["a", "", "b"]);
        assert_eq!(get_dedented_vec(&["a", " ", "  b"]), &["a", "", "b"]);
    }

    #[test]
    fn removes_uniform_indentation_from_a_string() {
        let lines = vec![
            "",
            "    Hello,",
            "      World!",
            "",
            "    Yours,",
            "      GraphQL.",
        ];
        assert_eq!(
            get_dedented_vec(&lines),
            &["Hello,", "  World!", "", "Yours,", "  GraphQL.",]
        );
    }

    #[test]
    fn removes_empty_leading_and_trailing_lines() {
        let lines = vec![
            "",
            "",
            "    Hello,",
            "      World!",
            "",
            "    Yours,",
            "      GraphQL.",
            "",
            "",
        ];
        assert_eq!(
            get_dedented_vec(&lines),
            &["Hello,", "  World!", "", "Yours,", "  GraphQL.",]
        );
    }

    #[test]
    fn removes_blank_leading_and_trailing_lines() {
        let lines = vec![
            "  ",
            "        ",
            "    Hello,",
            "      World!",
            "",
            "    Yours,",
            "      GraphQL.",
            "        ",
            "  ",
        ];
        assert_eq!(
            get_dedented_vec(&lines),
            &["Hello,", "  World!", "", "Yours,", "  GraphQL.",]
        );
    }

    #[test]
    fn retains_indentation_from_first_line() {
        let lines = vec![
            "    Hello,",
            "      World!",
            "",
            "    Yours,",
            "      GraphQL.",
        ];
        assert_eq!(
            get_dedented_vec(&lines),
            &["    Hello,", "  World!", "", "Yours,", "  GraphQL.",]
        );
    }

    #[test]
    fn does_not_alter_trailing_spaces() {
        let lines = vec![
            "               ",
            "    Hello,     ",
            "      World!   ",
            "               ",
            "    Yours,     ",
            "      GraphQL. ",
            "               ",
        ];
        assert_eq!(
            get_dedented_vec(&lines),
            &[
                "Hello,     ",
                "  World!   ",
                "           ",
                "Yours,     ",
                "  GraphQL. ",
            ]
        );
    }
}

#[cfg(test)]
mod test_print {
    fn print_block_string<I: AsRef<str>>(input: I) -> String {
        use std::cell::RefCell;

        use bumpalo::{Bump, collections::String as BumpaloString};

        use super::BlockStringLines;

        thread_local! {
            static BUMP: RefCell<Bump> = RefCell::new(Bump::new());
        }

        BUMP.with_borrow_mut(|bump| {
            let mut lines = BlockStringLines::new_in(bump);

            for line in input.as_ref().lines() {
                lines.push(BumpaloString::from_str_in(
                    line.replace(r#"""""#, r#"\""""#).as_str(),
                    bump,
                ));
            }

            let res = String::from(super::print_block_string(&lines, bump).as_ref());

            drop(lines);
            bump.reset();

            res
        })
    }

    #[test]
    fn does_not_escape_characters() {
        let str = r" \ / \b \f \n \r \t";
        assert_eq!(print_block_string(str), r#"""" \ / \b \f \n \r \t""""#);
    }

    #[test]
    fn by_default_print_block_strings_as_single_line() {
        let str = r"one liner";
        assert_eq!(print_block_string(str), r#""""one liner""""#);
    }

    #[test]
    fn by_default_print_block_strings_ending_with_triple_quotation_as_multi_line() {
        let str = r#"triple quotation """"#;
        assert_eq!(print_block_string(str), r#""""triple quotation \"""""""#);
    }

    #[test]
    fn correctly_prints_single_line_with_leading_space() {
        let str = "    space-led string";
        assert_eq!(print_block_string(str), r#""""    space-led string""""#);
    }

    #[test]
    fn correctly_prints_single_line_with_leading_space_and_trailing_quotation() {
        let str = "    space-led value \"quoted string\"";
        assert_eq!(
            print_block_string(str),
            r#""""    space-led value "quoted string"
""""#
        );
    }

    #[test]
    fn correctly_prints_single_line_with_trailing_backslash() {
        let str = "backslash \\";
        assert_eq!(
            print_block_string(str),
            r#""""backslash \
""""#
        );
    }

    #[test]
    fn correctly_prints_multi_line_with_internal_indent() {
        let str = "no indent\n with indent";
        assert_eq!(
            print_block_string(str),
            r#""""
no indent
 with indent""""#
        );
    }

    #[test]
    fn correctly_prints_string_with_a_first_line_indentation() {
        let str = ["    first  ", "  line     ", "indentation", "     string"].join("\n");

        assert_eq!(
            print_block_string(&str),
            [
                r#""""    first  "#,
                "  line     ",
                "indentation",
                r#"     string""""#
            ]
            .join("\n")
        );
    }
}
