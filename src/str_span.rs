use swc_core::common::{BytePos, Span};
use swc_core::ecma::ast::{Str, TplElement};

pub trait StrSpan {
    /// returns [`Span`] of the string including quotes
    fn outer_span(&self) -> Span;
    /// returns [`Span`] of the string excluding quotes
    fn value_span(&self) -> Span;
}

impl StrSpan for Str {
    fn outer_span(&self) -> Span {
        self.span
    }

    fn value_span(&self) -> Span {
        const QUOTE_LEN: BytePos = BytePos(1);

        Span::new(self.span.lo + QUOTE_LEN, self.span.hi - QUOTE_LEN)
    }
}

impl StrSpan for TplElement {
    fn outer_span(&self) -> Span {
        const QUOTE_LEN: BytePos = BytePos(1);

        Span::new(self.span.lo - QUOTE_LEN, self.span.hi + QUOTE_LEN)
    }

    fn value_span(&self) -> Span {
        self.span
    }
}
