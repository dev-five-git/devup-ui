use super::{Exports, Shape, Terminal, Walker};
use rustc_hash::FxHashSet;
use std::{collections::BTreeMap, rc::Rc};

impl Walker<'_, '_> {
    pub(in crate::barrel::native) fn native_names(
        &mut self,
        module: &Exports,
        seen: &mut FxHashSet<String>,
    ) -> Vec<String> {
        if !seen.insert(module.path.clone()) {
            return Vec::new();
        }
        let mut names: Vec<_> = module.named.keys().cloned().collect();
        for source in &module.stars {
            if source == self.package {
                names.extend(
                    crate::ordinary_ve::APIS
                        .iter()
                        .map(|name| (*name).to_string()),
                );
            } else if let Some(next) = self.module(source, &module.path) {
                names.extend(
                    self.native_names(&next, seen)
                        .into_iter()
                        .filter(|name| name != "default"),
                );
            }
        }
        names.sort();
        names.dedup();
        names
    }

    pub(in crate::barrel::native) fn native_shape(
        &mut self,
        terminal: Terminal,
        seen: &mut FxHashSet<String>,
    ) -> Option<Rc<Shape>> {
        match terminal {
            Terminal::Binding { api: Some(api), .. } => Some(Rc::new(Shape::Api(api))),
            Terminal::Binding { api: None, .. }
            | Terminal::Absent
            | Terminal::Ambiguous { native: false, .. } => None,
            Terminal::Ambiguous { native: true, name } => Some(Rc::new(Shape::Failed(format!(
                "ambiguous native export `{name}`"
            )))),
            Terminal::Failed(message) => Some(Rc::new(Shape::Failed(message))),
            Terminal::OriginalFailure(message) => Some(Rc::new(Shape::OriginalFailure(message))),
            Terminal::Namespace { source, .. } if source == self.package => {
                let members = crate::ordinary_ve::APIS
                    .iter()
                    .map(|api| ((*api).to_string(), Rc::new(Shape::Api(api))))
                    .collect();
                // Only the native package is closed; Devup also exports non-native APIs.
                Some(Rc::new(if self.package == "@vanilla-extract/css" {
                    Shape::PackageNamespace(members)
                } else {
                    Shape::Namespace(members)
                }))
            }
            Terminal::Namespace { source, .. } => {
                if !seen.insert(source.clone()) {
                    return Some(Rc::new(Shape::Failed(
                        "recursive native namespace needs an exact terminal member".to_string(),
                    )));
                }
                let module = self.modules.get(&source)?.clone();
                let names = self.native_names(&module, &mut FxHashSet::default());
                let mut members = BTreeMap::new();
                for name in names {
                    let terminal = self.native_origin(&module, &name, &mut Vec::new());
                    if let Some(shape) = self.native_shape(terminal, seen) {
                        members.insert(name, shape);
                    }
                }
                seen.remove(&source);
                (!members.is_empty()).then(|| Rc::new(Shape::Namespace(members)))
            }
        }
    }
}
