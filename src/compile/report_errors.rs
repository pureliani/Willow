use std::io::ErrorKind;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::Files;
use codespan_reporting::term::termcolor::{ColorChoice, NoColor, StandardStream};
use codespan_reporting::term::{self, Config};

use crate::compile::file_cache::FileCache;
use crate::mir::utils::points_to::PathSegment;
use crate::{
    ast::{ModulePath, Span},
    compile::{Compiler, CompilerErrorKind},
    globals::STRING_INTERNER,
    mir::errors::SemanticErrorKind,
    parse::ParsingErrorKind,
    tokenize::TokenizationErrorKind,
};

/// Generates a sort key: (Rank, Path, Offset)
/// Rank 0 = Spanned errors (come first)
/// Rank 1 = Global/IO errors (come last)
fn get_err_sort_key(err: &CompilerErrorKind) -> (u8, std::path::PathBuf, usize) {
    match err {
        CompilerErrorKind::Tokenization(e) => {
            (0, e.span.path.0.to_path_buf(), e.span.start.byte_offset)
        }
        CompilerErrorKind::Parsing(e) => {
            (0, e.span.path.0.to_path_buf(), e.span.start.byte_offset)
        }
        CompilerErrorKind::Semantic(e) => {
            (0, e.span.path.0.to_path_buf(), e.span.start.byte_offset)
        }
        CompilerErrorKind::CouldNotReadFile { path, .. } => (1, path.0.to_path_buf(), 0),
        CompilerErrorKind::ModuleNotFound { target_path, .. } => {
            (1, target_path.0.to_path_buf(), 0)
        }
        CompilerErrorKind::MissingMainFunction(module_path) => {
            (2, module_path.0.to_path_buf(), 0)
        }
    }
}

impl Compiler {
    pub fn report_errors(&mut self) {
        let cache = self.files.lock().unwrap();
        let config = Config {
            start_context_lines: 8,
            end_context_lines: 8,
            tab_width: 8,
            after_label_lines: 8,
            before_label_lines: 8,
            display_style: term::DisplayStyle::Rich,
            ..Default::default()
        };

        let mut buffer: Vec<u8> = Vec::new();
        let mut buf_writer = NoColor::new(&mut buffer);
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);

        self.errors.sort_by_key(get_err_sort_key);

        for error in &self.errors {
            let diagnostic = match error {
                CompilerErrorKind::Tokenization(e) => {
                    let (path, range) = self.extract_span(&e.span);
                    let Ok(file_id) = self.resolve_file_id(&cache, &path) else {
                        println!("Error: File not found: {:?}", path);
                        continue;
                    };

                    let diag = Diagnostic::error()
                        .with_code(format!("T{}", e.kind.code()))
                        .with_labels(vec![Label::primary(file_id, range.clone())]);

                    match &e.kind {
                        TokenizationErrorKind::UnterminatedString => diag
                            .with_message("Unterminated string")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("This string is not terminated")]),

                        TokenizationErrorKind::UnknownToken(char_str) => {
                            let readable = match char_str.as_str() {
                                "\n" => "`\\n`",
                                "\r" => "`\\r`",
                                "\t" => "`\\t`",
                                " " => "`<whitespace>`",
                                c => &format!("'{}'", c),
                            };
                            diag.with_message("Unknown token").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "This character {} is not recognized",
                                    readable
                                )),
                            ])
                        }

                        TokenizationErrorKind::UnknownEscapeSequence => diag
                            .with_message("Unknown escape sequence")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("The escape sequence here is invalid")]),

                        TokenizationErrorKind::InvalidFloatingNumber => diag
                            .with_message("Invalid floating-point number")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "This is not a valid floating-point number",
                                )]),

                        TokenizationErrorKind::InvalidIntegerNumber => diag
                            .with_message("Invalid integer number")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("This is not a valid integer number")]),

                        TokenizationErrorKind::UnterminatedDoc => diag
                            .with_message("Unterminated documentation")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "This documentation block is not terminated",
                                )]),
                    }
                }

                CompilerErrorKind::Parsing(e) => {
                    let (path, range) = self.extract_span(&e.span);
                    let Ok(file_id) = self.resolve_file_id(&cache, &path) else {
                        println!("Error: File not found: {:?}", path);
                        continue;
                    };

                    let diag = Diagnostic::error()
                        .with_code(format!("P{}", e.kind.code()))
                        .with_labels(vec![Label::primary(file_id, range.clone())]);

                    match &e.kind {
                        ParsingErrorKind::DocMustBeFollowedByDeclaration => diag
                            .with_message(
                                "Documentation must be followed by a declaration",
                            )
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Expected a type alias or variable here")]),

                        ParsingErrorKind::ExpectedAnExpressionButFound(token) => diag
                            .with_message("Expected an expression")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Expected an expression, found token `{}`",
                                    token.kind
                                ))]),

                        ParsingErrorKind::ExpectedATypeButFound(token) => diag
                            .with_message("Expected a type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Expected a type, found token `{}`",
                                    token.kind
                                ))]),

                        ParsingErrorKind::InvalidSuffixOperator(token) => diag
                            .with_message("Invalid suffix operator")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Token `{}` cannot be used as a suffix",
                                    token.kind
                                ))]),

                        ParsingErrorKind::UnexpectedEndOfInput => diag
                            .with_message("Unexpected end of input")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Input ended abruptly")]),

                        ParsingErrorKind::ExpectedAnIdentifier => diag
                            .with_message("Expected an identifier")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Expected an identifier")]),

                        ParsingErrorKind::ExpectedAPunctuationMark(p) => diag
                            .with_message("Expected punctuation")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!("Expected `{}`", p.to_string()))]),

                        ParsingErrorKind::ExpectedAKeyword(k) => diag
                            .with_message("Expected keyword")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!("Expected `{}`", k.to_string()))]),

                        ParsingErrorKind::ExpectedAStringValue => diag
                            .with_message("Expected string literal")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Expected a string")]),

                        ParsingErrorKind::ExpectedANumericValue => diag
                            .with_message("Expected numeric literal")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Expected a number")]),

                        ParsingErrorKind::UnknownStaticMethod(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Unknown static method").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "Method `{}` doesn't exist",
                                    name
                                )),
                            ])
                        }

                        ParsingErrorKind::UnexpectedStatementAfterFinalExpression => diag
                            .with_message("Unexpected statement")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Statements cannot follow the final expression",
                                )]),

                        ParsingErrorKind::ExpectedStatementOrExpression { found } => diag
                            .with_message("Expected statement or expression")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!("Found `{}`", found.kind))]),

                        ParsingErrorKind::UnexpectedTokenAfterFinalExpression {
                            found,
                        } => diag.with_message("Unexpected token").with_labels(vec![
                            Label::primary(file_id, range).with_message(format!(
                                "Token `{}` follows final expression",
                                found.kind
                            )),
                        ]),

                        ParsingErrorKind::ExpectedATagTypeButFound(_) => diag
                            .with_message("Expected a tag type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Union variants must start with '#'")]),

                        ParsingErrorKind::ExpectedToBeFollowedByOneOfTheTokens(
                            tokens,
                        ) => {
                            let expected: Vec<_> =
                                tokens.iter().map(|t| format!("{}", t.kind)).collect();
                            diag.with_message("Unexpected token").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "Expected one of: {}",
                                    expected.join(", ")
                                )),
                            ])
                        }
                    }
                }

                CompilerErrorKind::Semantic(e) => {
                    let (path, range) = self.extract_span(&e.span);
                    let Ok(file_id) = self.resolve_file_id(&cache, &path) else {
                        println!("Error: File not found: {:?}", path);
                        continue;
                    };

                    let diag = Diagnostic::error()
                        .with_code(format!("S{}", e.kind.code()))
                        .with_labels(vec![Label::primary(file_id, range.clone())]);

                    match &e.kind {
                        SemanticErrorKind::ArgumentAliasing {
                            passed_arg_span,
                            passed_path,
                            aliased_arg_span,
                            aliased_path,
                        } => {
                            let get_snippet = |span: &Span| -> String {
                                if let Ok(f_id) = self.resolve_file_id(&cache, &span.path)
                                {
                                    if let Ok(source) = cache.source(f_id) {
                                        let start = span.start.byte_offset;
                                        let end = span.end.byte_offset;
                                        if start <= source.len() && end <= source.len() {
                                            return source[start..end].to_string();
                                        }
                                    }
                                }
                                "<unknown>".to_string()
                            };

                            let format_full_path =
                                |span: &Span, path: &[PathSegment]| -> String {
                                    let mut s = get_snippet(span);
                                    for seg in path {
                                        match seg {
                                            PathSegment::Field(name) => {
                                                s.push('.');
                                                s.push_str(
                                                    &STRING_INTERNER.resolve(*name),
                                                );
                                            }
                                            PathSegment::Index => {
                                                s.push_str("[index]");
                                            }
                                        }
                                    }
                                    s
                                };

                            let passed_str =
                                format_full_path(passed_arg_span, passed_path);
                            let aliased_str =
                                format_full_path(aliased_arg_span, aliased_path);

                            diag.with_message("Argument aliasing detected").with_labels(
                                vec![Label::primary(file_id, range).with_message(
                                    format!(
                                        "Cannot pass argument `{}` which is an alias of \
                                         another argument `{}`",
                                        passed_str, aliased_str
                                    ),
                                )],
                            )
                        }
                        SemanticErrorKind::CannotGetLen(_ty) => {
                            diag.with_message("Cannot get length")
                        }
                        SemanticErrorKind::CannotNarrowNonUnion(ty) => {
                            let type_str = self.types.to_string(*ty);
                            diag.with_message("Redundant type check").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "Value is already `{}`, `::is()` only works on \
                                     unions",
                                    type_str
                                )),
                            ])
                        }
                        SemanticErrorKind::ExpectedANumericOperand => diag
                            .with_message("Expected numeric operand")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("This must be a numeric type")]),
                        SemanticErrorKind::ExpectedASignedNumericOperand => diag
                            .with_message("Expected signed numeric operand")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("This must be a signed numeric type")]),
                        SemanticErrorKind::MixedSignedAndUnsigned => diag
                            .with_message("Mixed signedness")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot mix signed and unsigned operands",
                                )]),
                        SemanticErrorKind::MixedFloatAndInteger => diag
                            .with_message("Mixed types")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Cannot mix float and integer operands")]),
                        SemanticErrorKind::CannotCompareType { of, to } => diag
                            .with_message("Incompatible comparison")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Cannot compare `{}` to `{}`",
                                    self.types.to_string(*of),
                                    self.types.to_string(*to)
                                ))]),
                        SemanticErrorKind::UndeclaredIdentifier(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Undeclared identifier")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!("`{}` is not defined", name))])
                        }
                        SemanticErrorKind::UndeclaredType(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Undeclared type").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "Type `{}` is not defined",
                                    name
                                )),
                            ])
                        }
                        SemanticErrorKind::TypeMismatch { expected, received } => diag
                            .with_message("Type mismatch")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Expected `{}`, found `{}`",
                                    self.types.to_string(*expected),
                                    self.types.to_string(*received)
                                ))]),
                        SemanticErrorKind::ReturnTypeMismatch { expected, received } => {
                            diag.with_message("Function return type mismatch")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "Expected the returned value to have a type \
                                         that is assignable to `{}`, but found `{}`",
                                        self.types.to_string(*expected),
                                        self.types.to_string(*received)
                                    ))])
                        }
                        SemanticErrorKind::ModuleNotFound(path_buf) => diag
                            .with_message("Module not found")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Could not find module at `{}`",
                                    path_buf.0.display()
                                ))]),
                        SemanticErrorKind::SymbolNotExported {
                            module_path,
                            symbol,
                        } => {
                            let name = STRING_INTERNER.resolve(symbol.name);
                            diag.with_message("Symbol not exported").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "`{}` is not exported from `{}`",
                                    name,
                                    module_path.0.display()
                                )),
                            ])
                        }
                        SemanticErrorKind::ValuedTagInIsExpression => diag
                            .with_message("Valued tag not allowed in `::is()` expression")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "The `::is()` operator only checks the variant \
                                     identifier. Remove the value type (e.g., use \
                                     `#Tag` instead of `#Tag(Type)`)",
                                )]),
                        SemanticErrorKind::UnreachableCode => diag
                            .with_message("Unreachable code")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("This code will never be executed")]),
                        SemanticErrorKind::DuplicateIdentifier(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Duplicate identifier").with_labels(vec![
                                Label::primary(file_id, range).with_message(format!(
                                    "Duplicate identifier declaration `{}`",
                                    name
                                )),
                            ])
                        }
                        SemanticErrorKind::CannotIndex(ty) => diag
                            .with_message("Cannot index type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Type `{}` cannot be indexed",
                                    self.types.to_string(*ty)
                                ))]),
                        SemanticErrorKind::FromStatementMustBeDeclaredAtTopLevel => diag
                            .with_message("Invalid import location")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "`from` statements must be declared at the top \
                                     level of the file",
                                )]),
                        SemanticErrorKind::CannotDeclareGlobalVariable => diag
                            .with_message("Global variables not allowed")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Variables cannot be declared at the file scope \
                                     (top-level)",
                                )]),
                        SemanticErrorKind::DuplicateStructFieldInitializer(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Duplicate initializer for a struct field")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "Struct field `{}` cannot be initialized \
                                         multiple times",
                                        name
                                    ))])
                        }
                        SemanticErrorKind::UnknownStructFieldInitializer(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Unknown field in the struct initializer")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "Unknown struct field `{}`",
                                        name
                                    ))])
                        }
                        SemanticErrorKind::MissingStructFieldInitializers(
                            missing_fields,
                        ) => {
                            let field_names: Vec<String> = missing_fields
                                .iter()
                                .map(|f| STRING_INTERNER.resolve(*f))
                                .collect();
                            let joined = field_names
                                .iter()
                                .map(|n| format!("`{}`", n))
                                .collect::<Vec<_>>()
                                .join(", ");
                            diag.with_message("Missing field initializers").with_labels(
                                vec![Label::primary(file_id, range).with_message(
                                    format!(
                                        "Missing initializers for the following struct \
                                         fields {}",
                                        joined
                                    ),
                                )],
                            )
                        }
                        SemanticErrorKind::CannotCall(target) => diag
                            .with_message("Cannot use the function call operator")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Cannot use the function-call operator on type `{}`",
                                    self.types.to_string(*target)
                                ))]),
                        SemanticErrorKind::ReturnKeywordOutsideFunction => diag
                            .with_message(
                                "Keyword `return` used outside of a function scope",
                            )
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use the `return` keyword outside of a \
                                     function scope",
                                )]),
                        SemanticErrorKind::BreakKeywordOutsideLoop => diag
                            .with_message("Keyword `break` used outside of a loop scope")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use the `break` keyword outside of a loop \
                                     scope",
                                )]),
                        SemanticErrorKind::ContinueKeywordOutsideLoop => diag
                            .with_message(
                                "Keyword `continue` used outside of a loop scope",
                            )
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use the `continue` keyword outside of a \
                                     loop scope",
                                )]),
                        SemanticErrorKind::InvalidLValue => diag
                            .with_message("Invalid assignment target")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Invalid assignment target")]),
                        SemanticErrorKind::CannotAccess(target) => diag
                            .with_message("Cannot access field")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(format!(
                                    "Cannot use the access operator on the type `{}`",
                                    self.types.to_string(*target)
                                ))]),
                        SemanticErrorKind::CannotStaticAccess(_) => diag
                            .with_message("Cannot access static field")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("Invalid static access")]),
                        SemanticErrorKind::AccessToUndefinedField(field) => {
                            let name = STRING_INTERNER.resolve(field.name);
                            diag.with_message("Access to an undefined field")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "Field `{}` is not defined",
                                        name
                                    ))])
                        }
                        SemanticErrorKind::AccessToUndefinedStaticField(id) => {
                            let name = STRING_INTERNER.resolve(id.name);
                            diag.with_message("Undefined static field")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "Static field `{}` does not exist",
                                        name
                                    ))])
                        }
                        SemanticErrorKind::FnArgumentCountMismatch {
                            expected,
                            received,
                        } => {
                            let s = if *expected > 1 { "s" } else { "" };
                            diag.with_message("Function argument count mismatch")
                                .with_labels(vec![Label::primary(file_id, range)
                                    .with_message(format!(
                                        "This function expects {} argument{}, but \
                                         instead received {}",
                                        expected, s, received
                                    ))])
                        }
                        SemanticErrorKind::CannotUseVariableDeclarationAsType => diag
                            .with_message("Cannot use variable declaration as a type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use variable declaration as a type",
                                )]),
                        SemanticErrorKind::CannotUseFunctionDeclarationAsType => diag
                            .with_message("Expected type, found function")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use a function declaration as a type",
                                )]),
                        SemanticErrorKind::CannotUseTypeDeclarationAsValue => diag
                            .with_message("Expected value, found type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Cannot use a type declaration as a value",
                                )]),
                        SemanticErrorKind::TypeAliasMustBeDeclaredAtTopLevel => diag
                            .with_message(
                                "Type aliases must be declared in the file scope",
                            )
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Type aliases must be declared in the file scope",
                                )]),
                        SemanticErrorKind::IfExpressionMissingElse => diag
                            .with_message("`if` expression missing `else` block")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "`if` expressions used as values must have an \
                                     `else` block",
                                )]),
                        SemanticErrorKind::CannotCastType {
                            source_type,
                            target_type,
                        } => diag.with_message("Invalid type cast").with_labels(vec![
                            Label::primary(file_id, range).with_message(format!(
                                "Cannot cast type `{}` to `{}`",
                                self.types.to_string(*source_type),
                                self.types.to_string(*target_type)
                            )),
                        ]),
                        SemanticErrorKind::ClosuresNotSupportedYet => diag
                            .with_message("Closures not supported")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Capturing variables from outer scopes (closures) \
                                     is not supported yet",
                                )]),
                        SemanticErrorKind::UnsupportedUnionNarrowing => diag
                            .with_message("Union-to-union narrowing not supported")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Narrowing to a subset union type is not yet \
                                     supported; try narrowing to a specific variant \
                                     instead",
                                )]),
                        SemanticErrorKind::TryExplicitCast => diag
                            .with_message("Try explicit casting to the target type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "Try to explicitly cast this value into the target \
                                     type using the ::as(T) method",
                                )])
                            .with_note("::as(T) method has runtime overhead"),
                        SemanticErrorKind::MainFunctionCannotHaveParameters => diag
                            .with_message("Main function cannot have parameters")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "main must be declared as fn main() or fn main(): \
                                     i32",
                                )]),
                        SemanticErrorKind::MainFunctionInvalidReturnType => diag
                            .with_message("Main function has invalid return type")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message("main must return void or i32")]),
                        SemanticErrorKind::MainFunctionMustBeInEntryFile => diag
                            .with_message("Main function must be in the entry file")
                            .with_labels(vec![Label::primary(file_id, range)
                                .with_message(
                                    "main can only be defined in the entry file passed \
                                     to the compiler",
                                )]),
                    }
                }
                CompilerErrorKind::CouldNotReadFile { path, error } => {
                    println!(
                        "Error: Could not read file `{}`: {}",
                        path.0.display(),
                        error
                    );
                    continue;
                }
                CompilerErrorKind::ModuleNotFound {
                    importing_module,
                    target_path,
                    error,
                } => {
                    println!(
                        "Error: Module `{}` (imported by `{}`) not found: {}",
                        target_path.0.display(),
                        importing_module.0.display(),
                        error
                    );
                    continue;
                }
                CompilerErrorKind::MissingMainFunction(module_path) => {
                    println!(
                        "Missing the main function in the entry path {}",
                        module_path.0.display(),
                    );
                    continue;
                }
            };

            if let Err(e) =
                term::emit_to_write_style(&mut stderr, &config, &*cache, &diagnostic)
            {
                eprintln!("Failed to emit diagnostic to stderr: {}", e);
            }

            if let Err(e) =
                term::emit_to_write_style(&mut buf_writer, &config, &*cache, &diagnostic)
            {
                eprintln!("Failed to emit diagnostic to buffer: {}", e);
            }
        }

        if !buffer.is_empty() {
            if let Err(e) = std::fs::write("diagnostics.log", &buffer) {
                eprintln!("Failed to write diagnostics to file: {}", e);
            }
        } else if let Err(e) = std::fs::remove_file("diagnostics.log") {
            if !matches!(e.kind(), ErrorKind::NotFound) {
                eprintln!("Failed to remove the stale diagnostics to file: {}", e);
            }
        }
    }

    fn resolve_file_id(&self, cache: &FileCache, path: &ModulePath) -> Result<usize, ()> {
        cache.get_id(path).ok_or(())
    }

    fn extract_span(&self, span: &Span) -> (ModulePath, std::ops::Range<usize>) {
        (
            span.path.clone(),
            span.start.byte_offset..span.end.byte_offset,
        )
    }
}
