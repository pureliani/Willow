use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
    },
    globals::next_declaration_id,
    parse::ParsingError,
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

use super::Parser;

impl Parser {
    pub fn parse_fn_type_annotation(&mut self) -> Result<TypeAnnotation, ParsingError> {
        let start_offset = self.offset;

        self.consume_keyword(KeywordKind::Fn)?;
        self.consume_punctuation(PunctuationKind::LParen)?;
        let params = self.comma_separated(
            |p| {
                let identifier = p.consume_identifier()?;
                p.consume_punctuation(PunctuationKind::Col)?;
                let constraint = p.parse_type_annotation(0)?;

                Ok(Param {
                    id: next_declaration_id(),
                    constraint,
                    identifier,
                })
            },
            |p| p.match_token(0, TokenKind::Punctuation(PunctuationKind::RParen)),
        )?;
        self.consume_punctuation(PunctuationKind::RParen)?;
        self.consume_punctuation(PunctuationKind::Col)?;
        let return_type = self.parse_type_annotation(0)?;

        let span = self.get_span(start_offset, self.offset - 1)?;

        Ok(TypeAnnotation {
            kind: TypeAnnotationKind::FnType {
                params,
                return_type: Box::new(return_type),
            },
            span,
        })
    }
}
