use super::{
    BTreeSet, Binding, CollectedStyles, Context, FxHashMap, FxHashSet, JsResult, JsValue,
    Reference, Source, js_str, operands, placeholder_index, value_to_code,
};

pub(super) struct NameScope<'a> {
    pub file_num: usize,
    pub reserved: &'a BTreeSet<String>,
}

impl NameScope<'_> {
    pub(super) fn name_entries(
        self,
        collected: &mut CollectedStyles,
        bindings: &[Binding],
        context: &mut Context,
    ) -> JsResult<()> {
        let values: Vec<(&Binding, JsValue)> = bindings
            .iter()
            .map(|binding| {
                context
                    .eval(Source::from_bytes(binding.read.as_bytes()))
                    .map(|value| (binding, value))
            })
            .collect::<JsResult<_>>()?;
        let names = name_values(collected, &values, self);
        for (binding, value) in &values {
            if let Some(alias) = &binding.alias {
                if let Some(code) = value_to_code(value, context, &names, &mut Vec::new())? {
                    collected
                        .export_aliases
                        .push((binding.name.clone(), alias.clone(), code));
                }
                continue;
            }
            let names_entry = js_str(value).is_some_and(|id| names.get(&id) == Some(&binding.name));
            if binding.exported
                && !names_entry
                && let Some(code) = value_to_code(value, context, &names, &mut Vec::new())?
                    .or_else(|| binding.init.clone())
            {
                collected
                    .constant_exports
                    .push((binding.name.clone(), code));
            }
        }
        Ok(())
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
