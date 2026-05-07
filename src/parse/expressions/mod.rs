pub mod parse_codeblock_expr;
pub mod parse_fn_call_expr;
pub mod parse_fn_expr;
pub mod parse_if_expr;
pub mod parse_parenthesized_expr;
pub mod parse_struct_init_expr;
pub mod parse_template_expr;

use crate::{
    ast::{
        expr::{Expr, ExprKind},
        Span,
    },
    globals::STRING_INTERNER,
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

use super::{Parser, ParsingError, ParsingErrorKind};

fn prefix_bp(token_kind: &TokenKind) -> Option<((), u8)> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(Minus) | Punctuation(Not) | Punctuation(And) => ((), 13),
        _ => return None,
    };

    Some(priority)
}

fn infix_bp(token_kind: &TokenKind) -> Option<(u8, u8)> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(DoubleOr) => (1, 2),
        Punctuation(DoubleAnd) => (3, 4),
        Punctuation(DoubleEq) | Punctuation(NotEq) => (5, 6),
        Punctuation(Lt) | Punctuation(Lte) | Punctuation(Gt) | Punctuation(Gte) => (7, 8),
        Punctuation(Plus) | Punctuation(Minus) => (9, 10),
        Punctuation(Star) | Punctuation(Slash) | Punctuation(Percent) => (11, 12),
        _ => return None,
    };

    Some(priority)
}

fn suffix_bp(token_kind: &TokenKind) -> Option<(u8, ())> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(LParen) => (14, ()),    // fn call
        Punctuation(Dot) => (14, ()),       // member access
        Punctuation(DoubleCol) => (14, ()), // static member accesses
        Punctuation(Lt) => (14, ()),        // generic instantiation
        _ => return None,
    };

    Some(priority)
}

pub fn is_start_of_expr(token_kind: &TokenKind) -> bool {
    match token_kind {
        TokenKind::Identifier(_)
        | TokenKind::Number(_)
        | TokenKind::String(_)
        | TokenKind::Keyword(KeywordKind::SelfValue) 
        | TokenKind::Keyword(KeywordKind::Fn)
        | TokenKind::Keyword(KeywordKind::True)
        | TokenKind::Keyword(KeywordKind::False)
        | TokenKind::Keyword(KeywordKind::If)
        | TokenKind::Keyword(KeywordKind::Null)
        | TokenKind::Punctuation(PunctuationKind::And)      // Address of
        | TokenKind::Punctuation(PunctuationKind::LParen)   // Parenthesized expr
        | TokenKind::Punctuation(PunctuationKind::LBrace)   // Codeblock or Struct expr
        | TokenKind::Punctuation(PunctuationKind::Minus)    // Negation
        | TokenKind::Punctuation(PunctuationKind::Not)      // Logical NOT
        | TokenKind::Punctuation(PunctuationKind::Backtick) // Template
          => true,
        _ => false,
    }
}

impl Parser {
    pub fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, ParsingError> {
        let token = self.current().ok_or(self.unexpected_end_of_input())?;

        let token_span = token.span.clone();

        let mut lhs = match token.kind {
             TokenKind::Punctuation(PunctuationKind::And) => {
                let ((), r_bp) = prefix_bp(&TokenKind::Punctuation(PunctuationKind::And))
                    .expect("INTERNAL COMPILER ERROR: expected '&' to have prefix binding power");
                
                let start_offset = self.offset;
                self.consume_punctuation(PunctuationKind::And)?;
                
                let is_mut = if self.match_token(0, TokenKind::Keyword(KeywordKind::Mut)) {
                    self.consume_keyword(KeywordKind::Mut)?;
                    true
                } else {
                    false
                };

                let expr = self.parse_expr(r_bp)?;
                
                Expr {
                    kind: ExprKind::AddressOf {
                        is_mut,
                        right: Box::new(expr),
                    },
                    span: self.get_span(start_offset, self.offset - 1)?,
                }
            }
            TokenKind::Keyword(KeywordKind::SelfValue) => {
                let span = self.consume_keyword(KeywordKind::SelfValue)?;
                Expr {
                    kind: ExprKind::SelfValue,
                    span,
                }
            }
            TokenKind::Identifier(_) => {
                let identifier = self.consume_identifier()?;
                Expr {
                    kind: ExprKind::Identifier(identifier),
                    span: token_span,
                }
            }
            TokenKind::Number(_) => {
                let number = self.consume_number()?;
                Expr {
                    kind: ExprKind::Number(number),
                    span: token_span,
                }
            }
            TokenKind::Keyword(KeywordKind::Fn) => self.parse_fn_expr()?,
            TokenKind::Punctuation(PunctuationKind::LParen) => {
                let start_offset = self.offset;
                let result = self.parse_parenthesized_expr()?;
                let span = self.get_span(start_offset, self.offset - 1)?;

                Expr {
                    kind: result.kind,
                    span,
                }
            }
            TokenKind::Punctuation(PunctuationKind::LBrace) => {
                self.place_checkpoint();

                self.parse_struct_init_expr()
                    .or_else(|struct_parsing_error| {
                        let struct_parsing_error_offset = self.offset;
                        self.goto_checkpoint();
                        self.parse_codeblock_expr()
                            .map(|codeblock| Expr {
                                span: codeblock.span.clone(),
                                kind: ExprKind::CodeBlock(codeblock),
                            })
                            .map_err(|codeblock_parsing_error| {
                                let codeblock_parsing_error_offset = self.offset;
                                if codeblock_parsing_error_offset
                                    > struct_parsing_error_offset
                                {
                                    codeblock_parsing_error
                                } else {
                                    struct_parsing_error
                                }
                            })
                    })?
            }
            TokenKind::Punctuation(PunctuationKind::Minus) => {
                let ((), r_bp) =
                    prefix_bp(&TokenKind::Punctuation(PunctuationKind::Minus)).expect(
                        "INTERNAL COMPILER ERROR: expected the minus \'-\' symbol to \
                         have a corresponding prefix binding power",
                    );
                let start_offset = self.offset;

                self.consume_punctuation(PunctuationKind::Minus)?;
                let expr = self.parse_expr(r_bp)?;
                Expr {
                    kind: ExprKind::Neg {
                        right: Box::new(expr),
                    },
                    span: self.get_span(start_offset, self.offset - 1)?,
                }
            }
            TokenKind::Punctuation(PunctuationKind::Not) => {
                let ((), r_bp) = prefix_bp(&TokenKind::Punctuation(PunctuationKind::Not))
                    .expect(
                        "INTERNAL COMPILER ERROR: expected the not \'!\' symbol to have \
                         a corresponding prefix binding power",
                    );
                let start_offset = self.offset;

                self.consume_punctuation(PunctuationKind::Not)?;
                let expr = self.parse_expr(r_bp)?;
                Expr {
                    kind: ExprKind::Not {
                        right: Box::new(expr),
                    },
                    span: self.get_span(start_offset, self.offset - 1)?,
                }
            }
            TokenKind::Keyword(KeywordKind::If) => self.parse_if_expr()?,
            TokenKind::Keyword(KeywordKind::Null) => {
                let span = self.consume_keyword(KeywordKind::Null)?;
                Expr {
                    kind: ExprKind::Null,
                    span,
                }
            }
            TokenKind::Keyword(
                variant @ KeywordKind::True | variant @ KeywordKind::False,
            ) => {
                let start_offset = self.offset;
                self.consume_keyword(variant)?;
                let is_true = matches!(variant, KeywordKind::True);
                Expr {
                    kind: ExprKind::BoolLiteral(is_true),
                    span: self.get_span(start_offset, self.offset - 1)?,
                }
            }
            TokenKind::String(_) => {
                let value = self.consume_string()?;
                Expr {
                    span: value.span.clone(),
                    kind: ExprKind::String(value),
                }
            }
            TokenKind::Punctuation(PunctuationKind::Backtick) => {
                self.parse_template_expr()?
            }
            _ => {
                return Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedAnExpressionButFound(token.clone()),
                    span: token_span,
                })
            }
        };

        while let Some(op) = self.current().cloned() {
            if let Some((left_prec, ())) = suffix_bp(&op.kind) {
                if left_prec < min_prec {
                    break;
                }
                let lhs_clone = lhs.clone();

                let new_lhs = match op.kind {
                    TokenKind::Punctuation(PunctuationKind::Lt) => {
                        self.place_checkpoint();
                        self.advance();
                        let args_result = self.comma_separated(
                            |p| p.parse_type_annotation(0),
                            |p| {
                                p.match_token(
                                    0,
                                    TokenKind::Punctuation(PunctuationKind::Gt),
                                )
                            },
                        );

                        let mut success = false;
                        let mut args = Vec::new();
                        if let Ok(parsed_args) = args_result {
                            if self.match_token(
                                0,
                                TokenKind::Punctuation(PunctuationKind::Gt),
                            ) {
                                self.advance();
                                success = true;
                                args = parsed_args;
                            }
                        }

                        if success {
                            let end_pos = self.tokens[self.offset - 1].span.end;
                            let span = Span {
                                start: lhs_clone.span.start,
                                end: end_pos,
                                path: self.path.clone(),
                            };
                            Some(Expr {
                                kind: ExprKind::GenericApply {
                                    left: Box::new(lhs_clone),
                                    type_args: args,
                                },
                                span,
                            })
                        } else {
                            self.goto_checkpoint();
                            break;
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::Dot) => {
                        self.consume_punctuation(PunctuationKind::Dot)?;

                        let start_pos = lhs_clone.span.start;
                        let field = self.consume_identifier()?;
                        let end_pos = field.span.end;
                        Some(Expr {
                            kind: ExprKind::Access {
                                left: Box::new(lhs_clone),
                                field,
                            },
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: self.path.clone(),
                            },
                        })
                    }
                    TokenKind::Punctuation(PunctuationKind::DoubleCol) => {
                        let start_offset = self.offset;
                        self.consume_punctuation(PunctuationKind::DoubleCol)?;
                        let field = self.consume_identifier()?;
                        let field_name = STRING_INTERNER.resolve(field.name);

                        let new_lhs = if field_name == "as" {
                            // lhs::as(Type)
                            self.consume_punctuation(PunctuationKind::LParen)?;
                            let target_type = self.parse_type_annotation(0)?;
                            self.consume_punctuation(PunctuationKind::RParen)?;

                            Expr {
                                kind: ExprKind::TypeCast {
                                    left: Box::new(lhs.clone()),
                                    target: target_type,
                                },
                                span: self.get_span(start_offset, self.offset - 1)?,
                            }
                        } else {
                            Expr {
                                kind: ExprKind::StaticAccess {
                                    left: Box::new(lhs.clone()),
                                    field,
                                },
                                span: self.get_span(start_offset, self.offset - 1)?,
                            }
                        };

                        Some(new_lhs)
                    }
                    TokenKind::Punctuation(PunctuationKind::LParen) => {
                        Some(self.parse_fn_call_expr(lhs.clone())?)
                    }
                    _ => {
                        return Err(ParsingError {
                            span: op.span.clone(),
                            kind: ParsingErrorKind::InvalidSuffixOperator(op.clone()),
                        })
                    }
                };

                if let Some(expr) = new_lhs {
                    lhs = expr;
                    continue;
                }
            }

            if let Some((left_prec, right_prec)) = infix_bp(&op.kind) {
                if left_prec < min_prec {
                    break;
                }

                let start_pos = lhs.span.start;

                self.advance();

                let rhs = self.parse_expr(right_prec)?;

                let end_pos = rhs.span.end;

                let expr_kind = match op.kind {
                    TokenKind::Punctuation(PunctuationKind::Plus) => ExprKind::Add {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    TokenKind::Punctuation(PunctuationKind::Minus) => {
                        ExprKind::Subtract {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::Star) => ExprKind::Multiply {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    TokenKind::Punctuation(PunctuationKind::Slash) => ExprKind::Divide {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    TokenKind::Punctuation(PunctuationKind::Percent) => {
                        ExprKind::Modulo {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::Lt) => ExprKind::LessThan {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    TokenKind::Punctuation(PunctuationKind::Lte) => {
                        ExprKind::LessThanOrEqual {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::Gt) => {
                        ExprKind::GreaterThan {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::Gte) => {
                        ExprKind::GreaterThanOrEqual {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::DoubleEq) => {
                        ExprKind::Equal {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::NotEq) => {
                        ExprKind::NotEqual {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        }
                    }
                    TokenKind::Punctuation(PunctuationKind::DoubleAnd) => ExprKind::And {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    TokenKind::Punctuation(PunctuationKind::DoubleOr) => ExprKind::Or {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    },
                    _ => break,
                };

                lhs = Expr {
                    kind: expr_kind,
                    span: Span {
                        start: start_pos,
                        end: end_pos,
                        path: self.path.clone(),
                    },
                };

                continue;
            }

            break;
        }

        Ok(lhs)
    }
}
