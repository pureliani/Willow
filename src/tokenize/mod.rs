use unicode_segmentation::UnicodeSegmentation;

pub mod tokenize_documentation;
pub mod tokenize_identifier;
pub mod tokenize_number;
pub mod tokenize_punctuation;
pub mod tokenize_string;

use crate::{
    ast::{ModulePath, Position, Span},
    compile::interner::StringId,
    globals::STRING_INTERNER,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenizationErrorKind {
    UnknownToken(String),
    UnknownEscapeSequence,
    InvalidFloatingNumber,
    InvalidIntegerNumber,
    UnterminatedString,
    UnterminatedDoc,
}

impl TokenizationErrorKind {
    pub fn code(&self) -> usize {
        match self {
            TokenizationErrorKind::UnknownToken { .. } => 1,
            TokenizationErrorKind::UnknownEscapeSequence => 2,
            TokenizationErrorKind::InvalidFloatingNumber => 3,
            TokenizationErrorKind::InvalidIntegerNumber => 4,
            TokenizationErrorKind::UnterminatedString => 5,
            TokenizationErrorKind::UnterminatedDoc => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenizationError {
    pub kind: TokenizationErrorKind,
    pub span: Span,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PunctuationKind {
    DoubleCol,
    DoubleOr,
    DoubleAnd,
    DoubleEq,
    Col,
    SemiCol,
    Lt,
    Gt,
    Lte,
    Gte,
    Or,
    And,
    Not,
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eq,
    NotEq,
    Plus,
    Minus,
    Slash,
    Star,
    Percent,
    Comma,
    Dollar,
    Question,
    Hash,
}

impl PunctuationKind {
    pub fn to_string(&self) -> String {
        String::from(match self {
            PunctuationKind::DoubleCol => "::",
            PunctuationKind::DoubleOr => "||",
            PunctuationKind::DoubleAnd => "&&",
            PunctuationKind::DoubleEq => "==",
            PunctuationKind::Col => ":",
            PunctuationKind::SemiCol => ";",
            PunctuationKind::Lt => "<",
            PunctuationKind::Gt => ">",
            PunctuationKind::Lte => "<=",
            PunctuationKind::Gte => ">=",
            PunctuationKind::Or => "|",
            PunctuationKind::And => "&",
            PunctuationKind::Not => "!",
            PunctuationKind::Dot => ".",
            PunctuationKind::LParen => "(",
            PunctuationKind::RParen => ")",
            PunctuationKind::LBracket => "[",
            PunctuationKind::RBracket => "]",
            PunctuationKind::LBrace => "{",
            PunctuationKind::RBrace => "}",
            PunctuationKind::Eq => "=",
            PunctuationKind::NotEq => "!=",
            PunctuationKind::Plus => "+",
            PunctuationKind::Minus => "-",
            PunctuationKind::Slash => "/",
            PunctuationKind::Star => "*",
            PunctuationKind::Percent => "%",
            PunctuationKind::Comma => ",",
            PunctuationKind::Dollar => "$",
            PunctuationKind::Question => "?",
            PunctuationKind::Hash => "#",
        })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum KeywordKind {
    Let,
    Return,
    If,
    Else,
    While,
    Break,
    Continue,
    Type,
    From,
    Void,
    True,
    False,
    Export,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Fn,
    String,
    Null,
    Extern,
    Unsafe,
}

impl KeywordKind {
    pub fn to_string(&self) -> String {
        String::from(match self {
            KeywordKind::Let => "let",
            KeywordKind::Return => "return",
            KeywordKind::If => "if",
            KeywordKind::Else => "else",
            KeywordKind::While => "while",
            KeywordKind::Break => "break",
            KeywordKind::Continue => "continue",
            KeywordKind::Type => "type",
            KeywordKind::From => "from",
            KeywordKind::Void => "void",
            KeywordKind::True => "true",
            KeywordKind::False => "false",
            KeywordKind::Export => "export",
            KeywordKind::Bool => "bool",
            KeywordKind::I8 => "i8",
            KeywordKind::I16 => "i16",
            KeywordKind::I32 => "i32",
            KeywordKind::I64 => "i64",
            KeywordKind::U8 => "u8",
            KeywordKind::U16 => "u16",
            KeywordKind::U32 => "u32",
            KeywordKind::U64 => "u64",
            KeywordKind::F32 => "f32",
            KeywordKind::F64 => "f64",
            KeywordKind::Fn => "fn",
            KeywordKind::String => "string",
            KeywordKind::Null => "null",
            KeywordKind::Extern => "extern",
            KeywordKind::Unsafe => "unsafe",
        })
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum NumberKind {
    I64(i64),
    I32(i32),
    I16(i16),
    I8(i8),
    F32(f32),
    F64(f64),
    U64(u64),
    U32(u32),
    U16(u16),
    U8(u8),
    ISize(isize),
    USize(usize),
}

pub fn number_kind_to_suffix(kind: &NumberKind) -> String {
    match kind {
        NumberKind::I64(_) => "i64".to_owned(),
        NumberKind::I32(_) => "i32".to_owned(),
        NumberKind::I16(_) => "i16".to_owned(),
        NumberKind::I8(_) => "i8".to_owned(),
        NumberKind::F32(_) => "f32".to_owned(),
        NumberKind::F64(_) => "f64".to_owned(),
        NumberKind::U64(_) => "u64".to_owned(),
        NumberKind::U32(_) => "u32".to_owned(),
        NumberKind::U16(_) => "u16".to_owned(),
        NumberKind::U8(_) => "u8".to_owned(),
        NumberKind::ISize(_) => "isize".to_owned(),
        NumberKind::USize(_) => "usize".to_owned(),
    }
}

impl NumberKind {
    pub fn to_string(&self) -> String {
        match self {
            NumberKind::I64(v) => format!("{}i64", v),
            NumberKind::I32(v) => format!("{}i32", v),
            NumberKind::I16(v) => format!("{}i16", v),
            NumberKind::I8(v) => format!("{}i8", v),
            NumberKind::F32(v) => format!("{}f32", v),
            NumberKind::F64(v) => format!("{}f64", v),
            NumberKind::U64(v) => format!("{}u64", v),
            NumberKind::U32(v) => format!("{}u32", v),
            NumberKind::U16(v) => format!("{}u16", v),
            NumberKind::U8(v) => format!("{}u8", v),
            NumberKind::ISize(v) => format!("{}isize", v),
            NumberKind::USize(v) => format!("{}usize", v),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(StringId),
    Punctuation(PunctuationKind),
    Keyword(KeywordKind),
    String(String),
    Number(NumberKind),
    Doc(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    input: &'a str,
    byte_offset: usize,
    grapheme_offset: usize,
    line: usize,
    col: usize,
    path: ModulePath,
}

impl<'a> Tokenizer<'a> {
    fn current(&self) -> Option<&'a str> {
        self.input.graphemes(true).nth(self.grapheme_offset)
    }

    fn consume(&mut self) {
        if let Some(c) = self.current() {
            if c == "\n" {
                self.byte_offset += c.len();
                self.line += 1;
                self.col = 1;
            } else {
                self.byte_offset += c.len();
                self.col += 1;
            }
            self.grapheme_offset += 1;
        }
    }

    fn peek(&self, i: usize) -> Option<&'a str> {
        self.input.graphemes(true).nth(self.grapheme_offset + i)
    }

    fn slice(&self, start: usize, end: usize) -> &'a str {
        let grapheme_indices: Vec<(usize, &str)> =
            self.input.grapheme_indices(true).collect();

        let start_idx = grapheme_indices[start].0;
        let end_idx = if end < grapheme_indices.len() {
            grapheme_indices[end].0
        } else {
            self.input.len()
        };

        &self.input[start_idx..end_idx]
    }

    fn synchronize(&mut self) {
        while let Some(ch) = self.current() {
            let is_whitespace = ch.chars().all(|c| c.is_whitespace());

            if is_whitespace || ch == ";" {
                self.consume();
                break;
            } else {
                self.consume();
            }
        }
    }

    fn skip_ignored(&mut self) {
        loop {
            let start_offset = self.grapheme_offset;
            self.skip_whitespace();
            self.skip_comment();

            if self.grapheme_offset == start_offset {
                break;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            let is_whitespace = ch.chars().all(|c| c.is_whitespace());

            if is_whitespace {
                self.consume();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek(0) == Some("/") && self.peek(1) == Some("/") {
            while let Some(c) = self.current() {
                if c == "\n" || c == "\r" {
                    break;
                }
                self.consume();
            }
        }
    }

    pub fn tokenize(
        input: &'a str,
        path: ModulePath,
    ) -> (Vec<Token>, Vec<TokenizationError>) {
        let mut state = Tokenizer {
            input,
            byte_offset: 0,
            grapheme_offset: 0,
            line: 1,
            col: 1,
            path: path.clone(),
        };
        let mut tokens: Vec<Token> = vec![];
        let mut errors: Vec<TokenizationError> = vec![];

        loop {
            state.skip_ignored();

            let start_pos = Position {
                line: state.line,
                col: state.col,
                byte_offset: state.byte_offset,
            };

            match state.current() {
                Some(letter) if is_letter(letter) => {
                    let identifier = state.tokenize_identifier();
                    let keyword = is_keyword(identifier);
                    let kind = if let Some(keyword_kind) = keyword {
                        TokenKind::Keyword(keyword_kind)
                    } else {
                        let id = STRING_INTERNER.intern(identifier);
                        TokenKind::Identifier(id)
                    };
                    let end_pos = Position {
                        line: state.line,
                        col: state.col,
                        byte_offset: state.byte_offset,
                    };

                    tokens.push(Token {
                        span: Span {
                            start: start_pos,
                            end: end_pos,
                            path: state.path.clone(),
                        },
                        kind,
                    });
                }
                Some("\"") => match state.string() {
                    Ok(value) => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        tokens.push(Token {
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                            kind: TokenKind::String(value.to_string()),
                        })
                    }
                    Err(kind) => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        errors.push(TokenizationError {
                            kind,
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                        });
                        state.synchronize();
                    }
                },
                Some(digit) if is_digit(digit) => match state.tokenize_number() {
                    Ok(number_kind) => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        tokens.push(Token {
                            kind: TokenKind::Number(number_kind),
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                        })
                    }
                    Err(kind) => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        errors.push(TokenizationError {
                            kind,
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                        });
                        state.synchronize();
                    }
                },
                Some("-") if state.peek(1) == Some("-") && state.peek(2) == Some("-") => {
                    match state.tokenize_documentation() {
                        Ok(content) => {
                            let end_pos = Position {
                                line: state.line,
                                col: state.col,
                                byte_offset: state.byte_offset,
                            };
                            tokens.push(Token {
                                kind: TokenKind::Doc(content.to_string()),
                                span: Span {
                                    start: start_pos,
                                    end: end_pos,
                                    path: state.path.clone(),
                                },
                            })
                        }
                        Err(kind) => {
                            let end_pos = Position {
                                line: state.line,
                                col: state.col,
                                byte_offset: state.byte_offset,
                            };
                            errors.push(TokenizationError {
                                kind,
                                span: Span {
                                    start: start_pos,
                                    end: end_pos,
                                    path: state.path.clone(),
                                },
                            });
                            state.synchronize();
                        }
                    }
                }
                Some(punct) => match state.tokenize_punctuation(punct) {
                    Some(kind) => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        tokens.push(Token {
                            kind: TokenKind::Punctuation(kind),
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                        })
                    }
                    None => {
                        let end_pos = Position {
                            line: state.line,
                            col: state.col,
                            byte_offset: state.byte_offset,
                        };
                        errors.push(TokenizationError {
                            kind: TokenizationErrorKind::UnknownToken(punct.to_string()),
                            span: Span {
                                start: start_pos,
                                end: end_pos,
                                path: state.path.clone(),
                            },
                        });
                        state.synchronize();
                    }
                },
                None => break,
            };
        }

        (tokens, errors)
    }
}

fn is_letter(value: &str) -> bool {
    value.graphemes(true).count() == 1 && value.chars().all(char::is_alphabetic)
}

fn is_digit(value: &str) -> bool {
    value.graphemes(true).count() == 1 && value.chars().all(|x| char::is_ascii_digit(&x))
}

fn is_alphanumeric(value: &str) -> bool {
    value.graphemes(true).count() == 1 && value.chars().all(char::is_alphanumeric)
}

fn is_keyword(identifier: &str) -> Option<KeywordKind> {
    match identifier {
        "fn" => Some(KeywordKind::Fn),
        "let" => Some(KeywordKind::Let),
        "return" => Some(KeywordKind::Return),
        "if" => Some(KeywordKind::If),
        "else" => Some(KeywordKind::Else),
        "while" => Some(KeywordKind::While),
        "break" => Some(KeywordKind::Break),
        "continue" => Some(KeywordKind::Continue),
        "type" => Some(KeywordKind::Type),
        "from" => Some(KeywordKind::From),
        "void" => Some(KeywordKind::Void),
        "true" => Some(KeywordKind::True),
        "false" => Some(KeywordKind::False),
        "export" => Some(KeywordKind::Export),
        "bool" => Some(KeywordKind::Bool),
        "i8" => Some(KeywordKind::I8),
        "i16" => Some(KeywordKind::I16),
        "i32" => Some(KeywordKind::I32),
        "i64" => Some(KeywordKind::I64),
        "u8" => Some(KeywordKind::U8),
        "u16" => Some(KeywordKind::U16),
        "u32" => Some(KeywordKind::U32),
        "u64" => Some(KeywordKind::U64),
        "f32" => Some(KeywordKind::F32),
        "f64" => Some(KeywordKind::F64),
        "string" => Some(KeywordKind::String),
        "null" => Some(KeywordKind::Null),
        "extern" => Some(KeywordKind::Extern),
        "unsafe" => Some(KeywordKind::Unsafe),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{ModulePath, Position, Span},
        globals::{reset_globals, STRING_INTERNER},
        tokenize::{
            KeywordKind, NumberKind, PunctuationKind, Token, TokenKind, Tokenizer,
        },
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_skip_single_line_comment() {
        reset_globals();

        let input = "// This is a comment\nlet x = 10;";
        let path = ModulePath::default();

        let x_id = STRING_INTERNER.intern("x");

        let (tokens, _) = Tokenizer::tokenize(input, path.clone());

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::Keyword(KeywordKind::Let),
                    span: Span {
                        start: Position {
                            line: 2,
                            col: 1,
                            byte_offset: 21
                        },
                        end: Position {
                            line: 2,
                            col: 4,
                            byte_offset: 24
                        },
                        path: path.clone()
                    }
                },
                Token {
                    kind: TokenKind::Identifier(x_id),
                    span: Span {
                        start: Position {
                            line: 2,
                            col: 5,
                            byte_offset: 25
                        },
                        end: Position {
                            line: 2,
                            col: 6,
                            byte_offset: 26
                        },
                        path: path.clone()
                    }
                },
                Token {
                    kind: TokenKind::Punctuation(PunctuationKind::Eq),
                    span: Span {
                        start: Position {
                            line: 2,
                            col: 7,
                            byte_offset: 27
                        },
                        end: Position {
                            line: 2,
                            col: 8,
                            byte_offset: 28
                        },
                        path: path.clone()
                    }
                },
                Token {
                    kind: TokenKind::Number(NumberKind::I64(10)),
                    span: Span {
                        start: Position {
                            line: 2,
                            col: 9,
                            byte_offset: 29
                        },
                        end: Position {
                            line: 2,
                            col: 11,
                            byte_offset: 31
                        },
                        path: path.clone()
                    }
                },
                Token {
                    kind: TokenKind::Punctuation(PunctuationKind::SemiCol),
                    span: Span {
                        start: Position {
                            line: 2,
                            col: 11,
                            byte_offset: 31
                        },
                        end: Position {
                            line: 2,
                            col: 12,
                            byte_offset: 32
                        },
                        path: path.clone()
                    }
                }
            ]
        );
    }

    #[test]
    fn test_skip_multiple_single_line_comments() {
        reset_globals();

        let input = "// Comment 1\n// Comment 2\nlet x = 10;";
        let path = ModulePath::default();

        let (tokens, _) = Tokenizer::tokenize(input, path);

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Keyword(KeywordKind::Let));
    }

    #[test]
    fn test_comment_at_end_of_input() {
        reset_globals();

        let input = "let x = 10; // Comment at the end";
        let path = ModulePath::default();

        let (tokens, _) = Tokenizer::tokenize(input, path);

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Keyword(KeywordKind::Let));
    }

    #[test]
    fn test_no_comments() {
        reset_globals();

        let input = "let x = 10;";
        let path = ModulePath::default();

        let (tokens, _) = Tokenizer::tokenize(input, path);

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Keyword(KeywordKind::Let));
    }

    #[test]
    fn test_only_comments() {
        reset_globals();

        let input = "// Only a comment";
        let path = ModulePath::default();

        let (tokens, _) = Tokenizer::tokenize(input, path);
        assert_eq!(tokens.len(), 0);
    }
}
