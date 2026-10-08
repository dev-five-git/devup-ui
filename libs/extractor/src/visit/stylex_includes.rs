use super::{DevupVisitor, FxHashMap, StylexNamespaceValue, SymbolId, build_time_error};
use crate::stylex::StylexIncludeRef;

impl DevupVisitor<'_> {
    pub(super) fn merge_stylex_includes(
        &mut self,
        own: Vec<(String, String)>,
        includes: &[StylexIncludeRef],
        visible: &FxHashMap<String, SymbolId>,
    ) -> Vec<(String, String)> {
        let mut merged = Vec::new();
        let mut includes = includes.iter().peekable();
        for (index, entry) in own.into_iter().map(Some).chain([None]).enumerate() {
            while let Some(include) = includes.peek()
                && include.before_group == index
            {
                let symbol = visible.get(&include.var_name);
                let namespace = symbol
                    .and_then(|symbol| self.stylex_namespaces.get(symbol))
                    .and_then(|namespace| namespace.get(&include.member_name));
                let requirement = match namespace {
                    Some(StylexNamespaceValue::Dynamic(_)) => Some(
                        "an uncalled dynamic namespace cannot be included exactly; include a plain static namespace or call the function directly in props/attrs",
                    ),
                    Some(StylexNamespaceValue::Static(_)) => None,
                    None => {
                        Some("it takes a namespace `stylex.create()` defines earlier in this file")
                    }
                };
                if let Some(requirement) = requirement {
                    self.errors.push((
                        include.offset,
                        build_time_error(
                            "stylex.include",
                            &format!("{}.{}", include.var_name, include.member_name),
                            requirement,
                        ),
                    ));
                } else if let Some(keys) = symbol
                    .and_then(|symbol| self.stylex_keys.get(symbol))
                    .and_then(|namespace| namespace.get(&include.member_name))
                {
                    for (key, class) in keys {
                        merged.retain(|(existing, _)| existing != key);
                        merged.push((key.clone(), class.clone()));
                    }
                }
                includes.next();
            }
            if let Some((key, class)) = entry {
                merged.retain(|(existing, _)| *existing != key);
                merged.push((key, class));
            }
        }
        merged
    }
}
