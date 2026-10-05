use oxc_ast::ast::{
    BindingPattern, Expression, ImportDeclarationSpecifier, Program, Statement, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;

use super::{StylexBinding, StylexBindings, symbol, unbound_reference, unchanged};
use crate::scope::stylex_sources::{StylexSource, require_source};
use crate::stylex::StylexFunction;
use crate::utils::{get_string_by_property_key, unwrap_syntax_only};

pub(super) fn exported(source: StylexSource, name: &str) -> Option<StylexBinding> {
    match source {
        StylexSource::Root => match name {
            "stylex" => Some(StylexBinding::Namespace),
            "default" => Some(StylexBinding::RootDefault),
            name if name != "keyframes" && StylexFunction::from_export_name(name).is_some() => {
                Some(StylexBinding::Invalid)
            }
            _ => None,
        },
        StylexSource::Upstream if name == "default" => Some(StylexBinding::Namespace),
        StylexSource::Upstream | StylexSource::Dedicated => Some(
            StylexFunction::from_export_name(name)
                .map_or(StylexBinding::Invalid, StylexBinding::Function),
        ),
        StylexSource::TypesOnly => Some(StylexBinding::Invalid),
        StylexSource::Other => None,
    }
}

pub(super) const fn namespace(source: StylexSource) -> Option<StylexBinding> {
    match source {
        StylexSource::Root => Some(StylexBinding::Root),
        StylexSource::Upstream => Some(StylexBinding::UpstreamNamespace),
        StylexSource::Dedicated => Some(StylexBinding::Namespace),
        StylexSource::TypesOnly => Some(StylexBinding::Invalid),
        StylexSource::Other => None,
    }
}

pub(super) fn bindings(program: &Program<'_>, scoping: &Scoping, package: &str) -> StylexBindings {
    let mut bindings = StylexBindings::default();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            if import.import_kind.is_type() {
                continue;
            }
            let source = StylexSource::classify(import.source.value.as_str(), package);
            for specifier in import.specifiers.iter().flatten() {
                let binding = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named)
                        if named.import_kind.is_type() =>
                    {
                        None
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        exported(source, &named.imported.name())
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => namespace(source),
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        exported(source, "default")
                    }
                };
                if let Some(binding) = binding {
                    bindings.insert(specifier.local().symbol_id.get(), binding);
                }
            }
        }
    }
    let mut collector = Collector {
        bindings,
        scoping,
        package,
    };
    collector.visit_program(program);
    collector.bindings
}

struct Collector<'s> {
    bindings: StylexBindings,
    scoping: &'s Scoping,
    package: &'s str,
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(init) = &declarator.init {
            let init = unwrap_syntax_only(init);
            let source = match init {
                Expression::CallExpression(call) => require_source(call)
                    .filter(|_| {
                        matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader)
                        if unbound_reference(self.scoping, loader))
                    })
                    .map(|source| StylexSource::classify(source, self.package)),
                _ => None,
            };
            match (&declarator.id, source) {
                (BindingPattern::BindingIdentifier(local), Some(source)) => {
                    if let Some(binding) = namespace(source) {
                        self.bindings.insert(local.symbol_id.get(), binding);
                    }
                }
                (BindingPattern::ObjectPattern(object), Some(source)) => {
                    for property in &object.properties {
                        if let (Some(name), Some(local)) = (
                            get_string_by_property_key(&property.key),
                            property.value.get_binding_identifier(),
                        ) && let Some(binding) = exported(source, &name)
                        {
                            self.bindings.insert(local.symbol_id.get(), binding);
                        }
                    }
                }
                (BindingPattern::BindingIdentifier(local), None) => {
                    if let Some(to) = local.symbol_id.get()
                        && unchanged(self.scoping, to)
                        && let Some(binding) =
                            self.bindings.resolve(init, &|id| symbol(self.scoping, id))
                    {
                        self.bindings.insert(Some(to), binding);
                    }
                }
                _ => {}
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }
}
