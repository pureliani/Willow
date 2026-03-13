use crate::{
    ast::{decl::FnDecl, IdentifierNode, Span, StringNode},
    tokenize::NumberKind,
};

use super::{stmt::Stmt, type_annotation::TypeAnnotation};

#[derive(Clone, Debug, PartialEq)]
pub struct BlockContents {
    pub statements: Vec<Stmt>,
    pub final_expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Null,
    Not {
        right: Box<Expr>,
    },
    Neg {
        right: Box<Expr>,
    },
    Add {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Subtract {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Multiply {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Divide {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Modulo {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    LessThan {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    LessThanOrEqual {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    GreaterThan {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    GreaterThanOrEqual {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Equal {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    NotEqual {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    And {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Or {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Struct(Vec<(IdentifierNode, Expr)>),
    Access {
        left: Box<Expr>,
        field: IdentifierNode,
    },
    StaticAccess {
        left: Box<Expr>,
        field: IdentifierNode,
    },
    TypeCast {
        left: Box<Expr>,
        target: TypeAnnotation,
    },
    IsType {
        left: Box<Expr>,
        ty: TypeAnnotation,
    },
    FnCall {
        left: Box<Expr>,
        args: Vec<Expr>,
    },
    BoolLiteral(bool),
    Number(NumberKind),
    String(StringNode),
    Identifier(IdentifierNode),
    Fn(Box<FnDecl>),
    If {
        branches: Vec<(Box<Expr>, BlockContents)>,
        else_branch: Option<BlockContents>,
    },
    List(Vec<Expr>),
    CodeBlock(BlockContents),
    UnsafeBlock(BlockContents),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
