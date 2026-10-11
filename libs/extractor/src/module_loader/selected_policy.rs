use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub(super) fn check(module: &crate::ResolvedModule) -> Result<(), String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        &module.code,
        SourceType::from_path(&module.path).unwrap_or_default(),
    )
    .parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let selected = crate::ordinary_ve::selection::select(&parsed.program, &semantic);
    if !crate::ordinary_ve::mixed(&parsed.program, &semantic, &selected)
        && (crate::ordinary_ve::is_module(&module.path, &module.code)
            || crate::utils::is_vanilla_extract_file(&module.path))
    {
        return Ok(());
    }
    let site = selected.roots.first().map(|root| root.span).or_else(|| {
        selected
            .imports
            .iter()
            .find(|import| import.native.is_some())
            .map(|import| import.specifier)
    });
    if let Some(site) = site {
        return Err(format!(
            "{}: an imported mixed native producer requires selected-view integration. Fix: move the producer to a stylesheet or static-data module",
            crate::locate(
                &module.path,
                &module.code,
                usize::try_from(site.start).unwrap_or(module.code.len())
            ),
        ));
    }
    Ok(())
}
