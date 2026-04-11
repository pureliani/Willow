use crate::{
    ast::{
        decl::TypeAliasDecl,
        stmt::{Stmt, StmtKind},
    },
    globals::next_declaration_id,
    parse::{Parser, ParsingError},
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

impl Parser {
    pub fn parse_type_alias_decl(&mut self) -> Result<Stmt, ParsingError> {
        let documentation = self.consume_optional_doc();
        let start_offset = self.offset;

        let is_exported = if self.match_token(0, TokenKind::Keyword(KeywordKind::Export))
        {
            self.consume_keyword(KeywordKind::Export)?;
            true
        } else {
            false
        };

        self.consume_keyword(KeywordKind::Type)?;
        let name = self.consume_identifier()?;
        let generic_params = self.parse_generic_params()?;

        self.consume_punctuation(PunctuationKind::Eq)?;
        let ty = self.parse_type_annotation(0)?;
        self.consume_punctuation(PunctuationKind::SemiCol)?;

        let span = self.get_span(start_offset, self.offset - 1)?;
        let id = next_declaration_id();

        Ok(Stmt {
            kind: StmtKind::TypeAliasDecl(TypeAliasDecl {
                id,
                identifier: name,
                generic_params,
                documentation,
                value: ty,
                is_exported,
            }),
            span,
        })
    }
}
