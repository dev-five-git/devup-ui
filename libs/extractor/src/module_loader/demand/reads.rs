use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

pub(super) fn rewrite(
    semantic: &Semantic<'_>,
    producer: &super::Producer,
) -> Vec<(u32, u32, String)> {
    let mut replacements = Vec::new();
    for binding in &producer.bindings {
        let Some(symbol) = semantic
            .scoping()
            .get_root_binding(binding.name.as_str().into())
        else {
            continue;
        };
        for reference in semantic.scoping().get_resolved_reference_ids(symbol) {
            let reference = semantic.scoping().get_reference(*reference);
            if !reference.is_read() || reference.is_write() {
                continue;
            }
            let node = reference.node_id();
            let span = semantic.nodes().kind(node).span();
            if matches!(
                semantic.nodes().parent_kind(node),
                AstKind::ExportSpecifier(_)
            ) {
                continue;
            }
            let read = format!("{}$read({})", producer.namespace, binding.name);
            let code = match semantic.nodes().parent_kind(node) {
                AstKind::ObjectProperty(property) if property.shorthand => {
                    format!("{}: {read}", binding.name)
                }
                _ => read,
            };
            replacements.push((span.start, span.end, code));
        }
    }
    replacements
}
