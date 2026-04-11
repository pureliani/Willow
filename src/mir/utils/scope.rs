use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::{Rc, Weak},
};

use crate::{
    ast::{Position, Span, SymbolId},
    compile::interner::StringId,
    mir::builders::{BasicBlockId, LoopJumpTargets},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ScopeKind {
    FunctionBody,
    WhileBody {
        break_target: BasicBlockId,
        continue_target: BasicBlockId,
    },
    CodeBlock,
    File,
    Global,
}

#[derive(Debug)]
struct ScopeData {
    kind: ScopeKind,
    symbols: HashMap<StringId, SymbolId>,
    parent: Option<Weak<RefCell<ScopeData>>>,
    children: Vec<Scope>,
    span: Span,
}

#[derive(Debug, Clone)]
pub struct Scope(Rc<RefCell<ScopeData>>);

impl Scope {
    pub fn new_root(kind: ScopeKind, span: Span) -> Self {
        Self(Rc::new(RefCell::new(ScopeData {
            kind,
            symbols: HashMap::new(),
            parent: None,
            children: Vec::new(),
            span,
        })))
    }

    pub fn enter(&self, kind: ScopeKind, start_position: Position) -> Scope {
        let child = Scope(Rc::new(RefCell::new(ScopeData {
            kind,
            symbols: HashMap::new(),
            parent: Some(Rc::downgrade(&self.0)),
            span: Span {
                start: start_position,
                end: Position::default(),
                path: self.0.borrow().span.path.clone(),
            },
            children: Vec::new(),
        })));

        self.0.borrow_mut().children.push(child.clone());

        child
    }

    pub fn exit(&self, exit_position: Position) -> Option<Scope> {
        {
            let mut data = self.0.borrow_mut();
            data.span.end = exit_position;
        }
        self.parent()
    }

    pub fn parent(&self) -> Option<Scope> {
        self.0.borrow().parent.as_ref()?.upgrade().map(Scope)
    }

    pub fn lookup(&self, name: StringId) -> Option<SymbolId> {
        if let Some(id) = self.0.borrow().symbols.get(&name) {
            return Some(*id);
        }

        self.parent()?.lookup(name)
    }

    pub fn map_name_to_symbol(&self, name: StringId, id: SymbolId) {
        let mut data = self.0.borrow_mut();
        if let Entry::Vacant(e) = data.symbols.entry(name) {
            e.insert(id);
        } else {
            panic!(
                "INTERNAL COMPILER ERROR: tried to re-map identifier to declaration, \
                 this should have been avoided at declaration site"
            );
        }
    }

    pub fn within_function_body(&self) -> bool {
        let kind = self.0.borrow().kind;

        if let ScopeKind::FunctionBody = kind {
            return true;
        }

        if matches!(kind, ScopeKind::CodeBlock | ScopeKind::WhileBody { .. }) {
            return self
                .parent()
                .is_some_and(|parent| parent.within_function_body());
        }

        false
    }

    pub fn within_loop_body(&self) -> Option<LoopJumpTargets> {
        let kind = self.0.borrow().kind;
        if let ScopeKind::WhileBody {
            break_target,
            continue_target,
        } = kind
        {
            return Some(LoopJumpTargets {
                on_break: break_target,
                on_continue: continue_target,
            });
        }

        if matches!(kind, ScopeKind::CodeBlock) {
            return self.parent().and_then(|parent| parent.within_loop_body());
        }

        None
    }

    pub fn is_file_scope(&self) -> bool {
        matches!(self.kind(), ScopeKind::File)
    }

    pub fn kind(&self) -> ScopeKind {
        self.0.borrow().kind
    }

    pub fn find_innermost_at(&self, byte_offset: usize) -> Option<Scope> {
        let data = self.0.borrow();

        if !data.span.contains(byte_offset) {
            return None;
        }

        for child in &data.children {
            if let Some(inner) = child.find_innermost_at(byte_offset) {
                return Some(inner);
            }
        }

        drop(data);
        Some(self.clone())
    }
}
