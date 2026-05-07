pub mod parse_fn_type_annotation;
pub mod parse_parenthesized_type_annotation;
pub mod parse_struct_type_annotation;

use super::{Parser, ParsingError, ParsingErrorKind};
use crate::{
    ast::{
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        Span,
    },
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

fn suffix_bp(token_kind: &TokenKind) -> Option<(u8, ())> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(Lt) => (3, ()),
        _ => return None,
    };

    Some(priority)
}

impl Parser {
    pub fn parse_type_annotation(
        &mut self,
        min_prec: u8,
    ) -> Result<TypeAnnotation, ParsingError> {
        let token = self.current().ok_or(self.unexpected_end_of_input())?;

        let mut lhs = match token.kind {
            TokenKind::Keyword(KeywordKind::Void) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::Void)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::Void,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::Bool) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::Bool)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::Bool,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U8) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U8)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U8,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U16) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U16)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U16,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U32,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U64,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I8) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I8)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I8,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I16) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I16)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I16,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I32,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I64,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::F32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::F32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::F32,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::F64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::F64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::F64,
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::Null) => {
                let span = token.span.clone();
                self.advance();
                TypeAnnotation {
                    kind: TypeAnnotationKind::Null,
                    span,
                }
            }
            TokenKind::Punctuation(PunctuationKind::Star) => {
                let start_offset = self.offset;
                self.consume_punctuation(PunctuationKind::Star)?;

                let is_mut = if self.match_token(0, TokenKind::Keyword(KeywordKind::Mut))
                {
                    self.consume_keyword(KeywordKind::Mut)?;
                    true
                } else {
                    false
                };

                let inner = self.parse_type_annotation(14)?;
                let span = self.get_span(start_offset, self.offset - 1)?;

                TypeAnnotation {
                    kind: if is_mut {
                        TypeAnnotationKind::MutPointer(Box::new(inner))
                    } else {
                        TypeAnnotationKind::Pointer(Box::new(inner))
                    },
                    span,
                }
            }
            TokenKind::Punctuation(PunctuationKind::LParen) => {
                self.parse_parenthesized_type_annotation()?
            }
            TokenKind::Punctuation(PunctuationKind::LBrace) => {
                self.parse_struct_type_annotation()?
            }
            TokenKind::Keyword(KeywordKind::Fn) => self.parse_fn_type_annotation()?,
            TokenKind::Identifier(_) => {
                let identifier = self.consume_identifier()?;
                TypeAnnotation {
                    span: identifier.span.clone(),
                    kind: TypeAnnotationKind::Identifier(identifier),
                }
            }
            _ => {
                return Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedATypeButFound(token.clone()),
                    span: token.span.clone(),
                })
            }
        };

        while let Some(op) = self.current() {
            if let Some((left_prec, ())) = suffix_bp(&op.kind) {
                if left_prec < min_prec {
                    break;
                }

                let lhs_clone = lhs.clone();

                lhs = match op.kind {
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
                            TypeAnnotation {
                                kind: TypeAnnotationKind::GenericApply {
                                    left: Box::new(lhs_clone),
                                    args,
                                },
                                span,
                            }
                        } else {
                            self.goto_checkpoint();
                            break;
                        }
                    }
                    _ => {
                        panic!(
                            "INTERNAL COMPILER ERROR: Unexpected suffix type-annotation \
                             operator"
                        )
                    }
                };

                continue;
            }

            break;
        }

        if self.match_token(0, TokenKind::Keyword(KeywordKind::Where)) {
            self.advance(); // consume `where`
            let condition = self.parse_expr(0)?;

            let span = Span {
                start: lhs.span.start,
                end: condition.span.end,
                path: self.path.clone(),
            };

            lhs = TypeAnnotation {
                kind: TypeAnnotationKind::Refinement {
                    base: Box::new(lhs),
                    condition: Box::new(condition),
                },
                span,
            };
        }

        Ok(lhs)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            type_annotation::{TypeAnnotation, TypeAnnotationKind},
            ModulePath, Span,
        },
        globals::reset_globals,
        parse::Parser,
    };

    #[test]
    fn parses_primitive_types() {
        use crate::ast::Position;
        use crate::tokenize::Tokenizer;
        use pretty_assertions::assert_eq;

        reset_globals();

        let path = ModulePath::default();

        let test_cases = vec![
            (
                "i8",
                TypeAnnotation {
                    kind: TypeAnnotationKind::I8,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 3,
                            byte_offset: 2,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "i16",
                TypeAnnotation {
                    kind: TypeAnnotationKind::I16,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "i32",
                TypeAnnotation {
                    kind: TypeAnnotationKind::I32,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "i64",
                TypeAnnotation {
                    kind: TypeAnnotationKind::I64,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "f32",
                TypeAnnotation {
                    kind: TypeAnnotationKind::F32,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "f64",
                TypeAnnotation {
                    kind: TypeAnnotationKind::F64,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "u8",
                TypeAnnotation {
                    kind: TypeAnnotationKind::U8,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 3,
                            byte_offset: 2,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "u16",
                TypeAnnotation {
                    kind: TypeAnnotationKind::U16,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "u32",
                TypeAnnotation {
                    kind: TypeAnnotationKind::U32,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "u64",
                TypeAnnotation {
                    kind: TypeAnnotationKind::U64,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 4,
                            byte_offset: 3,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "void",
                TypeAnnotation {
                    kind: TypeAnnotationKind::Void,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 5,
                            byte_offset: 4,
                        },
                        path: path.clone(),
                    },
                },
            ),
            (
                "bool",
                TypeAnnotation {
                    kind: TypeAnnotationKind::Bool,
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 5,
                            byte_offset: 4,
                        },
                        path: path.clone(),
                    },
                },
            ),
        ];

        for (input, expected) in test_cases {
            let (tokens, _) = Tokenizer::tokenize(input, path.clone());
            let mut parser = Parser {
                offset: 0,
                checkpoint_offset: 0,
                tokens,
                path: path.clone(),
            };
            let result = parser.parse_type_annotation(0);

            assert_eq!(result, Ok(expected))
        }
    }
}
