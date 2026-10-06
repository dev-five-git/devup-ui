//! Escape graph eligibility shared by constant substitution and Boa closures.

use super::{Change, ChangeSite, Constant, Imported, ModuleScope, Modules, provenance};
use crate::mutations::Use;
use oxc_semantic::SemanticBuilder;
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Evaluated own slots eligible for shallow freeze protection. Namespace
/// exports are not locally allocated containers and cannot acquire this proof.
enum Container {
    Record(Rc<Vec<(String, Constant)>>),
    Array(Rc<Vec<Constant>>),
}

impl ModuleScope<'_, '_> {
    /// What `name` holds, or where code changes it when it is an object or
    /// array the module changes.
    pub(super) fn lookup(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Constant> {
        let value = self.lookup_raw(modules, name);
        if value.as_ref().is_none_or(Constant::is_mutable)
            && let Some(change) = self.change(modules, name)
        {
            if let Some(value) = &value
                && self.primitive_snapshot(&change)
            {
                return Some(value.clone());
            }
            return Some(match value {
                Some(Constant::Record(fields)) => {
                    self.frozen_value(name, Container::Record(fields), &change)
                }
                Some(Constant::Array(values)) => {
                    self.frozen_value(name, Container::Array(values), &change)
                }
                Some(_) | None => Constant::Changed(change),
            });
        }
        value
    }

    pub(super) fn change(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Rc<Change>> {
        if self.uses.is_none() {
            let option = modules.option;
            let uses = Rc::new(crate::mutations::uses(
                self.program,
                &|name| self.is_style_import(option, name),
                self.css_prop,
            ));
            // Seed all vertices before lazy value lookup: cycles are not safety proofs.
            for name in uses.keys() {
                self.changes.insert(name.clone(), None);
            }
            self.uses = Some(uses.clone());
            self.effects(modules, &uses);
        }
        self.changes.get(name).cloned().flatten()
    }

    fn effects(&mut self, modules: &mut Modules<'_>, uses: &FxHashMap<String, Vec<Use>>) {
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(self.program)
            .semantic;
        let proof = provenance::Proof {
            nodes: semantic.nodes(),
            scoping: semantic.scoping(),
        };
        let mut edges = Vec::new();
        let mut names: Vec<_> = uses.keys().collect();
        names.sort();
        for name in names {
            let namespace = matches!(self.imports.get(name), Some((_, Imported::Namespace)));
            let value =
                if self.imports.contains_key(name) && !self.is_style_import(modules.option, name) {
                    self.lookup_raw(modules, name)
                } else {
                    self.locals.get(name).cloned()
                };
            let symbol = proof.scoping.get_root_binding(name.as_str().into());
            let shape = symbol.map_or(provenance::Shape::Unknown, |symbol| proof.binding(symbol));
            for found in &uses[name] {
                let (at, handed, path, into) = match found {
                    Use::Changes { at, depth } => {
                        if namespace && *depth <= 1 {
                            continue;
                        }
                        (*at, false, None, None)
                    }
                    Use::Calls { at, path } => {
                        if namespace {
                            continue;
                        }
                        (*at, true, Some(path.clone()), None)
                    }
                    Use::Escapes { at, path, into } => {
                        let mut path = path.clone();
                        if namespace && path.is_empty() {
                            path.push(None);
                        }
                        (*at, true, Some(path), into.as_ref())
                    }
                };
                if let Some(path) = &path
                    && (value
                        .as_ref()
                        .is_some_and(|value| value.reaches_only_primitives(path))
                        || shape.primitive_path(path))
                {
                    continue;
                }
                if let Some(into) = into {
                    if !into.is_empty() {
                        edges.push((name.clone(), into.clone()));
                    }
                    continue;
                }
                if shape.shallow_primitives()
                    && symbol.is_some_and(|symbol| {
                        super::freeze::before(&proof, self.program, symbol, at, uses)
                    })
                {
                    continue;
                }
                let change = self.site(name, at, handed);
                Self::keep_first(&mut self.changes, name, change);
            }
        }
        loop {
            let mut updated = false;
            for (name, into) in &edges {
                if let Some(change) = self.changes.get(into).cloned().flatten() {
                    if let ChangeSite::Here(at) = change.site
                        && proof
                            .scoping
                            .get_root_binding(name.as_str().into())
                            .is_some_and(|symbol| {
                                proof.binding(symbol).shallow_primitives()
                                    && super::freeze::before(&proof, self.program, symbol, at, uses)
                            })
                    {
                        continue;
                    }
                    let original = Rc::new(Change {
                        name: name.clone(),
                        site: match &change.site {
                            ChangeSite::Here(at) => ChangeSite::Here(*at),
                            ChangeSite::In(location) => ChangeSite::In(location.clone()),
                        },
                        handed: change.handed,
                    });
                    updated |= Self::keep_first(&mut self.changes, name, original);
                }
            }
            if !updated {
                break;
            }
        }
    }

    fn frozen_value(&self, name: &str, value: Container, change: &Rc<Change>) -> Constant {
        let ChangeSite::Here(at) = change.site else {
            return Constant::Changed(change.clone());
        };
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(self.program)
            .semantic;
        let proof = provenance::Proof {
            nodes: semantic.nodes(),
            scoping: semantic.scoping(),
        };
        if !proof
            .scoping
            .get_root_binding(name.into())
            .is_some_and(|symbol| {
                self.uses.as_deref().is_some_and(|uses| {
                    super::freeze::before(&proof, self.program, symbol, at, uses)
                })
            })
        {
            return Constant::Changed(change.clone());
        }
        let protect = |value: &Constant| {
            if value.is_mutable() {
                Constant::Changed(change.clone())
            } else {
                value.clone()
            }
        };
        match value {
            Container::Record(fields) => Constant::Record(Rc::new(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), protect(value)))
                    .collect(),
            )),
            Container::Array(values) => {
                Constant::Array(Rc::new(values.iter().map(protect).collect()))
            }
        }
    }

    fn keep_first(
        changes: &mut FxHashMap<String, Option<Rc<Change>>>,
        name: &str,
        change: Rc<Change>,
    ) -> bool {
        let slot = changes.entry(name.to_string()).or_default();
        let earlier = slot
            .as_ref()
            .is_none_or(|previous| change.site < previous.site);
        if earlier {
            *slot = Some(change);
        }
        earlier
    }
}

#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
