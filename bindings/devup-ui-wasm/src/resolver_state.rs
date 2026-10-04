use std::collections::{BTreeSet, HashMap};

use extractor::ResolvedModule;
use extractor::module_reference::{ModuleReferenceLocation, original_module_reference_locations};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OriginSite {
    filename: String,
    location: ModuleReferenceLocation,
}

#[derive(Clone)]
struct OriginSites {
    first: OriginSite,
    rest: Vec<OriginSite>,
}

struct ModuleSource {
    code: String,
    ingress: Option<OriginSites>,
}

/// A callback request with at least one original reference site.
pub(crate) struct ResolverRequest {
    specifier: String,
    importer: String,
    origins: OriginSites,
}

impl ResolverRequest {
    pub(crate) fn specifier(&self) -> &str {
        &self.specifier
    }

    pub(crate) fn importer(&self) -> &str {
        &self.importer
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct ResolverFailure {
    site: OriginSite,
    specifier: String,
    importer: String,
    cause: String,
}

/// Resolver diagnostics and original module sources owned by one extraction.
pub(crate) struct ResolverState {
    sources: HashMap<String, ModuleSource>,
    failures: BTreeSet<ResolverFailure>,
}

impl ResolverState {
    pub(crate) fn new(filename: &str, source: &str) -> Self {
        Self {
            sources: HashMap::from([(
                filename.to_string(),
                ModuleSource {
                    code: source.to_string(),
                    ingress: None,
                },
            )]),
            failures: BTreeSet::new(),
        }
    }

    pub(crate) fn cache(&mut self, module: &ResolvedModule, request: &ResolverRequest) {
        self.sources.insert(
            module.path.clone(),
            ModuleSource {
                code: module.code.clone(),
                ingress: Some(request.origins.clone()),
            },
        );
    }

    /// Bind original references before calling JS; recursive exploratory reads
    /// without a semantic reference retain the real import that loaded the module.
    pub(crate) fn request(&self, specifier: &str, importer: &str) -> Option<ResolverRequest> {
        let source = self.sources.get(importer)?;
        let mut sites = original_module_reference_locations(importer, &source.code, specifier)
            .into_iter()
            .map(|location| OriginSite {
                filename: importer.to_string(),
                location,
            });
        let origins = match sites.next() {
            Some(first) => OriginSites {
                first,
                rest: sites.collect(),
            },
            None => source.ingress.clone()?,
        };
        Some(ResolverRequest {
            specifier: specifier.to_string(),
            importer: importer.to_string(),
            origins,
        })
    }

    pub(crate) fn record(&mut self, request: &ResolverRequest, cause: &str) {
        for site in std::iter::once(&request.origins.first).chain(&request.origins.rest) {
            self.failures.insert(ResolverFailure {
                site: site.clone(),
                specifier: request.specifier.clone(),
                importer: request.importer.clone(),
                cause: cause.to_string(),
            });
        }
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        if self.failures.is_empty() {
            Ok(())
        } else {
            Err(self.failures.iter().map(|failure| {
                let OriginSite { filename, location } = &failure.site;
                format!(
                    "{filename}:{}:{}: module resolver failed for `{}` from `{}`: {}\nFix: repair the module resolver; return {{path:string,code:string}} with a non-empty path, or null/undefined for an unresolved module.",
                    location.line, location.column, failure.specifier, failure.importer, failure.cause,
                )
            }).collect::<Vec<_>>().join("\n"))
        }
    }
}

#[cfg(test)]
#[path = "resolver_state_tests.rs"]
mod tests;
