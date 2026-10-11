use super::support::*;
use super::*;
use std::rc::Rc;

#[rstest]
#[serial]
fn sheet_when_consumer_fails_after_producer_completion_is_unchanged(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let mut graph = native(suffix, single);
    graph.extract()?;
    let before = export_sheet_internal()?;
    graph.replace_origin(&format!(
        "{}const make=ve.style;export {{make}};const browser=window.document;",
        roots(("13px", "teal", "0.25"))
    ));
    graph.source = graph.source.replace("color:'blue'", "color:window.name");
    // When
    let error = graph
        .extract()
        .err()
        .ok_or("consumer failure published output")?;
    // Then
    assert!(error.contains(&format!("{}:", graph.entry)), "{error}");
    assert!(
        error.contains("cannot use `window.name` at build time"),
        "{error}"
    );
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}

#[rstest]
#[serial]
fn sheet_when_fallback_parent_fails_after_child_extraction_is_unchanged(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let mut graph = native(suffix, single);
    graph.extract()?;
    let before = export_sheet_internal()?;
    let parent = format!("/b-failed-fallback.{suffix}");
    graph.source = concat!("import {style} from '@vanilla-extract/css';", "import {base} from './parent';export const box=style([base,{margin:3}]);const browser=window.document;").into();
    graph.modules.insert("./parent".into(), ResolvedModule { path: parent.clone(), code: "import {css} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({width:'17px'});throw new Error('fallback after child');".into() });
    graph.modules.insert("./child".into(), ResolvedModule { path: format!("/b-failed-child.{suffix}"), code: "import {css,globalCss} from '@devup-ui/react';export const base=css({color:'orange',p:2});globalCss({body:{borderWidth:'12px'}});".into() });
    // When
    let error = graph
        .extract()
        .err()
        .ok_or("fallback failure published output")?;
    // Then
    assert!(error.contains("fallback after child"), "{error}");
    assert!(error.contains(&format!("{parent}:")), "{error}");
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}

#[rstest]
#[serial]
fn sheet_when_exploratory_resolver_fails_after_graph_resolution_is_unchanged(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let graph = native(suffix, single);
    graph.extract()?;
    let before = export_sheet_internal()?;
    let state = Rc::new(RefCell::new(ResolverState::new(
        &graph.entry,
        &graph.source,
    )));
    let recorded = Rc::clone(&state);
    let resolve = graph.resolver();
    let resolver = move |specifier: &str, importer: &str| {
        let request = recorded.borrow().request(specifier, importer)?;
        let module = resolve(specifier, importer)?;
        recorded.borrow_mut().cache(&module, &request);
        if specifier == "./origin" {
            recorded
                .borrow_mut()
                .record(&request, "producer exploration denied");
        }
        Some(module)
    };
    // When
    let error = code_extract_internal_impl(
        &graph.entry,
        &graph.source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        aliases(),
        SourceMapMode::Generate,
        Some(&resolver),
        Some(&state),
    )
    .err()
    .ok_or("resolver failure published output")?;
    // Then
    assert!(
        error.contains(&format!("/b-left.{suffix}:1:31:")),
        "{error}"
    );
    assert!(
        error.contains("module resolver failed") && error.contains("producer exploration denied"),
        "{error}"
    );
    assert!(error.contains("Fix: repair the module resolver"), "{error}");
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}
