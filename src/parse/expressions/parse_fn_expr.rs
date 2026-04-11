use crate::{
    ast::{
        decl::{FnDecl, GenericParam, Param},
        expr::{Expr, ExprKind},
        type_annotation::{TypeAnnotation, TypeAnnotationKind},
        IdentifierNode,
    },
    globals::next_declaration_id,
    parse::{Parser, ParsingError},
    tokenize::{KeywordKind, PunctuationKind, TokenKind},
};

impl Parser {
    pub fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, ParsingError> {
        let mut params = Vec::new();

        if self.match_token(0, TokenKind::Punctuation(PunctuationKind::Lt)) {
            self.place_checkpoint();
            self.advance();

            let args_result = self.comma_separated(
                |p| {
                    let identifier = p.consume_identifier()?;
                    let constraint = if p
                        .match_token(0, TokenKind::Punctuation(PunctuationKind::Col))
                    {
                        p.advance();
                        Some(p.parse_type_annotation(0)?)
                    } else {
                        None
                    };
                    Ok(GenericParam {
                        identifier,
                        constraint,
                    })
                },
                |p| p.match_token(0, TokenKind::Punctuation(PunctuationKind::Gt)),
            );

            if let Ok(args) = args_result {
                if self.match_token(0, TokenKind::Punctuation(PunctuationKind::Gt)) {
                    self.advance();
                    params = args;
                } else {
                    self.goto_checkpoint();
                }
            } else {
                self.goto_checkpoint();
            }
        }

        Ok(params)
    }

    pub fn parse_fn_signature(
        &mut self,
    ) -> Result<
        (
            IdentifierNode,
            Vec<GenericParam>,
            Vec<Param>,
            TypeAnnotation,
        ),
        ParsingError,
    > {
        let start_offset = self.offset;

        self.consume_keyword(KeywordKind::Fn)?;
        let identifier = self.consume_identifier()?;
        let generic_params = self.parse_generic_params()?;

        self.consume_punctuation(PunctuationKind::LParen)?;
        let params = self.comma_separated(
            |p| {
                let param_ident = p.consume_identifier()?;
                p.consume_punctuation(PunctuationKind::Col)?;
                let constraint = p.parse_type_annotation(0)?;

                Ok(Param {
                    constraint,
                    identifier: param_ident,
                })
            },
            |p| p.match_token(0, TokenKind::Punctuation(PunctuationKind::RParen)),
        )?;
        self.consume_punctuation(PunctuationKind::RParen)?;

        let return_type =
            if self.match_token(0, TokenKind::Punctuation(PunctuationKind::Col)) {
                self.consume_punctuation(PunctuationKind::Col)?;
                self.parse_type_annotation(0)?
            } else {
                TypeAnnotation {
                    kind: TypeAnnotationKind::Void,
                    span: self.get_span(start_offset, self.offset - 1)?,
                }
            };

        Ok((identifier, generic_params, params, return_type))
    }

    pub fn parse_fn_expr(&mut self) -> Result<Expr, ParsingError> {
        let documentation = self.consume_optional_doc();
        let start_offset = self.offset;

        let is_exported = if self.match_token(0, TokenKind::Keyword(KeywordKind::Export))
        {
            self.consume_keyword(KeywordKind::Export)?;
            true
        } else {
            false
        };

        let (identifier, generic_params, params, return_type) =
            self.parse_fn_signature()?;
        let body = self.parse_codeblock_expr()?;

        let id = next_declaration_id();

        Ok(Expr {
            kind: ExprKind::Fn(Box::new(FnDecl {
                id,
                identifier,
                generic_params,
                params,
                return_type,
                body,
                documentation,
                is_exported,
            })),
            span: self.get_span(start_offset, self.offset - 1)?,
        })
    }
}
