use super::{Allocator, AstKind, Parser, SemanticBuilder, SourceType, identifier_names};

/// Put the written expression back into an error message that names a
/// generated import; names the program itself binds or reads stay as written.
pub(crate) fn original_error_code(
    source: &str,
    offset: usize,
    mut message: String,
    source_type: SourceType,
) -> String {
    if !message.contains("__emotion_") {
        return message;
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return format!(
            "{message}\nEmotion diagnostic restoration failed: {:?}",
            parsed.diagnostics
        );
    }
    let program = parsed.program;
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    let owned = identifier_names(&semantic);
    let original = semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::StaticMemberExpression(member) => Some(member.span),
            AstKind::ComputedMemberExpression(member) => Some(member.span),
            AstKind::IdentifierReference(identifier) => Some(identifier.span),
            AstKind::ParenthesizedExpression(expression) => Some(expression.span),
            AstKind::TSAsExpression(expression) => Some(expression.span),
            AstKind::TSSatisfiesExpression(expression) => Some(expression.span),
            AstKind::TSNonNullExpression(expression) => Some(expression.span),
            AstKind::TSTypeAssertion(expression) => Some(expression.span),
            AstKind::TSInstantiationExpression(expression) => Some(expression.span),
            _ => None,
        })
        .filter(|span| span.start as usize == offset)
        .max_by_key(|span| span.end);
    let markers = message
        .find("cannot use `")
        .map(|start| start + "cannot use `".len())
        .zip(message.rfind("` at build time:"));
    if let Some(span) = original
        && let Some((start, end)) = markers
    {
        let original = &source[span.start as usize..span.end as usize];
        let code = format!("({})", &message[start..end]);
        // The diagnostic expression is emitted by readable_code from the parsed AST.
        let parsed = Parser::new(&allocator, &code, source_type).parse();
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(&parsed.program)
            .semantic;
        let mut references = semantic
            .nodes()
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::IdentifierReference(identifier)
                    if identifier.name.starts_with("__emotion_")
                        && !owned.contains(identifier.name.as_str()) =>
                {
                    Some(identifier.span)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        references.sort_unstable_by_key(|span| std::cmp::Reverse(span.start));
        for span in references {
            message.replace_range(
                start + span.start as usize - 1..start + span.end as usize - 1,
                original,
            );
        }
    }
    message
}
