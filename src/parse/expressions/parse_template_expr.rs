use crate::{
    ast::{
        expr::{Expr, ExprKind},
        StringNode,
    },
    parse::{unescape_string, Parser, ParsingError, ParsingErrorKind},
    tokenize::{PunctuationKind, TokenKind},
};
use unicode_segmentation::UnicodeSegmentation;

impl Parser {
    pub fn parse_template_expr(&mut self) -> Result<Expr, ParsingError> {
        let start_offset = self.offset;
        self.consume_punctuation(PunctuationKind::Backtick)?;

        let mut parts = Vec::new();

        loop {
            if self.match_token(0, TokenKind::Punctuation(PunctuationKind::Backtick)) {
                self.consume_punctuation(PunctuationKind::Backtick)?;
                break;
            }

            let current = self
                .current()
                .ok_or_else(|| self.unexpected_end_of_input())?;

            match &current.kind {
                TokenKind::TemplateString(text) => {
                    let span = current.span.clone();

                    let unescaped = unescape_string(text);
                    let len = unescaped.graphemes(true).count();

                    parts.push(Expr {
                        kind: ExprKind::String(StringNode {
                            value: unescaped,
                            len,
                            span: span.clone(),
                        }),
                        span,
                    });
                    self.advance();
                }
                TokenKind::Punctuation(PunctuationKind::DollarLBrace) => {
                    self.consume_punctuation(PunctuationKind::DollarLBrace)?;
                    let expr = self.parse_expr(0)?;
                    parts.push(expr);
                    self.consume_punctuation(PunctuationKind::RBrace)?;
                }
                _ => {
                    return Err(ParsingError {
                        kind: ParsingErrorKind::ExpectedAnExpressionButFound(
                            current.clone(),
                        ),
                        span: current.span.clone(),
                    });
                }
            }
        }

        let span = self.get_span(start_offset, self.offset - 1)?;

        Ok(Expr {
            kind: ExprKind::TemplateString(parts),
            span,
        })
    }
}
