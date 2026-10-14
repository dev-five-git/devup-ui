use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use crate::ModuleResolver;

use super::{
    collect,
    model::{Model, Node, Shape},
    resolve::Evaluation,
};

pub(super) struct Graph<'r> {
    resolver: Option<&'r ModuleResolver>,
    package: &'r str,
    requests: FxHashMap<(String, String), Option<String>>,
    exports: FxHashMap<String, BTreeMap<String, Shape>>,
    loading: BTreeSet<String>,
    pub(super) dependencies: BTreeSet<String>,
}

impl<'r> Graph<'r> {
    pub(super) fn new(resolver: Option<&'r ModuleResolver>, package: &'r str) -> Self {
        Self {
            resolver,
            package,
            requests: FxHashMap::default(),
            exports: FxHashMap::default(),
            loading: BTreeSet::new(),
            dependencies: BTreeSet::new(),
        }
    }

    pub(super) fn resolve(&mut self, model: &Model, filename: &str, node: &Node) -> Shape {
        Evaluation::new(model, filename).resolve(self, node)
    }

    pub(super) fn imported(&mut self, source: &str, importer: &str) -> BTreeMap<String, Shape> {
        if source == self.package
            || source
                .strip_prefix(self.package)
                .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return BTreeMap::new();
        }
        let request = (importer.to_string(), source.to_string());
        if let Some(path) = self.requests.get(&request) {
            return path
                .as_ref()
                .and_then(|path| self.exports.get(path))
                .cloned()
                .unwrap_or_default();
        }
        let Some(module) = self
            .resolver
            .and_then(|resolver| resolver(source, importer))
        else {
            self.requests.insert(request, None);
            return BTreeMap::new();
        };
        self.requests.insert(request, Some(module.path.clone()));
        self.dependencies.insert(module.path.clone());
        if let Some(exports) = self.exports.get(&module.path) {
            return exports.clone();
        }
        if self.loading.len() >= 64 || !self.loading.insert(module.path.clone()) {
            return BTreeMap::new();
        }
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&module.path).unwrap_or_else(|_| SourceType::ts());
        let parsed = Parser::new(&allocator, &module.code, source_type).parse();
        let model = if parsed.fatal_error || !parsed.diagnostics.is_empty() {
            Model::default()
        } else {
            collect::model(&parsed.program)
        };
        let mut exports: BTreeMap<_, _> = model
            .exports
            .iter()
            .map(|(name, node)| (name.clone(), self.resolve(&model, &module.path, node)))
            .collect();
        let mut stars = BTreeMap::new();
        for source in &model.stars {
            for (name, shape) in self.imported(source, &module.path) {
                if name != "default" && !model.exports.contains_key(&name) {
                    stars
                        .entry(name)
                        .and_modify(|previous| *previous = Shape::UNKNOWN)
                        .or_insert(shape);
                }
            }
        }
        exports.extend(stars);
        self.loading.remove(&module.path);
        self.exports.insert(module.path, exports.clone());
        exports
    }
}
