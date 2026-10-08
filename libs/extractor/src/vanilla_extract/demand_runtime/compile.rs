use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use super::Owner;
use crate::vanilla_extract::{CollectedStyles, Reference, StylesheetImports};

pub(super) struct Materialized {
    pub atoms: crate::vanilla_extract::producer_atoms::ProducerAtoms,
    pub references: crate::vanilla_extract::style_references::StyleReferences,
    pub values: FxHashMap<String, String>,
    pub artifacts: crate::graph::Artifacts,
}

impl Owner {
    pub(super) fn compile(
        &self,
        collected: &CollectedStyles,
        imports: StylesheetImports,
    ) -> Result<Materialized, String> {
        let option = &self.option;
        let stylesheet = crate::vanilla_extract::Stylesheet {
            filename: &self.plan.filename,
            code: &self.plan.source,
            source: &self.plan.source,
            edits: &[],
        };
        let lowered = crate::ordinary_ve::lowering::lower(stylesheet, collected, option)?;
        let references = collected
            .references
            .values()
            .filter_map(|reference| match reference {
                Reference::Style { name, class_name } => Some((name.clone(), class_name.clone())),
                Reference::Keyframes(_) => None,
            })
            .collect();
        let mut class_references = imports.references.clone();
        class_references.merge(collected.class_references.clone());
        let prepared = crate::ordinary_ve::Prepared {
            native: true,
            readback: false,
            code: lowered.code,
            edits: Vec::new(),
            imports,
            references: class_references,
            bindings: references,
            css: lowered.css,
        };
        let result = crate::extract_source(
            &self.plan.filename,
            &prepared.code,
            Some(crate::Evaluated {
                source: &self.plan.source,
                edits: &[],
                native: Some(&prepared),
                readback: None,
            }),
            true,
            option.clone(),
            false,
            None,
        )
        .map_err(|error| error.to_string())?;
        let allocator = Allocator::default();
        let parsed = Parser::new(
            &allocator,
            &result.output.code,
            SourceType::from_path(&self.plan.filename).unwrap_or_default(),
        )
        .parse();
        let mut bindings = FxHashMap::default();
        for statement in &parsed.program.body {
            let declarations = match statement {
                Statement::VariableDeclaration(declaration) => Some(&declaration.declarations),
                Statement::ExportDeclaration(export) => match &export.declaration {
                    Declaration::VariableDeclaration(declaration) => {
                        Some(&declaration.declarations)
                    }
                    _ => None,
                },
                _ => None,
            };
            for declaration in declarations.into_iter().flatten() {
                if let Some(identifier) = declaration.id.get_binding_identifier()
                    && let Some(Expression::StringLiteral(value)) = &declaration.init
                {
                    bindings.insert(identifier.name.to_string(), value.value.to_string());
                }
            }
        }
        let mut atoms = result.atoms;
        let mut references = result.references;
        let mut values = FxHashMap::default();
        for (placeholder, reference) in &collected.references {
            let name = match reference {
                Reference::Style { name, .. } | Reference::Keyframes(name) => name,
            };
            let value = bindings.get(name).ok_or_else(|| format!("{}:1:1: native result `{name}` has no final emitted value. Fix: report this extraction error", self.plan.filename))?;
            if let Reference::Style { class_name, .. } = reference {
                atoms.alias(placeholder.clone(), value);
                references.register(placeholder.clone(), class_name.clone());
            }
            values.insert(placeholder.clone(), value.clone());
        }
        let mut artifacts = result.artifacts;
        artifacts.insert(std::rc::Rc::new(crate::StylesheetArtifact {
            filename: self.plan.filename.clone(),
            output: result.output,
        }))?;
        Ok(Materialized {
            atoms,
            references,
            values,
            artifacts,
        })
    }
}
