use crate::{
    module_loader::{ModuleLoader, ModuleScript, Unit, module_script},
    stylesheet_compile::Compiled,
    vanilla_extract::{Stylesheet, StylesheetImports, producer_atoms::ProducerAtoms},
};

pub(super) fn entry(
    stylesheet: Stylesheet<'_>,
    compiled: &Compiled,
    loader: &mut ModuleLoader<'_>,
) -> Result<Option<ModuleScript>, String> {
    let unit = if let Some(evaluation) = &compiled.evaluation {
        Unit::deferred(stylesheet, evaluation)?
    } else if let Some(code) = &compiled.code {
        let layers: Vec<_> = std::iter::once(compiled.edits.as_slice())
            .chain(stylesheet.edits.iter().copied())
            .collect();
        Unit::written(stylesheet.filename, code, stylesheet.source, &layers)?
    } else {
        return Ok(None);
    };
    module_script(&unit, loader, true).map(Some)
}

pub(super) fn seed(compiled: &Compiled, imports: &mut StylesheetImports, scope: Option<&str>) {
    let atoms = ProducerAtoms::from_styles(&compiled.styles, scope);
    for style in &compiled.styles {
        if let Some(crate::extract_style::style_property::StyleProperty::ClassName(class)) =
            style.extract(scope)
            && atoms.get(&class).is_some()
        {
            imports.references.register_composition(class);
        }
    }
    imports.atoms.merge(atoms);
}

pub(super) fn exports(
    compiled: &Compiled,
    exports: &mut Vec<crate::vanilla_extract::naming::ExportValue>,
) {
    exports.extend(
        compiled
            .evaluation
            .iter()
            .flat_map(|evaluation| &evaluation.exports)
            .map(|name| crate::vanilla_extract::naming::ExportValue {
                name: name.clone(),
                representation: crate::vanilla_extract::naming::Representation::Authored,
            }),
    );
}
