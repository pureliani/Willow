macro_rules! matches_token {
    ($parser:expr, $index:expr, $pattern:pat $(if $guard:expr)?) => {
        $parser.tokens.get($parser.offset + $index).map_or(false, |token| {
            matches!(token.kind, $pattern $(if $guard)?)
        })
    };
}

mod expressions;
mod statements;
mod type_annotations;

pub struct Parser {
    pub offset: usize,
    pub tokens: Vec<Token>,
    pub checkpoint_offset: usize,
    pub path: ModulePath,
}

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    ast::{
        stmt::Stmt, type_annotation::TypeAnnotation, IdentifierNode, ModulePath,
        Position, Span, StringNode,
    },
    tokenize::{KeywordKind, NumberKind, PunctuationKind, Token, TokenKind},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ParsingErrorKind {
    ExternFnCannotBeGeneric,
    ExpectedATagTypeButFound(TypeAnnotation),
    DocMustBeFollowedByDeclaration,
    ExpectedAnExpressionButFound(Token),
    ExpectedATypeButFound(Token),
    InvalidSuffixOperator(Token),
    UnexpectedEndOfInput,
    ExpectedAnIdentifier,
    ExpectedAPunctuationMark(PunctuationKind),
    ExpectedAKeyword(KeywordKind),
    ExpectedAStringValue,
    ExpectedANumericValue,
    UnknownStaticMethod(IdentifierNode),
    UnexpectedStatementAfterFinalExpression,
    ExpectedStatementOrExpression { found: Token },
    UnexpectedTokenAfterFinalExpression { found: Token },
    ExpectedToBeFollowedByOneOfTheTokens(Vec<Token>),
}

impl ParsingErrorKind {
    pub fn code(&self) -> usize {
        match self {
            ParsingErrorKind::DocMustBeFollowedByDeclaration => 1,
            ParsingErrorKind::ExpectedAnExpressionButFound(..) => 2,
            ParsingErrorKind::ExpectedATypeButFound(..) => 3,
            ParsingErrorKind::InvalidSuffixOperator(..) => 4,
            ParsingErrorKind::UnexpectedEndOfInput => 5,
            ParsingErrorKind::ExpectedAnIdentifier => 6,
            ParsingErrorKind::ExpectedAPunctuationMark(..) => 7,
            ParsingErrorKind::ExpectedAKeyword(..) => 8,
            ParsingErrorKind::ExpectedAStringValue => 9,
            ParsingErrorKind::ExpectedANumericValue => 10,
            ParsingErrorKind::UnknownStaticMethod(..) => 11,
            ParsingErrorKind::UnexpectedStatementAfterFinalExpression => 12,
            ParsingErrorKind::ExpectedStatementOrExpression { .. } => 13,
            ParsingErrorKind::UnexpectedTokenAfterFinalExpression { .. } => 14,
            ParsingErrorKind::ExpectedATagTypeButFound(..) => 15,
            ParsingErrorKind::ExpectedToBeFollowedByOneOfTheTokens(..) => 16,
            ParsingErrorKind::ExternFnCannotBeGeneric => 17,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsingError {
    pub kind: ParsingErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocAnnotation {
    message: String,
    span: Span,
}

fn unescape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('0') => out.push('\0'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

impl Parser {
    fn match_token(&self, index: usize, kind: TokenKind) -> bool {
        if let Some(token) = self.tokens.get(self.offset + index) {
            return token.kind == kind;
        }

        false
    }

    fn advance(&mut self) {
        self.offset += 1;
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.offset)
    }

    fn unexpected_end_of_input(&self) -> ParsingError {
        // TODO: fix this
        let first_token_span = Span {
            start: Position {
                line: 1,
                col: 1,
                byte_offset: 0,
            },
            end: Position {
                line: 1,
                col: 1,
                byte_offset: 0,
            },
            path: self.path.clone(),
        };

        let last_token_span = self
            .tokens
            .last()
            .map(|t| &t.span)
            .unwrap_or(&first_token_span)
            .clone();

        ParsingError {
            kind: ParsingErrorKind::UnexpectedEndOfInput,
            span: last_token_span,
        }
    }

    fn get_span(
        &self,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<Span, ParsingError> {
        let start = self
            .tokens
            .get(start_offset)
            .ok_or(self.unexpected_end_of_input())?;

        let end = self
            .tokens
            .get(end_offset)
            .ok_or(self.unexpected_end_of_input())?;

        Ok(Span {
            start: start.span.start,
            end: end.span.end,
            path: self.path.clone(),
        })
    }

    fn place_checkpoint(&mut self) {
        self.checkpoint_offset = self.offset;
    }

    fn goto_checkpoint(&mut self) {
        self.offset = self.checkpoint_offset;
    }

    pub fn consume_string(&mut self) -> Result<StringNode, ParsingError> {
        if let Some(t) = self.current() {
            let span = t.span.clone();
            match &t.kind {
                TokenKind::String(value) => {
                    let owned_value = unescape_string(value);
                    let len = owned_value.graphemes(true).count();
                    self.advance();

                    Ok(StringNode {
                        span,
                        len,
                        value: owned_value,
                    })
                }
                _ => Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedAStringValue,
                    span,
                }),
            }
        } else {
            Err(self.unexpected_end_of_input())
        }
    }

    pub fn consume_punctuation(
        &mut self,
        expected: PunctuationKind,
    ) -> Result<(), ParsingError> {
        if let Some(token) = self.current() {
            match &token.kind {
                TokenKind::Punctuation(pk) if *pk == expected => {
                    self.advance();
                    Ok(())
                }
                _ => Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedAPunctuationMark(expected),
                    span: token.span.clone(),
                }),
            }
        } else {
            Err(self.unexpected_end_of_input())
        }
    }

    pub fn consume_number(&mut self) -> Result<NumberKind, ParsingError> {
        if let Some(token) = self.current() {
            match token.kind {
                TokenKind::Number(number_kind) => {
                    self.advance();
                    return Ok(number_kind);
                }
                _ => {
                    return Err(ParsingError {
                        kind: ParsingErrorKind::ExpectedANumericValue,
                        span: token.span.clone(),
                    })
                }
            }
        }

        Err(self.unexpected_end_of_input())
    }

    pub fn consume_keyword(
        &mut self,
        expected: KeywordKind,
    ) -> Result<Span, ParsingError> {
        if let Some(token) = self.current() {
            let span = token.span.clone();
            match token.kind {
                TokenKind::Keyword(keyword_kind) if keyword_kind == expected => {
                    self.advance();
                    Ok(span)
                }
                _ => Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedAKeyword(expected),
                    span,
                }),
            }
        } else {
            Err(self.unexpected_end_of_input())
        }
    }

    pub fn consume_identifier(&mut self) -> Result<IdentifierNode, ParsingError> {
        if let Some(token) = self.current() {
            match token.kind {
                TokenKind::Identifier(name) => {
                    let span = token.span.clone();
                    self.advance();
                    Ok(IdentifierNode { name, span })
                }
                _ => Err(ParsingError {
                    kind: ParsingErrorKind::ExpectedAnIdentifier,
                    span: token.span.clone(),
                }),
            }
        } else {
            Err(self.unexpected_end_of_input())
        }
    }

    pub fn consume_optional_doc(&mut self) -> Option<DocAnnotation> {
        let result = if let Some(Token {
            kind: TokenKind::Doc(doc),
            span,
        }) = self.current()
        {
            Some(DocAnnotation {
                span: span.clone(),
                message: doc.clone(),
            })
        } else {
            None
        };

        if result.is_some() {
            self.advance();
        };

        result
    }

    pub fn comma_separated<F, T, E>(
        &mut self,
        mut parser: F,
        is_end: E,
    ) -> Result<Vec<T>, ParsingError>
    where
        F: FnMut(&mut Self) -> Result<T, ParsingError>,
        E: Fn(&Self) -> bool,
    {
        let mut items = Vec::new();

        if is_end(self) {
            return Ok(items);
        }

        let first_item = parser(self)?;
        items.push(first_item);

        loop {
            if is_end(self) {
                break;
            }

            self.consume_punctuation(PunctuationKind::Comma)?;

            if is_end(self) {
                break;
            }

            let item = parser(self)?;
            items.push(item);
        }

        Ok(items)
    }

    pub fn parse(tokens: Vec<Token>, path: ModulePath) -> (Vec<Stmt>, Vec<ParsingError>) {
        let mut state = Parser {
            offset: 0,
            checkpoint_offset: 0,
            tokens,
            path,
        };

        let mut statements: Vec<Stmt> = vec![];
        let mut errors: Vec<ParsingError> = vec![];

        while state.current().is_some() {
            let stmt = state.parse_stmt();
            match stmt {
                Ok(s) => {
                    statements.push(s);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        (statements, errors)
    }
}
