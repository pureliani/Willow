use crate::ast::{decl::FnDecl, IdentifierNode, Span, StringNode};

use super::{
    decl::{TypeAliasDecl, VarDecl},
    expr::{BlockContents, Expr},
};

#[derive(Clone, Debug, PartialEq)]
pub enum ImportItem {
    Symbol {
        identifier: IdentifierNode,
        alias: Option<IdentifierNode>,
    },
    ExternFn(FnDecl),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Expression(Expr),
    TypeAliasDecl(TypeAliasDecl),
    VarDecl(VarDecl),
    Break,
    Continue,
    Return {
        value: Expr,
    },
    Assignment {
        target: Expr,
        value: Expr,
    },
    From {
        path: StringNode,
        items: Vec<ImportItem>,
    },
    While {
        condition: Box<Expr>,
        body: BlockContents,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
