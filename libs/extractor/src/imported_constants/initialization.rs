use oxc_ast::ast::{
    ArrowFunctionExpression, CallExpression, Class, ClassElement, Expression, Function,
    IdentifierReference, Program, VariableDeclaration,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::{reference::ReferenceId, scope::ScopeFlags, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy)]
enum Read {
    Initialized,
    Deferred,
    Before {
        temporal: bool,
        at: u32,
        symbol: SymbolId,
    },
}

pub(super) struct Initialization {
    reads: FxHashMap<ReferenceId, Read>,
}

impl Initialization {
    pub(super) fn new(program: &Program<'_>, scoping: &Scoping) -> Self {
        let mut collector = Collector {
            scoping,
            bindings: FxHashMap::default(),
            references: Vec::new(),
            frames: vec![Frame {
                parent: 0,
                deferred: false,
            }],
            frame: 0,
            invoked: FxHashSet::default(),
        };
        collector.visit_program(program);
        let reads = collector
            .references
            .iter()
            .map(|&(reference, symbol, at, frame)| {
                let read = collector
                    .bindings
                    .get(&symbol)
                    .map_or(Read::Initialized, |binding| {
                        if collector.is_deferred(frame, binding.frame) {
                            Read::Deferred
                        } else if at < binding.initialized
                            && !binding
                                .class_initializers
                                .iter()
                                .any(|span| span.start <= at && at < span.end)
                        {
                            Read::Before {
                                temporal: binding.temporal,
                                at,
                                symbol,
                            }
                        } else {
                            Read::Initialized
                        }
                    });
                (reference, read)
            })
            .collect();
        Self { reads }
    }

    pub(super) fn allows(&self, identifier: &IdentifierReference<'_>) -> bool {
        match identifier
            .reference_id
            .get()
            .and_then(|reference| self.reads.get(&reference))
        {
            Some(Read::Before { .. }) => false,
            Some(Read::Initialized | Read::Deferred) | None => true,
        }
    }

    pub(super) fn errors(
        &self,
        references: &FxHashSet<ReferenceId>,
        scoping: &Scoping,
    ) -> Vec<(u32, String)> {
        let mut errors: Vec<_> = references.iter().filter_map(|reference| {
            let Read::Before { temporal: true, at, symbol } = self.reads.get(reference)? else {
                return None;
            };
            let name = scoping.symbol_name(*symbol);
            Some((*at, format!("style value cannot use `{name}` at build time: it is read before its lexical declaration is initialized, so the original JavaScript throws ReferenceError; move the declaration of `{name}` above this use")))
        }).collect();
        errors.sort_unstable_by_key(|(at, _)| *at);
        errors
    }
}

struct Binding {
    initialized: u32,
    class_initializers: Vec<Span>,
    frame: usize,
    temporal: bool,
}

struct Frame {
    parent: usize,
    deferred: bool,
}

struct Collector<'s> {
    scoping: &'s Scoping,
    bindings: FxHashMap<SymbolId, Binding>,
    references: Vec<(ReferenceId, SymbolId, u32, usize)>,
    frames: Vec<Frame>,
    frame: usize,
    invoked: FxHashSet<u32>,
}

impl Collector<'_> {
    fn is_deferred(&self, mut from: usize, declaration: usize) -> bool {
        while from != declaration && from != 0 {
            let frame = &self.frames[from];
            if frame.deferred {
                return true;
            }
            from = frame.parent;
        }
        false
    }

    fn function(&mut self, start: u32, visit: impl FnOnce(&mut Self)) {
        let outer = self.frame;
        self.frame = self.frames.len();
        self.frames.push(Frame {
            parent: outer,
            deferred: !self.invoked.contains(&start),
        });
        visit(self);
        self.frame = outer;
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            for identifier in declarator.id.get_binding_identifiers() {
                if let Some(symbol) = identifier.symbol_id.get() {
                    self.bindings.insert(
                        symbol,
                        Binding {
                            initialized: declarator.span.end,
                            class_initializers: Vec::new(),
                            frame: self.frame,
                            temporal: declaration.kind.is_lexical(),
                        },
                    );
                }
            }
        }
        walk::walk_variable_declaration(self, declaration);
    }

    fn visit_class(&mut self, class: &Class<'a>) {
        if let Some(symbol) = class.id.as_ref().and_then(|id| id.symbol_id.get()) {
            self.bindings.insert(
                symbol,
                Binding {
                    initialized: class.span.end,
                    // Class self-bindings initialize after keys/extends, before static values.
                    class_initializers: class
                        .body
                        .body
                        .iter()
                        .filter_map(|element| match element {
                            ClassElement::PropertyDefinition(property) if property.r#static => {
                                property.value.as_ref().map(GetSpan::span)
                            }
                            ClassElement::StaticBlock(block) => Some(block.span),
                            _ => None,
                        })
                        .collect(),
                    frame: self.frame,
                    temporal: true,
                },
            );
        }
        walk::walk_class(self, class);
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some(reference) = identifier.reference_id.get()
            && self.scoping.get_reference(reference).is_read()
            && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
        {
            self.references
                .push((reference, symbol, identifier.span.start, self.frame));
        }
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        self.function(function.span.start, |collector| {
            walk::walk_function(collector, function, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.function(arrow.span.start, |collector| {
            walk::walk_arrow_function_expression(collector, arrow);
        });
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        match crate::utils::unwrap_syntax_only(&call.callee) {
            Expression::FunctionExpression(function) => {
                self.invoked.insert(function.span.start);
            }
            Expression::ArrowFunctionExpression(arrow) => {
                self.invoked.insert(arrow.span.start);
            }
            _ => {}
        }
        walk::walk_call_expression(self, call);
    }
}
