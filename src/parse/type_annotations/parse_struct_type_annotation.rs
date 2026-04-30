use crate::{
    ast::{
        decl::Param,
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
    },
    globals::next_declaration_id,
    parse::{Parser, ParsingError},
    tokenize::{PunctuationKind, TokenKind},
};

impl Parser {
    pub fn parse_struct_type_annotation(
        &mut self,
    ) -> Result<TypeAnnotation, ParsingError> {
        let start_offset = self.offset;
        self.consume_punctuation(PunctuationKind::LBrace)?;
        let fields = self.comma_separated(
            |p| {
                let identifier = p.consume_identifier()?;
                p.consume_punctuation(PunctuationKind::Col)?;
                let constraint = p.parse_type_annotation(0)?;

                Ok(Param {
                    id: next_declaration_id(),
                    identifier,
                    constraint,
                })
            },
            |p| p.match_token(0, TokenKind::Punctuation(PunctuationKind::RBrace)),
        )?;
        self.consume_punctuation(PunctuationKind::RBrace)?;

        let span = self.get_span(start_offset, self.offset - 1)?;

        Ok(TypeAnnotation {
            kind: TypeAnnotationKind::Struct(fields),
            span,
        })
    }
}
