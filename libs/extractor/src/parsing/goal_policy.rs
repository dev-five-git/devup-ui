use oxc_ast::ast::Statement;
use oxc_parser::ParserReturn;
use oxc_span::{GetSpan, LabeledSpan};

#[cfg(test)]
mod tests;

/// Recognize only Oxc 0.154's goal mismatch on a direct hybrid-CJS declaration.
pub(crate) fn accepts_declaration_goal(
    parsed: &ParserReturn<'_>,
    message: &str,
    labels: &[LabeledSpan],
) -> bool {
    if parsed.fatal_error
        || !parsed.program.source_type.is_javascript()
        || !parsed.program.source_type.is_commonjs()
    {
        return false;
    }
    let keyword = match message {
        "Cannot use import statement outside a module" => "import",
        "Cannot use export statement outside a module" => "export",
        _ => return false,
    };
    let [label] = labels else {
        return false;
    };
    if label.primary() || label.label().is_some() || label.len() != 6 {
        return false;
    }
    parsed.program.body.iter().any(|statement| {
        let declaration_keyword = match statement {
            Statement::ImportDeclaration(_) => "import",
            Statement::ExportDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::ExportFromDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::ExportAllDeclaration(_) => "export",
            _ => return false,
        };
        let start = statement.span().start;
        declaration_keyword == keyword
            && label.offset() == start
            && parsed.program.source_text.as_bytes().get(
                usize::try_from(start).unwrap_or(usize::MAX)
                    ..usize::try_from(label.span().end).unwrap_or(usize::MAX),
            ) == Some(keyword.as_bytes())
    })
}

/// Check without modifying stock diagnostics, AST nodes or source-type metadata.
pub(crate) fn has_diagnostics(parsed: &ParserReturn<'_>) -> bool {
    parsed.fatal_error
        || parsed.diagnostics.iter().any(|diagnostic| {
            !accepts_declaration_goal(parsed, &diagnostic.to_string(), &diagnostic.labels)
        })
}
