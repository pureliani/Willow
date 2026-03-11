pub mod parse_fn_type_annotation;
pub mod parse_parenthesized_type_annotation;
pub mod parse_struct_type_annotation;

use super::{Parser, ParsingError, ParsingErrorKind};
use crate::{
    ast::{
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        Span,
    },
    globals::STRING_INTERNER,
    tokenize::{KeywordKind, NumberKind, PunctuationKind, TokenKind},
};

fn suffix_bp(token_kind: &TokenKind) -> Option<(u8, ())> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(LBracket) => (3, ()),
        _ => return None,
    };

    Some(priority)
}

fn infix_bp(token_kind: &TokenKind) -> Option<(u8, u8)> {
    use PunctuationKind::*;
    use TokenKind::*;

    let priority = match token_kind {
        Punctuation(Or) => (1, 2),
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
                    kind: TypeAnnotationKind::Bool(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::String) => {
                let start_offset = self.offset;
                self.consume_keyword(KeywordKind::String)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::String(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U8) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U8)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U8(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U16) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U16)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U16(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U32(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::U64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::U64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::U64(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I8) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I8)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I8(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I16) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I16)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I16(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I32(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::I64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::I64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::I64(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::F32) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::F32)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::F32(None),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::F64) => {
                let start_offset = self.offset;

                self.consume_keyword(KeywordKind::F64)?;
                let span = self.get_span(start_offset, self.offset - 1)?;
                TypeAnnotation {
                    kind: TypeAnnotationKind::F64(None),
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
            TokenKind::Number(num_kind) => {
                let span = token.span.clone();
                let type_kind = match num_kind {
                    NumberKind::I64(lit) => TypeAnnotationKind::I64(Some(lit)),
                    NumberKind::I32(lit) => TypeAnnotationKind::I32(Some(lit)),
                    NumberKind::I16(lit) => TypeAnnotationKind::I16(Some(lit)),
                    NumberKind::I8(lit) => TypeAnnotationKind::I8(Some(lit)),
                    NumberKind::F32(lit) => TypeAnnotationKind::F32(Some(lit)),
                    NumberKind::F64(lit) => TypeAnnotationKind::F64(Some(lit)),
                    NumberKind::U64(lit) => TypeAnnotationKind::U64(Some(lit)),
                    NumberKind::U32(lit) => TypeAnnotationKind::U32(Some(lit)),
                    NumberKind::U16(lit) => TypeAnnotationKind::U16(Some(lit)),
                    NumberKind::U8(lit) => TypeAnnotationKind::U8(Some(lit)),
                    NumberKind::ISize(lit) => TypeAnnotationKind::ISize(Some(lit)),
                    NumberKind::USize(lit) => TypeAnnotationKind::USize(Some(lit)),
                };
                self.advance();
                TypeAnnotation {
                    kind: type_kind,
                    span,
                }
            }
            TokenKind::String(ref s) => {
                let span = token.span.clone();
                let id = STRING_INTERNER.intern(s);
                self.advance();
                TypeAnnotation {
                    kind: TypeAnnotationKind::String(Some(id)),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::True) => {
                let span = token.span.clone();
                self.advance();
                TypeAnnotation {
                    kind: TypeAnnotationKind::Bool(Some(true)),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::False) => {
                let span = token.span.clone();
                self.advance();
                TypeAnnotation {
                    kind: TypeAnnotationKind::Bool(Some(false)),
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

                lhs = match op.kind {
                    TokenKind::Punctuation(PunctuationKind::LBracket) => {
                        self.consume_punctuation(PunctuationKind::LBracket)?;
                        self.consume_punctuation(PunctuationKind::RBracket)?;

                        let span =
                            self.get_span(lhs.span.start.byte_offset, self.offset - 1)?;
                        TypeAnnotation {
                            kind: TypeAnnotationKind::List(Box::new(lhs.clone())),
                            span,
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

            if let Some((left_prec, right_prec)) = infix_bp(&op.kind) {
                if left_prec < min_prec {
                    break;
                }

                let op_kind = op.kind.clone();
                self.advance();

                let rhs = self.parse_type_annotation(right_prec)?;

                let start_pos = lhs.span.start;
                let end_pos = rhs.span.end;
                let combined_span = Span {
                    start: start_pos,
                    end: end_pos,
                    path: self.path.clone(),
                };

                lhs = match op_kind {
                    TokenKind::Punctuation(PunctuationKind::Or) => {
                        let mut variants = match lhs.kind {
                            TypeAnnotationKind::Union(v) => v,
                            _ => vec![lhs],
                        };

                        match rhs.kind {
                            TypeAnnotationKind::Union(v) => variants.extend(v),
                            _ => variants.push(rhs),
                        }

                        TypeAnnotation {
                            kind: TypeAnnotationKind::Union(variants),
                            span: combined_span,
                        }
                    }

                    _ => unreachable!(
                        "Operator found in infix_bp but not handled in match: {:?}",
                        op_kind
                    ),
                };

                continue;
            }

            break;
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
                    kind: TypeAnnotationKind::I8(None),
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
                    kind: TypeAnnotationKind::I16(None),
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
                    kind: TypeAnnotationKind::I32(None),
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
                    kind: TypeAnnotationKind::I64(None),
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
                    kind: TypeAnnotationKind::F32(None),
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
                    kind: TypeAnnotationKind::F64(None),
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
                    kind: TypeAnnotationKind::U8(None),
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
                    kind: TypeAnnotationKind::U16(None),
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
                    kind: TypeAnnotationKind::U32(None),
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
                    kind: TypeAnnotationKind::U64(None),
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
                    kind: TypeAnnotationKind::Bool(None),
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
                "string",
                TypeAnnotation {
                    kind: TypeAnnotationKind::String(None),
                    span: Span {
                        start: Position {
                            line: 1,
                            col: 1,
                            byte_offset: 0,
                        },
                        end: Position {
                            line: 1,
                            col: 7,
                            byte_offset: 6,
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
