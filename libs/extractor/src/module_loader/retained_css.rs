//! CSS loads survive removal of evaluated helpers without becoming content dependencies.

use rustc_hash::FxHashSet;

use crate::{ExtractOption, ModuleResolver};

pub(super) fn file_part(specifier: &str) -> &str {
    specifier
        .split_once('?')
        .map_or(specifier, |(path, _)| path)
}

pub(super) fn is_css(specifier: &str) -> bool {
    file_part(specifier)
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("css"))
}

pub(super) fn is_generated(specifier: &str, option: &ExtractOption) -> bool {
    let Some(file) = file_part(specifier)
        .strip_prefix(&option.css_dir)
        .and_then(|suffix| suffix.strip_prefix('/'))
    else {
        return false;
    };
    file == "devup-ui.css"
        || file
            .strip_prefix("devup-ui-")
            .and_then(|suffix| suffix.strip_suffix(".css"))
            .is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
}

pub(super) struct CssImport<'a> {
    pub specifier: &'a str,
    pub path: &'a str,
    pub direct: bool,
}

#[derive(Debug)]
pub(super) struct PathError {
    root: String,
    target: String,
}

impl PathError {
    pub(super) fn describe(&self) -> String {
        format!(
            "Cannot retain CSS '{}' relative to '{}': the paths have different Windows drives or incompatible roots. Fix: keep the stylesheet and CSS on the same drive, or import CSS from a component module",
            self.target, self.root
        )
    }
}

#[derive(Default)]
pub(super) struct RetainedCss {
    pub root: String,
    files: FxHashSet<String>,
    deferred_roots: Vec<String>,
    resolved: bool,
}

impl RetainedCss {
    pub(super) fn keep_root(&mut self, specifier: &str, resolver: Option<&ModuleResolver>) -> bool {
        if self.resolved {
            if let Some(module) = resolver.and_then(|resolve| resolve(specifier, &self.root)) {
                return self.files.insert(identity(&module.path));
            }
        } else {
            self.deferred_roots.push(specifier.to_string());
        }
        true
    }

    pub(super) fn retain(
        &mut self,
        import: CssImport<'_>,
        resolver: &ModuleResolver,
        kept_imports: &mut Vec<String>,
    ) -> Result<Option<String>, PathError> {
        // Root-only bare loads still belong to the bundler. Resolve their identities
        // only when an evaluated closure needs resolved-file deduplication.
        for specifier in self.deferred_roots.drain(..) {
            if let Some(module) = resolver(&specifier, &self.root)
                && !self.files.insert(identity(&module.path))
            {
                kept_imports.retain(|kept| *kept != specifier);
            }
        }
        self.resolved = true;
        if !self.files.insert(identity(import.path)) {
            return Ok(None);
        }
        if import.direct {
            return Ok(Some(import.specifier.to_string()));
        }
        let relative = relative_specifier(&self.root, import.path)?;
        let suffix = import
            .specifier
            .find('?')
            .map_or("", |at| &import.specifier[at..]);
        Ok(Some(format!("{relative}{suffix}")))
    }
}

struct PortablePath {
    volume: String,
    segments: Vec<String>,
}

impl PortablePath {
    fn parse(path: &str) -> Self {
        let path = file_part(path).replace('\\', "/");
        let (volume, tail) = if path.as_bytes().get(1) == Some(&b':') {
            (path[..2].to_string(), &path[2..])
        } else if path.starts_with('/') {
            ("/".to_string(), path.as_str())
        } else {
            (String::new(), path.as_str())
        };
        let mut segments = Vec::new();
        for segment in tail.split('/') {
            match segment {
                "" | "." => {}
                ".." if segments.last().is_some_and(|last| last != "..") => {
                    segments.pop();
                }
                ".." if !volume.is_empty() => {}
                segment => segments.push(segment.to_string()),
            }
        }
        Self { volume, segments }
    }
}

fn identity(path: &str) -> String {
    let parsed = PortablePath::parse(path);
    let path = format!("{}/{}", parsed.volume, parsed.segments.join("/"));
    if parsed.volume.ends_with(':') {
        path.to_ascii_lowercase()
    } else {
        path
    }
}

pub(super) fn relative_specifier(root: &str, target: &str) -> Result<String, PathError> {
    let mut root_path = PortablePath::parse(root);
    root_path.segments.pop();
    let target_path = PortablePath::parse(target);
    if !root_path.volume.eq_ignore_ascii_case(&target_path.volume) {
        return Err(PathError {
            root: root.to_string(),
            target: target.to_string(),
        });
    }
    let windows = root_path.volume.ends_with(':');
    let common = root_path
        .segments
        .iter()
        .zip(&target_path.segments)
        .take_while(|(left, right)| {
            if windows {
                left.eq_ignore_ascii_case(right)
            } else {
                left == right
            }
        })
        .count();
    let parents = "../".repeat(root_path.segments.len() - common);
    let tail = target_path.segments[common..].join("/");
    let prefix = if parents.is_empty() { "./" } else { &parents };
    Ok(format!("{prefix}{tail}"))
}
