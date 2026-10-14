use std::sync::{LazyLock, Mutex};

#[derive(Clone)]
struct NamingContext {
    root: String,
    relative_base: Option<String>,
}

static ROOT: LazyLock<Mutex<Option<NamingContext>>> = LazyLock::new(|| Mutex::new(None));

/// Set the independent names-only root; resolver and delivery identities stay unchanged.
pub fn set_root(root: Option<String>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_root");
    set_context(root, None);
}

/// A relative-ID basis is names-only too; raw resolver and sheet keys are never rewritten.
pub fn set_context(root: Option<String>, relative_base: Option<String>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_context");
    let _root = crate::root_held::RootHeld::enter("naming_root");
    *ROOT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = root.map(|root| NamingContext {
        root,
        relative_base,
    });
}

fn parts(path: &str) -> (String, Vec<&str>) {
    let segments: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    let (root, start) = if path.starts_with("//") && segments.len() >= 2 {
        (format!("//{}/{}", segments[0], segments[1]), 2)
    } else if segments.first().is_some_and(|part| part.ends_with(':')) {
        (segments[0].to_ascii_lowercase(), 1)
    } else if path.starts_with('/') {
        ("/".into(), 0)
    } else {
        (String::new(), 0)
    };
    let mut normalized = Vec::new();
    for part in &segments[start..] {
        match *part {
            "." => {}
            ".." if normalized.last().is_some_and(|last| *last != "..") => {
                normalized.pop();
            }
            ".." if !root.is_empty() => {}
            _ => normalized.push(*part),
        }
    }
    (root, normalized)
}

/// Relativize only naming keys; outsiders keep `..`, incompatible roots keep absolute keys.
#[must_use]
pub fn key(filename: &str) -> String {
    let _admission = crate::admission::enter();
    let root = {
        let _root = crate::root_held::RootHeld::enter("naming_root");
        ROOT.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    };
    let Some(context) = root.as_ref() else {
        return filename.to_string();
    };
    let filename = filename.replace('\\', "/");
    let filename = match &context.relative_base {
        Some(base) if parts(&filename).0.is_empty() => {
            format!("{}/{filename}", base.replace('\\', "/"))
        }
        _ => filename,
    };
    let root = context.root.replace('\\', "/");
    let (file_root, file) = parts(&filename);
    if file_root.is_empty() {
        return file.join("/");
    }
    let (root_root, root) = parts(&root);
    if file_root != root_root {
        return if file_root == "/" {
            format!("/{}", file.join("/"))
        } else {
            format!("{file_root}/{}", file.join("/"))
        };
    }
    let common = file
        .iter()
        .zip(&root)
        .take_while(|(file, root)| file == root)
        .count();
    std::iter::repeat_n("..", root.len().saturating_sub(common))
        .chain(file[common..].iter().copied())
        .collect::<Vec<_>>()
        .join("/")
}
