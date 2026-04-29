use crate::codegen::CodeGenerator;
use crate::compile::interner::TypeInterner;
use crate::hir::statements::from::is_linkable_external_file;
use crate::{
    ast::{
        decl::Declaration,
        expr::{Expr, ExprKind},
        stmt::{Stmt, StmtKind},
        ModulePath, Span,
    },
    compile::file_cache::FileCache,
    globals::STRING_INTERNER,
    hir::{
        builders::{Builder, InGlobal, Program},
        errors::SemanticError,
        utils::{
            dump::dump_program,
            scope::{Scope, ScopeKind},
        },
    },
    parse::{Parser, ParsingError},
    tokenize::{TokenizationError, Tokenizer},
};
use inkwell::context::Context;
use inkwell::targets::*;
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;
use std::collections::BTreeMap;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub mod file_cache;
pub mod interner;
pub mod report_errors;

fn get_runtime_files() -> Vec<PathBuf> {
    let mut runtime_files = Vec::new();

    let runtime_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("runtime");

    if !runtime_dir.exists() {
        panic!("INTERNAL COMPILER ERROR: Could not locate the runtime directory");
    }

    let mut dirs_to_visit = vec![runtime_dir];

    while let Some(dir) = dirs_to_visit.pop() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs_to_visit.push(path);
                } else if is_linkable_external_file(
                    path.extension().and_then(|e| e.to_str()),
                ) {
                    runtime_files.push(path);
                }
            }
        }
    }

    runtime_files
}

pub struct CompileOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub target: Option<String>,
    pub opt_level: u8,
    pub emit_hir: bool,
    pub emit_llvm_ir: bool,
    pub emit_obj: bool,
}

impl CompileOptions {
    fn optimization_level(&self) -> OptimizationLevel {
        match self.opt_level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Less,
            2 => OptimizationLevel::Default,
            3 => OptimizationLevel::Aggressive,
            _ => unreachable!("clap enforces 0..=3"),
        }
    }

    fn target_triple(&self) -> TargetTriple {
        match &self.target {
            Some(t) => TargetTriple::create(t),
            None => TargetMachine::get_default_triple(),
        }
    }

    fn object_path(&self) -> PathBuf {
        if self.emit_obj {
            self.output.clone()
        } else {
            self.output.with_extension("o")
        }
    }
}

#[derive(Debug)]
pub enum CompilerErrorKind {
    CouldNotReadFile {
        path: ModulePath,
        error: std::io::Error,
    },
    ModuleNotFound {
        importing_module: ModulePath,
        target_path: ModulePath,
        error: std::io::Error,
    },
    Tokenization(TokenizationError),
    Parsing(ParsingError),
    Semantic(SemanticError),
    MissingMainFunction(ModulePath),
}

pub struct Compiler {
    files: Arc<Mutex<FileCache>>,
    pub types: TypeInterner,
    errors: Vec<CompilerErrorKind>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            files: Arc::new(Mutex::new(FileCache::default())),
            types: TypeInterner::default(),
            errors: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ParallelParseResult {
    pub path: ModulePath,
    pub statements: Vec<Stmt>,
    pub tokenization_errors: Vec<TokenizationError>,
    pub parsing_errors: Vec<ParsingError>,
    pub declarations: Vec<Declaration>,
}

impl Compiler {
    pub fn compile(&mut self, options: CompileOptions) {
        let canonical_main = ModulePath(Arc::new(
            options
                .input
                .canonicalize()
                .expect("Could not find the main module"),
        ));

        let parsed_modules = self.parallel_parse_modules(canonical_main.clone());
        let mut modules_to_compile = Vec::new();

        for m in parsed_modules {
            match m {
                Err(e) => self.errors.push(e),
                Ok(mut module) => {
                    let has_tokenization_errors = !module.tokenization_errors.is_empty();
                    let has_parsing_errors = !module.parsing_errors.is_empty();

                    self.errors.extend(
                        std::mem::take(&mut module.tokenization_errors)
                            .into_iter()
                            .map(CompilerErrorKind::Tokenization),
                    );

                    self.errors.extend(
                        std::mem::take(&mut module.parsing_errors)
                            .into_iter()
                            .map(CompilerErrorKind::Parsing),
                    );

                    if !has_tokenization_errors && !has_parsing_errors {
                        modules_to_compile.push(module);
                    }
                }
            };
        }

        self.report_errors();
        if !self.errors.is_empty() {
            return;
        }

        Target::initialize_all(&InitializationConfig::default());
        let triple = options.target_triple();

        let target = Target::from_triple(&triple).unwrap_or_else(|e| {
            eprintln!("Unsupported target '{}': {}", triple, e.to_string());
            std::process::exit(1);
        });

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                options.optimization_level(),
                RelocMode::PIC,
                CodeModel::Default,
            )
            .unwrap_or_else(|| {
                eprintln!("Failed to create target machine for '{}'", triple);
                std::process::exit(1);
            });

        let context = Context::create();
        let target_data = target_machine.get_target_data();

        let target_ptr_size = target_data.get_pointer_byte_size(None) as usize;
        let ptr_type = context.ptr_type(AddressSpace::default());
        let target_ptr_align = target_data.get_abi_alignment(&ptr_type) as usize;

        let mut builder_errors = vec![];
        let mut current_facts = HashMap::new();
        let mut incomplete_fact_merges = HashMap::new();
        let mut condition_facts = HashMap::new();
        let mut aliases = HashMap::new();
        let mut own_declarations = HashSet::new();

        let mut program = Program {
            entry_path: Some(canonical_main.clone()),
            declarations: BTreeMap::new(),
            modules: BTreeMap::new(),
            foreign_links: HashSet::new(),
            target_ptr_size,
            target_ptr_align,
            generic_declarations: BTreeMap::new(),
            monomorphizations: BTreeMap::new(),
        };

        let global_scope = Scope::new_root(ScopeKind::Global, Span::default());

        let mut program_builder = Builder {
            context: InGlobal,
            current_scope: global_scope,
            errors: &mut builder_errors,
            program: &mut program,
            current_facts: &mut current_facts,
            incomplete_fact_merges: &mut incomplete_fact_merges,
            condition_facts: &mut condition_facts,
            aliases: &mut aliases,
            types: &self.types,
            own_declarations: &mut own_declarations,
        };

        program_builder.build(modules_to_compile);

        let entry_module = program.modules.get(&canonical_main);
        let has_main = entry_module
            .and_then(|m| m.root_scope.lookup(STRING_INTERNER.intern("main")))
            .is_some();

        if !has_main {
            self.errors
                .push(CompilerErrorKind::MissingMainFunction(canonical_main));
        }

        if options.emit_hir {
            dump_program(&program, &self.types);
        }

        self.errors
            .extend(builder_errors.into_iter().map(CompilerErrorKind::Semantic));

        self.report_errors();
        if !self.errors.is_empty() {
            return;
        }

        // Codegen
        let mut codegen =
            CodeGenerator::new(&context, &program, &self.types, target_machine);

        if options.emit_llvm_ir {
            codegen.generate_ir();
            codegen.dump_ir(&options.output.with_extension("ll"));
            return;
        }

        codegen.generate_ir();
        let obj_path = options.object_path();
        codegen.emit_object_file(&obj_path);

        if options.emit_obj {
            return;
        }

        if options.emit_obj {
            return;
        }

        let mut linker = std::process::Command::new("cc");

        linker.arg(&obj_path);

        for foreign_file in &program.foreign_links {
            linker.arg(foreign_file);
        }

        for runtime_file in get_runtime_files() {
            linker.arg(runtime_file);
        }

        linker.arg("-o").arg(&options.output);

        let linker_status = linker.status();

        let _ = std::fs::remove_file(&obj_path);

        match linker_status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!("Linker failed with exit code: {}", status);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to invoke linker: {}", e);
                eprintln!("Make sure 'cc' is available in your PATH");
                std::process::exit(1);
            }
        }
    }

    pub fn parallel_parse_modules(
        &self,
        main_path: ModulePath,
    ) -> Vec<Result<ParallelParseResult, CompilerErrorKind>> {
        let visited = Arc::new(Mutex::new(HashSet::new()));
        let all_results = Arc::new(Mutex::new(Vec::new()));

        rayon::scope(|s| {
            fn parse_recursive(
                s: &rayon::Scope,
                path: ModulePath,
                files: Arc<Mutex<FileCache>>,
                visited: Arc<Mutex<HashSet<ModulePath>>>,
                all_results: Arc<
                    Mutex<Vec<Result<ParallelParseResult, CompilerErrorKind>>>,
                >,
            ) {
                {
                    let mut visited_guard = visited.lock().unwrap();
                    if !visited_guard.insert(path.clone()) {
                        return;
                    }
                }

                let source_code = match fs::read_to_string(path.0.as_path()) {
                    Ok(sc) => sc,
                    Err(e) => {
                        all_results.lock().unwrap().push(Err(
                            CompilerErrorKind::CouldNotReadFile {
                                path: path.clone(),
                                error: e,
                            },
                        ));
                        return;
                    }
                };

                let (tokens, tokenization_errors) =
                    Tokenizer::tokenize(&source_code, path.clone());
                let (statements, parsing_errors) = Parser::parse(tokens, path.clone());

                let (dependencies, dependency_errors, declarations) =
                    find_dependencies(path.clone(), &statements);

                for dep_path in dependencies {
                    let files = Arc::clone(&files);
                    let visited = Arc::clone(&visited);
                    let all_results = Arc::clone(&all_results);

                    s.spawn(move |s| {
                        parse_recursive(s, dep_path, files, visited, all_results);
                    });
                }

                files.lock().unwrap().insert(path.clone(), source_code);

                let mut results_guard = all_results.lock().unwrap();
                results_guard.extend(dependency_errors.into_iter().map(Err));
                results_guard.push(Ok(ParallelParseResult {
                    path,
                    statements,
                    declarations,
                    tokenization_errors,
                    parsing_errors,
                }));
            }

            parse_recursive(
                s,
                main_path,
                self.files.clone(),
                visited,
                all_results.clone(),
            );
        });

        Arc::try_unwrap(all_results)
            .expect("Arc unwrap failed")
            .into_inner()
            .expect("Mutex into_inner failed")
    }
}

fn find_dependencies(
    current_module_path: ModulePath,
    statements: &[Stmt],
) -> (
    HashSet<ModulePath>,
    Vec<CompilerErrorKind>,
    Vec<Declaration>,
) {
    let mut dependencies = HashSet::new();
    let mut errors = vec![];
    let mut declarations: Vec<Declaration> = vec![];

    for stmt in statements {
        match &stmt.kind {
            StmtKind::From { path, .. } => {
                let relative_path_str = &path.value;
                if relative_path_str == "std" || relative_path_str.starts_with("std") {
                    continue;
                }

                let mut target_path = current_module_path.0.to_path_buf();
                target_path.pop();
                target_path.push(relative_path_str);

                match fs::canonicalize(target_path.clone()) {
                    Ok(canonical_path) => {
                        let ext = canonical_path.extension().and_then(|e| e.to_str());
                        if let Some("wl") = ext {
                            dependencies.insert(ModulePath(Arc::new(canonical_path)));
                        }
                    }
                    Err(e) => {
                        errors.push(CompilerErrorKind::ModuleNotFound {
                            importing_module: current_module_path.clone(),
                            target_path: ModulePath(Arc::new(target_path)),
                            error: e,
                        });
                    }
                }
            }
            StmtKind::Expression(Expr {
                kind: ExprKind::Fn(decl),
                ..
            }) => {
                declarations.push(Declaration::Fn(*decl.clone()));
            }
            StmtKind::TypeAliasDecl(decl) => {
                declarations.push(Declaration::TypeAlias(decl.clone()));
            }
            _ => {}
        }
    }

    (dependencies, errors, declarations)
}
