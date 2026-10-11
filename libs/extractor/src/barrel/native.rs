//! Native terminal mode of the barrel walker; never rewrites Devup imports.
use std::{collections::BTreeMap, rc::Rc};

use super::{Exports, Link, Walker};

pub(super) mod aliases;
mod demand;
mod exports;
mod facts;
mod shapes;
pub(crate) use exports::{export_site, exported_terminals};
pub(crate) use facts::Facts;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(crate) enum Shape {
    Api(&'static str),
    Namespace(BTreeMap<String, Rc<Self>>),
    PackageNamespace(BTreeMap<String, Rc<Self>>),
    Failed(String),
    OriginalFailure(String),
}

impl Shape {
    pub(crate) fn paths(&self) -> Vec<Vec<String>> {
        match self {
            Self::Api(_) | Self::Failed(_) | Self::OriginalFailure(_) => vec![Vec::new()],
            Self::Namespace(members) | Self::PackageNamespace(members) => members
                .iter()
                .flat_map(|(key, shape)| {
                    shape
                        .paths()
                        .into_iter()
                        .map(|path| std::iter::once(key.clone()).chain(path).collect())
                })
                .collect(),
        }
    }
    pub(crate) fn member(&self, name: &str) -> Option<Rc<Self>> {
        match self {
            Self::Namespace(members) | Self::PackageNamespace(members) => {
                members.get(name).cloned()
            }
            Self::Api(_) | Self::Failed(_) | Self::OriginalFailure(_) => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Terminal {
    Binding {
        owner: String,
        name: String,
        api: Option<&'static str>,
    },
    Namespace {
        source: String,
    },
    Absent,
    Ambiguous {
        native: bool,
        name: String,
    },
    Failed(String),
    OriginalFailure(String),
}

fn api(name: &str) -> Option<&'static str> {
    crate::ordinary_ve::APIS
        .iter()
        .copied()
        .find(|api| *api == name)
}

impl Walker<'_, '_> {
    fn native_import(&mut self, source: &str, importer: &str, name: Option<&str>) -> Terminal {
        if source == self.package {
            return match name {
                Some(name) => Terminal::Binding {
                    owner: source.to_string(),
                    name: name.to_string(),
                    api: api(name),
                },
                None => Terminal::Namespace {
                    source: source.to_string(),
                },
            };
        }
        let Some(module) = self.module(source, importer) else {
            return Terminal::Absent;
        };
        match name {
            Some(name) => self.native_origin(&module, name, &mut Vec::new()),
            None => Terminal::Namespace {
                source: module.path.clone(),
            },
        }
    }

    fn native_link(
        &mut self,
        module: &Exports,
        link: &Link,
        reading: &mut Vec<(String, String)>,
    ) -> Terminal {
        match link {
            Link::Own => Terminal::Absent,
            Link::Changed { base, place } => {
                let terminal = self.native_link(module, base, reading);
                if self
                    .native_shape(terminal.clone(), &mut rustc_hash::FxHashSet::default())
                    .is_some()
                {
                    Terminal::OriginalFailure(format!(
                        "{place}: native API alias may be changed outside its exact initialization slice. Fix: use immutable native API aliases without mutation or handoff"
                    ))
                } else {
                    terminal
                }
            }
            Link::From { source, imported } if source == self.package => {
                self.native_import(source, &module.path, imported.as_deref())
            }
            Link::From { source, imported } => {
                let Some(next) = self.module(source, &module.path) else {
                    return if module.mentions_package {
                        Terminal::Failed(format!("native re-export `{source}` cannot be read"))
                    } else {
                        Terminal::Absent
                    };
                };
                match imported {
                    Some(name) => self.native_origin(&next, name, reading),
                    None => Terminal::Namespace {
                        source: next.path.clone(),
                    },
                }
            }
            Link::Member { base, name } => {
                let terminal = self.native_link(module, base, reading);
                match terminal {
                    Terminal::Namespace { source } => {
                        if source == self.package {
                            self.native_import(&source, "", Some(name))
                        } else if let Some(next) = self.modules.get(&source).cloned() {
                            self.native_origin(&next, name, reading)
                        } else {
                            Terminal::Absent
                        }
                    }
                    Terminal::Binding { .. } | Terminal::Absent => Terminal::Absent,
                    Terminal::Failed(message) => Terminal::Failed(message),
                    Terminal::OriginalFailure(message) => Terminal::OriginalFailure(message),
                    Terminal::Ambiguous { native, name } => Terminal::Ambiguous { native, name },
                }
            }
        }
    }

    fn native_origin(
        &mut self,
        module: &Exports,
        name: &str,
        reading: &mut Vec<(String, String)>,
    ) -> Terminal {
        let key = (module.path.clone(), name.to_string());
        if reading.contains(&key) {
            return Terminal::Absent;
        }
        reading.push(key);
        let found = match module.named.get(name) {
            Some(Link::Own) => Terminal::Binding {
                owner: module.path.clone(),
                name: name.to_string(),
                api: None,
            },
            Some(link) => self.native_link(module, link, reading),
            None if name == "default" => Terminal::Absent,
            None => {
                let mut found = Terminal::Absent;
                for source in &module.stars {
                    let candidate = if source == self.package {
                        self.native_import(source, &module.path, Some(name))
                    } else if let Some(next) = self.module(source, &module.path) {
                        self.native_origin(&next, name, reading)
                    } else if module.mentions_package {
                        Terminal::Failed(format!("native re-export `{source}` cannot be read"))
                    } else {
                        Terminal::Absent
                    };
                    match (&found, &candidate) {
                        (_, Terminal::Absent) => {}
                        (Terminal::Absent, _) => found = candidate,
                        _ if found == candidate => {}
                        _ => {
                            let native = [&found, &candidate].iter().any(|terminal| {
                                matches!(
                                    terminal,
                                    Terminal::Binding { api: Some(_), .. }
                                        | Terminal::Namespace { .. }
                                        | Terminal::Failed(_)
                                        | Terminal::OriginalFailure(_)
                                        | Terminal::Ambiguous { native: true, .. }
                                )
                            });
                            found = Terminal::Ambiguous {
                                native,
                                name: name.to_string(),
                            };
                        }
                    }
                }
                found
            }
        };
        reading.pop();
        found
    }
}
