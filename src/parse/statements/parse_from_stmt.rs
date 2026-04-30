use crate::{
    ast::{
        decl::ExternFnDecl,
        stmt::{ImportItem, Stmt, StmtKind},
    },
    globals::next_declaration_id,
    parse::{Parser, ParsingError, ParsingErrorKind},
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

impl Parser {
    pub fn parse_from_stmt(&mut self) -> Result<Stmt, ParsingError> {
        let start_offset = self.offset;

        self.consume_keyword(KeywordKind::From)?;
        let path = self.consume_string()?;

        self.consume_punctuation(PunctuationKind::LBrace)?;
        let items = self.comma_separated(
            |p| {
                if p.match_token(0, TokenKind::Keyword(KeywordKind::Fn)) {
                    let (identifier, generic_params, params, return_type) =
                        p.parse_fn_signature()?;

                    if !generic_params.is_empty() {
                        return Err(ParsingError {
                            kind: ParsingErrorKind::ExternFnCannotBeGeneric,
                            span: identifier.span.clone(),
                        });
                    }

                    let id = next_declaration_id();

                    Ok(ImportItem::ExternFn(ExternFnDecl {
                        id,
                        documentation: None,
                        identifier,
                        params,
                        return_type,
                    }))
                } else {
                    let identifier = p.consume_identifier()?;
                    let alias = if p
                        .match_token(0, TokenKind::Punctuation(PunctuationKind::Col))
                    {
                        p.advance();
                        Some(p.consume_identifier()?)
                    } else {
                        None
                    };

                    Ok(ImportItem::Symbol { identifier, alias })
                }
            },
            |p| p.match_token(0, TokenKind::Punctuation(PunctuationKind::RBrace)),
        )?;
        self.consume_punctuation(PunctuationKind::RBrace)?;

        let span = self.get_span(start_offset, self.offset - 1)?;

        Ok(Stmt {
            kind: StmtKind::From { path, items },
            span,
        })
    }
}
