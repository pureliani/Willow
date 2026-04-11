use std::collections::HashSet;

use crate::{
    ast::{IdentifierNode, ModulePath, Span},
    compile::interner::{StringId, TypeId},
    mir::types::checked_type::Type,
};

#[derive(Debug, Clone)]
pub enum SemanticErrorKind {
    UnsupportedUnionNarrowing,
    GenericClosuresNotSupported,
    CannotInferGenericArgument(IdentifierNode),
    ConflictingGenericBinding {
        param_name: IdentifierNode,
        expected: TypeId,
        received: TypeId,
    },
    AmbiguousGenericInference,
    MainFunctionCannotHaveParameters,
    MainFunctionInvalidReturnType,
    MainFunctionMustBeInEntryFile,
    CannotNarrowNonUnion(TypeId),
    ValuedTagInIsExpression,
    UnreachableCode,
    DuplicateIdentifier(IdentifierNode),
    CannotIndex(TypeId),
    FromStatementMustBeDeclaredAtTopLevel,
    MissingGenericArguments,
    ModuleNotFound(ModulePath),
    CannotDeclareGlobalVariable,
    DuplicateStructFieldInitializer(IdentifierNode),
    UnknownStructFieldInitializer(IdentifierNode),
    MissingStructFieldInitializers(HashSet<StringId>),
    CannotCall(TypeId),
    ExpectedANumericOperand,
    ExpectedASignedNumericOperand,
    MixedSignedAndUnsigned,
    MixedFloatAndInteger,
    CannotCompareType {
        of: TypeId,
        to: TypeId,
    },
    UndeclaredIdentifier(IdentifierNode),
    UndeclaredType(IdentifierNode),
    ReturnKeywordOutsideFunction,
    BreakKeywordOutsideLoop,
    ContinueKeywordOutsideLoop,
    InvalidLValue,
    CannotGetLen(Type),
    TypeMismatch {
        expected: TypeId,
        received: TypeId,
    },
    ReturnTypeMismatch {
        expected: TypeId,
        received: TypeId,
    },
    CannotAccess(TypeId),
    CannotStaticAccess(TypeId),
    AccessToUndefinedField(IdentifierNode),
    AccessToUndefinedStaticField(IdentifierNode),
    FnArgumentCountMismatch {
        expected: usize,
        received: usize,
    },
    GenericArgumentCountMismatch {
        expected: usize,
        received: usize,
    },
    CannotApplyTypeArguments,
    IdentifierIsNotAType(IdentifierNode),
    CannotUseTypeDeclarationAsValue,
    TypeAliasMustBeDeclaredAtTopLevel,
    IfExpressionMissingElse,
    CannotCastType {
        source_type: TypeId,
        target_type: TypeId,
    },
    TryExplicitCast,
    SymbolNotExported {
        module_path: ModulePath,
        symbol: IdentifierNode,
    },
    ClosuresNotSupportedYet,
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum SemanticErrorSeverity {
    Error,
    Warn,
    Info,
}

impl SemanticErrorKind {
    pub fn severity(&self) -> SemanticErrorSeverity {
        match self {
            SemanticErrorKind::ExpectedANumericOperand => SemanticErrorSeverity::Error,
            SemanticErrorKind::MixedSignedAndUnsigned => SemanticErrorSeverity::Error,
            SemanticErrorKind::MixedFloatAndInteger => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotCompareType { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::UndeclaredIdentifier { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::ReturnKeywordOutsideFunction => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::BreakKeywordOutsideLoop => SemanticErrorSeverity::Error,
            SemanticErrorKind::ContinueKeywordOutsideLoop => SemanticErrorSeverity::Error,
            SemanticErrorKind::InvalidLValue => SemanticErrorSeverity::Error,
            SemanticErrorKind::TypeMismatch { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::ReturnTypeMismatch { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::UndeclaredType { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotAccess { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotCall { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::AccessToUndefinedField { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::FnArgumentCountMismatch { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::TypeAliasMustBeDeclaredAtTopLevel => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::DuplicateStructFieldInitializer { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::UnknownStructFieldInitializer { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::MissingStructFieldInitializers { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::DuplicateIdentifier { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::IfExpressionMissingElse => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotCastType { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotIndex { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotStaticAccess { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::AccessToUndefinedStaticField { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::CannotUseTypeDeclarationAsValue => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::CannotDeclareGlobalVariable => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::UnreachableCode => SemanticErrorSeverity::Error,
            SemanticErrorKind::FromStatementMustBeDeclaredAtTopLevel => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::ModuleNotFound { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::IdentifierIsNotAType { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::SymbolNotExported { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::ClosuresNotSupportedYet => SemanticErrorSeverity::Error,
            SemanticErrorKind::ValuedTagInIsExpression => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotNarrowNonUnion(_) => SemanticErrorSeverity::Error,
            SemanticErrorKind::UnsupportedUnionNarrowing => SemanticErrorSeverity::Error,
            SemanticErrorKind::ExpectedASignedNumericOperand => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::CannotGetLen { .. } => SemanticErrorSeverity::Error,
            SemanticErrorKind::TryExplicitCast => SemanticErrorSeverity::Error,
            SemanticErrorKind::MainFunctionCannotHaveParameters => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::MainFunctionInvalidReturnType => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::MainFunctionMustBeInEntryFile => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::GenericArgumentCountMismatch { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::CannotApplyTypeArguments => SemanticErrorSeverity::Error,
            SemanticErrorKind::MissingGenericArguments => SemanticErrorSeverity::Error,
            SemanticErrorKind::ConflictingGenericBinding { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::AmbiguousGenericInference => SemanticErrorSeverity::Error,
            SemanticErrorKind::CannotInferGenericArgument { .. } => {
                SemanticErrorSeverity::Error
            }
            SemanticErrorKind::GenericClosuresNotSupported => {
                SemanticErrorSeverity::Error
            }
        }
    }

    pub fn code(&self) -> usize {
        match self {
            SemanticErrorKind::ExpectedANumericOperand => 1,
            SemanticErrorKind::MixedSignedAndUnsigned => 2,
            SemanticErrorKind::MixedFloatAndInteger => 3,
            SemanticErrorKind::CannotCompareType { .. } => 4,
            SemanticErrorKind::UndeclaredIdentifier { .. } => 5,
            SemanticErrorKind::ReturnKeywordOutsideFunction => 6,
            SemanticErrorKind::BreakKeywordOutsideLoop => 7,
            SemanticErrorKind::ContinueKeywordOutsideLoop => 8,
            SemanticErrorKind::InvalidLValue => 9,
            SemanticErrorKind::TypeMismatch { .. } => 10,
            SemanticErrorKind::ReturnTypeMismatch { .. } => 11,
            SemanticErrorKind::UndeclaredType { .. } => 12,
            SemanticErrorKind::CannotAccess { .. } => 13,
            SemanticErrorKind::CannotCall { .. } => 14,
            SemanticErrorKind::AccessToUndefinedField { .. } => 16,
            SemanticErrorKind::FnArgumentCountMismatch { .. } => 17,
            SemanticErrorKind::TypeAliasMustBeDeclaredAtTopLevel => 18,
            SemanticErrorKind::DuplicateStructFieldInitializer { .. } => 19,
            SemanticErrorKind::UnknownStructFieldInitializer { .. } => 20,
            SemanticErrorKind::MissingStructFieldInitializers { .. } => 21,
            SemanticErrorKind::DuplicateIdentifier { .. } => 22,
            SemanticErrorKind::IfExpressionMissingElse => 23,
            SemanticErrorKind::CannotCastType { .. } => 24,
            SemanticErrorKind::CannotIndex { .. } => 25,
            SemanticErrorKind::CannotStaticAccess { .. } => 26,
            SemanticErrorKind::AccessToUndefinedStaticField { .. } => 27,
            SemanticErrorKind::CannotUseTypeDeclarationAsValue => 28,
            SemanticErrorKind::CannotDeclareGlobalVariable => 29,
            SemanticErrorKind::UnreachableCode => 30,
            SemanticErrorKind::FromStatementMustBeDeclaredAtTopLevel => 31,
            SemanticErrorKind::ModuleNotFound { .. } => 32,
            SemanticErrorKind::IdentifierIsNotAType { .. } => 33,
            SemanticErrorKind::SymbolNotExported { .. } => 34,
            SemanticErrorKind::ClosuresNotSupportedYet => 35,
            SemanticErrorKind::ValuedTagInIsExpression => 36,
            SemanticErrorKind::CannotNarrowNonUnion(_) => 37,
            SemanticErrorKind::UnsupportedUnionNarrowing => 38,
            SemanticErrorKind::ExpectedASignedNumericOperand => 39,
            SemanticErrorKind::CannotGetLen { .. } => 40,
            SemanticErrorKind::TryExplicitCast => 42,
            SemanticErrorKind::MainFunctionInvalidReturnType => 43,
            SemanticErrorKind::MainFunctionMustBeInEntryFile => 44,
            SemanticErrorKind::MainFunctionCannotHaveParameters => 45,
            SemanticErrorKind::GenericArgumentCountMismatch { .. } => 46,
            SemanticErrorKind::CannotApplyTypeArguments => 47,
            SemanticErrorKind::MissingGenericArguments => 48,
            SemanticErrorKind::ConflictingGenericBinding { .. } => 49,
            SemanticErrorKind::AmbiguousGenericInference => 50,
            SemanticErrorKind::CannotInferGenericArgument { .. } => 51,
            SemanticErrorKind::GenericClosuresNotSupported => 52,
        }
    }
}
