use crate::{barrel::native::Shape, vanilla_extract::Stylesheet};
use oxc_ast::ast::{Program, Statement};

pub(super) const DIRECT_IMPORT: &str =
    "import the API directly from @vanilla-extract/css in the consuming stylesheet";

pub(super) fn check(
    stylesheet: Stylesheet<'_>,
    program: &Program<'_>,
    resolver: Option<&crate::ModuleResolver>,
) -> Result<(), String> {
    let pure: Vec<_> = program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ExportFromDeclaration(export) if !export.export_kind.is_type() => {
                Some(export.span)
            }
            Statement::ExportAllDeclaration(export) if !export.export_kind.is_type() => {
                Some(export.span)
            }
            _ => None,
        })
        .collect();
    if pure.is_empty() {
        return Ok(());
    }
    let terminals = crate::barrel::native::exported_terminals(
        (program, stylesheet.code),
        (stylesheet.filename, "@vanilla-extract/css"),
        resolver,
    );
    for (site, shape) in &terminals {
        if pure.iter().any(|span| span.contains_inclusive(*site))
            && let Some(failure) = failure(shape)
        {
            return Err(match failure {
                Failure::Original(message) => message.to_string(),
                Failure::Local(message) => format!(
                    "{}: {message}. Fix: use an unambiguous immutable native API binding and {DIRECT_IMPORT}",
                    super::execution::policy::place(stylesheet, site.start)
                ),
            });
        }
    }
    for (site, _) in terminals {
        if pure.iter().any(|span| span.contains_inclusive(site)) {
            return Err(format!(
                "{}: a native styling API escapes its exact initialization slice. Fix: call the API in an exact initializer instead of exporting or handing off the API; {DIRECT_IMPORT}",
                super::execution::policy::place(stylesheet, site.start)
            ));
        }
    }
    Ok(())
}

enum Failure<'a> {
    Original(&'a str),
    Local(&'a str),
}

fn failure(shape: &Shape) -> Option<Failure<'_>> {
    match shape {
        Shape::OriginalFailure(message) => Some(Failure::Original(message)),
        Shape::Failed(message) => Some(Failure::Local(message)),
        Shape::Namespace(members) | Shape::PackageNamespace(members) => {
            members.values().find_map(|member| failure(member))
        }
        Shape::Api(_) => None,
    }
}
