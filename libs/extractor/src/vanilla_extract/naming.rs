use super::{
    BTreeSet, Binding, CollectedStyles, Context, FxHashMap, FxHashSet, JsResult, JsValue,
    Reference, Source, js_str, operands, placeholder_index, value_to_code,
};

pub(super) struct NameScope<'a> {
    pub file_num: usize,
    pub reserved: &'a BTreeSet<String>,
}

pub(super) fn reserved(stylesheet: super::Stylesheet<'_>) -> BTreeSet<String> {
    let allocator = oxc_allocator::Allocator::default();
    let program = oxc_parser::Parser::new(
        &allocator,
        stylesheet.code,
        oxc_span::SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse()
    .program;
    let semantic = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic;
    reserved_names(semantic.scoping())
}

pub(super) fn reserved_names(scoping: &oxc_semantic::Scoping) -> BTreeSet<String> {
    scoping
        .symbol_ids()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == scoping.root_scope_id())
        .map(|symbol| scoping.symbol_name(symbol).to_string())
        .chain(
            scoping
                .root_unresolved_references()
                .keys()
                .map(ToString::to_string),
        )
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Representation {
    Generated,
    Serialized,
    Authored,
}

#[derive(Debug, Clone)]
pub(super) struct ExportValue {
    pub name: String,
    pub representation: Representation,
}

impl NameScope<'_> {
    pub(super) fn name_entries(
        self,
        collected: &mut CollectedStyles,
        bindings: &[Binding],
        context: &mut Context,
    ) -> JsResult<Vec<ExportValue>> {
        let values: Vec<(&Binding, JsValue)> = bindings
            .iter()
            .map(|binding| {
                context
                    .eval(Source::from_bytes(binding.read.as_bytes()))
                    .map(|value| (binding, value))
            })
            .collect::<JsResult<_>>()?;
        let names = name_values(collected, &values, self);
        let mut exports = Vec::new();
        for (binding, value) in &values {
            if let Some(alias) = &binding.alias {
                let code = value_to_code(value, context, &names, &mut Vec::new())?;
                exports.push(ExportValue {
                    name: alias.clone(),
                    representation: match &code {
                        Some(_) => Representation::Serialized,
                        None => Representation::Authored,
                    },
                });
                if let Some(code) = code {
                    collected
                        .export_aliases
                        .push((binding.name.clone(), alias.clone(), code));
                }
                continue;
            }
            let names_entry = js_str(value).is_some_and(|id| names.get(&id) == Some(&binding.name));
            if !binding.exported {
                continue;
            }
            if names_entry {
                exports.push(ExportValue {
                    name: binding.name.clone(),
                    representation: Representation::Generated,
                });
                continue;
            }
            let code = value_to_code(value, context, &names, &mut Vec::new())?;
            exports.push(ExportValue {
                name: binding.name.clone(),
                representation: match &code {
                    Some(_) => Representation::Serialized,
                    None => Representation::Authored,
                },
            });
            if let Some(code) = code.or_else(|| binding.init.clone()) {
                collected
                    .constant_exports
                    .push((binding.name.clone(), code));
            }
        }
        Ok(exports)
    }
}

pub(super) fn name_values(
    collected: &mut CollectedStyles,
    values: &[(&Binding, JsValue)],
    scope: NameScope<'_>,
) -> FxHashMap<String, String> {
    let NameScope { file_num, reserved } = scope;
    let mut names: FxHashMap<String, String> = FxHashMap::default();
    for (binding, value) in values {
        if let Some(id) = js_str(value)
            && !names.contains_key(&id)
            && let Some(entry) = collected
                .styles
                .get_mut(&id)
                .or_else(|| collected.keyframes.get_mut(&id))
        {
            entry.exported = binding.exported;
            names.insert(id, binding.name.clone());
        }
    }
    let declared: FxHashSet<&str> = values
        .iter()
        .map(|(binding, _)| binding.name.as_str())
        .chain(reserved.iter().map(String::as_str))
        .collect();
    let mut anonymous: Vec<String> = collected
        .styles
        .keys()
        .chain(collected.keyframes.keys())
        .filter(|id| !names.contains_key(*id))
        .cloned()
        .collect();
    anonymous.sort_by_key(|id| placeholder_index(id));
    for id in anonymous {
        let mut name = format!("_ve{}", placeholder_index(&id));
        while declared.contains(name.as_str()) || names.values().any(|used| used == &name) {
            name.push('_');
        }
        names.insert(id, name);
    }
    let mut styles = std::mem::take(&mut collected.styles);
    let mut keyframes = std::mem::take(&mut collected.keyframes);
    let mut ordered: Vec<_> = names.iter().collect();
    ordered.sort_by_key(|(id, _)| placeholder_index(id));
    for (id, name) in ordered {
        let reference = if let Some(mut entry) = styles.remove(id) {
            for operand in &mut entry.operands {
                if let operands::StyleOperand::Base(base) = operand
                    && let Some(base_name) = names.get(base.as_str())
                {
                    base_name.clone_into(base);
                }
            }
            collected.styles.insert(name.clone(), entry);
            Reference::Style {
                name: name.clone(),
                class_name: format!("f{file_num}_{}", placeholder_index(id)),
            }
        } else {
            if let Some(entry) = keyframes.remove(id) {
                collected.keyframes.insert(name.clone(), entry);
            }
            Reference::Keyframes(name.clone())
        };
        collected.references.insert(id.clone(), reference);
    }
    names
}
