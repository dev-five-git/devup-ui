use oxc_allocator::{GetAllocator, TakeIn, Vec};
use oxc_ast::ast::{
    BindingIdentifier, Declaration, ExportSpecifier, IdentifierName, IdentifierReference,
    ImportOrExportKind, ModuleExportName, Program, Statement,
};

/// Separate renamed declarations from their public names with live export
/// specifiers, moving the original AST to retain semantic IDs and source spans.
pub(super) fn preserve<'a>(program: &mut Program<'a>, rename: &super::Rename<'_, 'a>) {
    let mut body = Vec::with_capacity_in(program.body.len(), rename.builder);
    for statement in program.body.take_in(rename.builder) {
        let Statement::ExportDeclaration(mut export) = statement else {
            body.push(statement);
            continue;
        };
        let bindings: std::vec::Vec<&BindingIdentifier<'a>> = match &export.declaration {
            Declaration::VariableDeclaration(variable) => variable
                .declarations
                .iter()
                .flat_map(|declarator| declarator.id.get_binding_identifiers())
                .collect(),
            declaration => declaration.id().into_iter().collect(),
        };
        if !bindings.iter().any(|binding| {
            binding
                .symbol_id
                .get()
                .is_some_and(|symbol| rename.aliases.contains_key(&symbol))
        }) {
            body.push(Statement::ExportDeclaration(export));
            continue;
        }
        let specifiers = Vec::from_iter_in(
            bindings.iter().map(|binding| {
                let local = binding
                    .symbol_id
                    .get()
                    .and_then(|symbol| rename.aliases.get(&symbol))
                    .map_or(binding.name.as_str(), String::as_str);
                ExportSpecifier::new(
                    binding.span,
                    ModuleExportName::IdentifierReference(IdentifierReference::new(
                        binding.span,
                        rename.builder.allocator().alloc_str(local),
                        rename.builder,
                    )),
                    ModuleExportName::IdentifierName(IdentifierName::new(
                        binding.span,
                        binding.name,
                        rename.builder,
                    )),
                    ImportOrExportKind::Value,
                    rename.builder,
                )
            }),
            rename.builder,
        );
        body.push(Statement::from(export.declaration.take_in(rename.builder)));
        body.push(Statement::new_export_named_declaration(
            export.span,
            specifiers,
            ImportOrExportKind::Value,
            rename.builder,
        ));
    }
    program.body = body;
}
