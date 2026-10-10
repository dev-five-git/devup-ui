use oxc_allocator::Allocator;
use rstest::rstest;

use super::{Demand, Frozen};
use crate::module_loader::ModuleLoader;
use crate::{ModuleResolver, ResolvedModule};

#[path = "loader_coverage_support.rs"]
pub(super) mod support;
use support::{backedge, binds, filenames, program, semantic, stylesheet};

#[test]
fn native_selection_refreshes_when_a_helper_returns_to_the_entry() -> Result<(), String> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nimport {read} from './helper';\nexport function palette(){return 'blue';}\nexport const box=style({color:read()});\nconst browser=window.document;\n";
    let helper = "import {palette} from './entry';\nexport function read(){return palette();}\nconst browser=window.document;\n";
    let resolver = backedge(source, helper);
    let option = support::option();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    let allocator = Allocator::default();
    let program = program(&allocator, source);
    let semantic = semantic(&program);
    let initial = crate::ordinary_ve::selection::select_resolved(
        (&program, &semantic),
        ("/entry.ts", "@vanilla-extract/css"),
        Some(&resolver),
    );
    assert!(!binds(&initial, "palette"));
    // When
    loader.select_demands(stylesheet("/entry.ts", source), true)?;
    // Then
    let selected = loader
        .entry_selection()
        .ok_or("missing refreshed native selection")?;
    assert!(binds(selected, "palette"));
    assert!(binds(selected, "box"));
    assert_eq!(selected.roots.len(), 1);
    let frozen = loader.selected.as_ref().ok_or("missing frozen discovery")?;
    assert!(!frozen.units.contains_key("/entry.ts"));
    assert_eq!(filenames(loader.environments()), ["/helper.ts"]);
    assert!(!filenames(loader.producers()).contains(&"/entry.ts"));
    assert_eq!(loader.namespaces().len(), 1);
    Ok(())
}

#[test]
fn consumer_view_refreshes_when_a_native_helper_returns_to_the_entry() -> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react';\nimport {token,read} from './helper';\nexport function palette(){return 'blue';}\nexport const box=css({margin:token,color:read()});\nconst browser=window.document;\n";
    let helper = "import {createVar} from '@vanilla-extract/css';\nimport {palette} from './entry';\nexport const token=createVar();\nexport function read(){return palette();}\nconst browser=window.document;\n";
    let resolver = backedge(source, helper);
    let option = support::option();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    // When
    let (accepted, initial) = support::consumers(&mut loader, stylesheet("/entry.ts", source));
    // Then
    assert!(accepted?);
    assert_eq!(initial.exports, Vec::<(String, String)>::new());
    assert!(!binds(&initial.selection, "palette"));
    let view = loader
        .entry_view()
        .ok_or("missing refreshed consumer view")?;
    assert!(binds(&view.selection, "palette"));
    assert!(view.exports.contains(&("palette".into(), "palette".into())));
    let frozen = loader.selected.as_ref().ok_or("missing frozen discovery")?;
    assert!(!frozen.units.contains_key("/entry.ts"));
    assert_eq!(filenames(loader.environments()), ["/helper.ts"]);
    assert_eq!(filenames(loader.producers()), ["/helper.ts"]);
    assert_eq!(loader.namespaces().len(), 1);
    Ok(())
}

#[rstest]
#[case(false, "Cannot load './missing' without a module resolver")]
#[case(true, "Cannot resolve './missing' from '/failure.ts'")]
fn native_failure_is_located_when_a_demanded_import_cannot_load(
    #[case] with_resolver: bool,
    #[case] category: &str,
) -> Result<(), String> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nimport {color} from './missing';\nexport const box=style({color});\n";
    let unresolved = |_: &str, _: &str| None;
    let resolver: Option<&ModuleResolver> = with_resolver.then_some(&unresolved);
    let option = support::option();
    let mut loader = ModuleLoader::new(resolver, &option);
    let offset = source
        .find("color} from")
        .ok_or("missing authored specifier")?;
    let place = crate::locate("/failure.ts", source, offset);
    // When
    let error = loader
        .select_demands(stylesheet("/failure.ts", source), true)
        .err()
        .ok_or("missing native load failure")?;
    // Then
    let prefix = format!("{place}: {category}. Fix: ");
    assert!(error.starts_with(&prefix), "{error}");
    assert!(!loader.has_demands());
    assert_eq!(loader.namespaces().len(), 0);
    Ok(())
}

#[rstest]
#[case(false, "Cannot load './b' without a module resolver")]
#[case(true, "Cannot resolve './b' from '/failure.ts'")]
fn consumer_failure_is_reported_when_the_entry_has_a_native_anchor(
    #[case] with_resolver: bool,
    #[case] category: &str,
) -> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react';\nimport {style} from '@vanilla-extract/css';\nimport {a} from './a';\nimport {b} from './b';\nexport const native=style({color:'red'});\nexport const box=css({color:a,background:b});\n";
    let unresolved = |_: &str, _: &str| None;
    let resolver: Option<&ModuleResolver> = with_resolver.then_some(&unresolved);
    let option = support::option();
    let mut loader = ModuleLoader::new(resolver, &option);
    let offset = source
        .find("b} from")
        .ok_or("missing authored b specifier")?;
    let place = crate::locate("/failure.ts", source, offset);
    // When
    let (result, initial) = support::consumers(&mut loader, stylesheet("/failure.ts", source));
    // Then
    assert_eq!(initial.forwarded.len(), 2);
    assert_eq!(
        initial
            .forwarded
            .iter()
            .map(|(specifier, demand, _)| (specifier.as_str(), demand.clone()))
            .collect::<Vec<_>>(),
        [
            ("./a", Demand::prefixed("a", &Demand::whole())),
            ("./b", Demand::prefixed("b", &Demand::whole())),
        ]
    );
    let error = result.err().ok_or("missing anchored consumer failure")?;
    let prefix = format!("{place}: {category}. Fix: ");
    assert!(error.starts_with(&prefix), "{error}");
    Ok(())
}

#[test]
fn consumer_declines_when_load_failures_have_no_native_anchor() -> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react';\nimport {a} from './a';\nimport {b} from './b';\nexport const box=css({color:a,background:b});\n";
    let option = support::option();
    let mut loader = ModuleLoader::new(None, &option);
    // When
    let (result, initial) = support::consumers(&mut loader, stylesheet("/failure.ts", source));
    // Then
    assert_eq!(
        initial
            .forwarded
            .iter()
            .map(|(specifier, _, _)| specifier.as_str())
            .collect::<Vec<_>>(),
        ["./a", "./b"]
    );
    assert!(!result?);
    assert!(!loader.has_demands());
    assert!(loader.entry_view().is_none());
    assert_eq!(loader.namespaces().len(), 0);
    Ok(())
}

#[test]
fn raw_css_is_not_published_when_a_value_import_resolves_to_css() -> Result<(), String> {
    // Given
    let source = "import styles from './raw.module.css';export const result=styles.card;";
    let css = ".card { color: red; }";
    let resolver = move |specifier: &str, importer: &str| {
        ((specifier, importer) == ("./raw.module.css", "/css-edge.ts")).then(|| ResolvedModule {
            path: "/raw.module.css".into(),
            code: css.into(),
        })
    };
    let option = support::option();
    let loader = ModuleLoader::new(Some(&resolver), &option);
    // When
    let frozen = Frozen::discover(stylesheet("/css-edge.ts", source), (false, None), &loader)?;
    // Then
    assert!(!frozen.units.contains_key("/raw.module.css"));
    assert!(!filenames(&frozen.environments).contains(&"/raw.module.css"));
    assert!(!filenames(&frozen.producers).contains(&"/raw.module.css"));
    assert!(!frozen.source.contains(css));
    Ok(())
}
