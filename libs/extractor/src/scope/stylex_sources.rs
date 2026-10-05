use oxc_ast::ast::{CallExpression, Expression, Program, Statement};
use oxc_ast_visit::{Visit, walk};

/// Exact public entrypoints; a type augmentation is not an API module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StylexSource {
    Root,
    Upstream,
    Dedicated,
    TypesOnly,
    Other,
}

impl StylexSource {
    pub(crate) fn classify(source: &str, package: &str) -> Self {
        if source == "@stylexjs/stylex" {
            Self::Upstream
        } else if source == package {
            Self::Root
        } else {
            match source.strip_prefix(package) {
                Some("/stylex") => Self::Dedicated,
                Some("/compat/stylex") => Self::TypesOnly,
                _ => Self::Other,
            }
        }
    }

    pub(crate) const fn is_api_module(self) -> bool {
        matches!(self, Self::Upstream | Self::Dedicated)
    }
}

/// The literal first source operand; callers prove loader identity and arity.
pub(crate) fn require_source<'e>(call: &'e CallExpression<'_>) -> Option<&'e str> {
    match crate::utils::unwrap_syntax_only(&call.callee) {
        Expression::Identifier(callee) if callee.name == "require" => {
            match crate::utils::unwrap_syntax_only(call.arguments.first()?.as_expression()?) {
                Expression::StringLiteral(source) => Some(source.value.as_str()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A conservative AST gate, including invalid value imports that extraction diagnoses.
pub(crate) fn has_stylex_source(program: &Program<'_>, package: &str) -> bool {
    let imported = program.body.iter().any(|statement| {
        matches!(statement, Statement::ImportDeclaration(import)
            if !import.import_kind.is_type()
                && StylexSource::classify(import.source.value.as_str(), package) != StylexSource::Other
                && import.specifiers.as_ref().is_none_or(|specifiers| specifiers.is_empty()
                    || specifiers.iter().any(|specifier| match specifier {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) => !named.import_kind.is_type(),
                        _ => true,
                    })))
            || matches!(statement, Statement::ExportFromDeclaration(export)
                if !export.export_kind.is_type()
                    && StylexSource::classify(export.source.value.as_str(), package) != StylexSource::Other
                    && export.specifiers.iter().any(|specifier| !specifier.export_kind.is_type()))
            || matches!(statement, Statement::ExportAllDeclaration(export)
                if !export.export_kind.is_type()
                    && StylexSource::classify(export.source.value.as_str(), package) != StylexSource::Other)
    });
    imported || has_stylex_require(program, package)
}

pub(crate) fn has_stylex_require(program: &Program<'_>, package: &str) -> bool {
    struct Requires<'p> {
        package: &'p str,
        found: bool,
    }
    impl<'a> Visit<'a> for Requires<'_> {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            self.found |= require_source(call).is_some_and(|source| {
                StylexSource::classify(source, self.package) != StylexSource::Other
            });
            walk::walk_call_expression(self, call);
        }
    }
    let mut requires = Requires {
        package,
        found: false,
    };
    requires.visit_program(program);
    requires.found
}

#[cfg(test)]
mod tests;
