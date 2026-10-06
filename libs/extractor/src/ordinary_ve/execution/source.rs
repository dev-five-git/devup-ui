use std::collections::BTreeSet;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Statement, VariableDeclarationKind},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};

use super::{SelectedModule, Selection, Stylesheet, imports, observe::Observations, policy};
use crate::module_loader::Mapped;
use crate::ordinary_ve::selection::plan::{Binding, UnitKind};
use crate::vanilla_extract::{
    capture::{Capture, Observation, ObservationKind},
    json_string,
};

pub(super) struct Source {
    pub mapped: Mapped,
    pub captures: Vec<Capture>,
    pub identities: Vec<(Span, Option<Binding>)>,
    pub reserved: BTreeSet<String>,
    pub observations: Observations,
}

fn fresh(reserved: &mut BTreeSet<String>, index: usize) -> String {
    let mut name = format!("__ve_capture_{index}__");
    while reserved.contains(&name) {
        name.push('_');
    }
    reserved.insert(name.clone());
    name
}

pub(super) fn build(
    stylesheet: Stylesheet<'_>,
    selection: &Selection,
    option: &crate::ExtractOption,
) -> Result<Source, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        stylesheet.code,
        SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    policy::writes(stylesheet, selection, &semantic)?;
    let mut reserved = selection.reserved_names.clone();
    let observations = Observations::new(&mut reserved);
    let mut source = Source {
        mapped: Mapped::default(),
        captures: Vec::new(),
        identities: Vec::new(),
        reserved,
        observations,
    };
    imports::write(
        SelectedModule {
            stylesheet,
            selection,
        },
        option,
        &mut source.mapped,
    )?;
    for import in selection
        .imports
        .iter()
        .filter(|import| import.native.is_none())
    {
        source.observations.write(
            &mut source.mapped,
            Observation {
                span: import.declaration,
                place: policy::place(stylesheet, import.specifier.start),
                kind: ObservationKind::Input { declarator: false },
                reads: vec![import.binding.name.clone()],
                mutations: Vec::new(),
            },
        );
    }
    let expression = |span: Span, code: String| {
        let shorthand = semantic.nodes().iter().any(|node| matches!(node.kind(),
            AstKind::ObjectProperty(property) if property.shorthand && property.value.span() == span));
        let prefix = if shorthand {
            format!("{}: ", span.source_text(stylesheet.code))
        } else {
            String::new()
        };
        (span, format!("{prefix}{code}"))
    };
    let mut checks: Vec<_> = selection
        .checks
        .iter()
        .map(|escape| {
            let message = format!(
                "{}: {}. Fix: {}",
                policy::place(stylesheet, escape.span.start),
                escape.cause(),
                escape.fix()
            );
            expression(
                escape.span,
                format!("(function(){{throw {};}})()", json_string(&message)),
            )
        })
        .collect::<Vec<_>>();
    // Boa can attribute a bound TDZ read to its enclosing property. An immediate
    // thunk gives that read its own frame, anchored through the existing trace.
    for read in selection.reads.iter().filter(|read| !read.write) {
        let forward = read.symbol.is_some_and(|symbol| {
            selection.units.iter().any(|unit| {
                matches!(
                    unit.kind,
                    UnitKind::Declarator {
                        kind: VariableDeclarationKind::Const | VariableDeclarationKind::Let,
                        ..
                    }
                ) && unit
                    .bindings
                    .iter()
                    .any(|binding| binding.symbol == symbol && binding.span.start > read.span.start)
            })
        });
        if forward && read.name != "eval" && !checks.iter().any(|(span, _)| *span == read.span) {
            checks.push(expression(read.span, format!("(()=>{})()", read.name)));
        }
    }
    checks.sort_by_key(|(span, _)| span.start);
    let copy = |mapped: &mut Mapped, span: Span| {
        let mut start = span.start;
        for (check, replacement) in &checks {
            if check.start >= start && check.end <= span.end {
                mapped.copy(stylesheet.code, Span::new(start, check.start));
                mapped.synthesize(check.start, replacement);
                start = check.end;
            }
        }
        mapped.copy(stylesheet.code, Span::new(start, span.end));
    };
    for unit in &selection.units {
        let root = selection.roots.iter().find(|root| root.owner == unit.node);
        let capture_start = source.captures.len();
        if root.is_some() {
            source.observations.write(
                &mut source.mapped,
                super::observe::before(
                    SelectedModule {
                        stylesheet,
                        selection,
                    },
                    unit,
                ),
            );
        }
        match &unit.kind {
            UnitKind::Declarator { kind, .. } => {
                let keyword = match kind {
                    VariableDeclarationKind::Const => "const",
                    VariableDeclarationKind::Let => "let",
                    VariableDeclarationKind::Var => "var",
                    VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => {
                        return Err(format!(
                            "{}: resource declarations cannot be captured exactly. Fix: use a plain exact initializer",
                            policy::place(stylesheet, unit.span.start)
                        ));
                    }
                };
                source
                    .mapped
                    .synthesize(unit.span.start, &format!("{keyword} "));
                copy(&mut source.mapped, unit.span);
                source.mapped.synthesize(unit.span.end, ";\n");
            }
            UnitKind::Function { .. } | UnitKind::Statement => {
                let expression = root.filter(|root| root.captures.is_empty()).and_then(|_| {
                    parsed
                        .program
                        .body
                        .iter()
                        .find_map(|statement| match statement {
                            Statement::ExpressionStatement(statement)
                                if statement.span == unit.span =>
                            {
                                Some(statement.expression.span())
                            }
                            Statement::ExportDefaultDeclaration(export)
                                if export.declaration.span() == unit.span =>
                            {
                                Some(unit.span)
                            }
                            _ => None,
                        })
                });
                if let Some(expression) = expression {
                    let name = fresh(&mut source.reserved, source.captures.len());
                    source
                        .mapped
                        .synthesize(expression.start, &format!("const {name} = ("));
                    copy(&mut source.mapped, expression);
                    source.mapped.synthesize(expression.end, ");\n");
                    source.captures.push(Capture {
                        name: name.clone(),
                        read: name,
                        place: policy::place(stylesheet, expression.start),
                        root: unit.span,
                    });
                    source.identities.push((unit.span, None));
                } else {
                    copy(&mut source.mapped, unit.span);
                    source.mapped.synthesize(unit.span.end, "\n");
                }
            }
        }
        if let Some(root) = root {
            for binding in &root.captures {
                let name = fresh(&mut source.reserved, source.captures.len());
                source.captures.push(Capture {
                    name,
                    read: binding.name.clone(),
                    place: policy::place(stylesheet, binding.span.start),
                    root: root.span,
                });
                source.identities.push((root.span, Some(binding.clone())));
            }
        }
        if let Some(site) = super::observe::after(
            SelectedModule {
                stylesheet,
                selection,
            },
            unit,
            &source.captures[capture_start..],
        ) {
            source.observations.write(&mut source.mapped, site);
        }
    }
    Ok(source)
}
