use textwrap::dedent;

pub(crate) const TRIPLE_QUOTES: &str = r#"""""#;

pub(crate) fn cmp_description(left: &str, right: &str) -> bool {
    macro_rules! clean {
        ($ident:ident) => {
            let $ident = if $ident.starts_with(TRIPLE_QUOTES) {
                &$ident[3..$ident.len() - 3]
            } else if $ident.starts_with('"') {
                &$ident[1..$ident.len() - 1]
            } else {
                $ident
            };

            let $ident = dedent($ident);
            let $ident = $ident.trim_matches([' ', '\r', '\n']);
        };
    }

    clean!(left);
    clean!(right);

    left == right
}
