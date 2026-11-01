use super::execute_extracted;
use crate::{ExtractOption, ExtractStyleValue, extract};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::arguments("const mixin=css({color:'black'});const Card=styled.div(mixin,{'@layer':{a:{p:1},b:{p:2}}},{'@layer':{a:{p:3}}});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::local("const mixin=css({'@layer':{a:{p:1},b:{p:2}}});const Card=styled.div(mixin,{'@layer':{a:{p:3}}});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::local_template("const mixin=css`@layer a{ padding: 4px }@layer b{ padding: 8px }`;const Card=styled.div(mixin,{'@layer':{a:{ p: 3 }}});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::composed_class("const mixin=css({'@layer':{a:{p:1},b:{p:2}}});const merged=css(mixin,{'@layer':{a:{p:3}}});const Card=styled.div(merged);", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::css_prop("const mixin=css({'@layer':{a:{p:1},b:{p:2}}});const Card=styled.div({});const App=()=>jsx(Card,{css:[mixin,{'@layer':{a:{p:3}}}]});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::nested_raw("const Card=styled.div`@layer outer{${{'@layer':{a:{p:1},b:{p:2}}}}${{'@layer':{a:{p:3}}}}}`;", vec!["outer", "outer.a", "outer.b"], serde_json::json!([["outer.a","12px"],["outer.b","8px"]]))]
#[case::nested_known("const mixin=css({'@layer':{a:{p:1},b:{p:2}}});const Card=styled.div`@layer outer{${mixin}${{'@layer':{a:{p:3}}}}}`;", vec!["a", "b", "outer", "outer.a", "outer.b"], serde_json::json!([["outer.a","12px"],["outer.b","8px"]]))]
#[case::text("const Card=styled.div`@layer a{ padding: 4px }@layer b{ padding: 8px }@layer a{ padding: 12px }`;", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::conditional("const Card=styled.div(p=>p.on?{'@layer':{a:{p:1},b:{p:2}}}:null,{'@layer':{a:{p:3}}});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[case::with_component("const Base=styled.div({'@layer':{a:{p:1},b:{p:2}}});const Middle=Base.withComponent('section');const Card=styled(Middle)({'@layer':{a:{p:3}}});", vec!["a", "b"], serde_json::json!([["a","12px"],["b","8px"]]))]
#[serial]
fn authored_layers_survive_when_composition_discards_first_use(
    #[case] source: &str,
    #[case] expected_order: Vec<&str>,
    #[case] expected_atoms: serde_json::Value,
    #[values(false, true)] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: known authored layers whose first declaration may be removed.
    let _state = crate::test_state::TestStateGuard::new();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let filename = "/layers.ts";
    // When: extraction and the complete generated renderer consume the metadata.
    let output = extract(
        filename,
        &format!(
            "import {{styled,css}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';{source}"
        ),
        ExtractOption {
            single_css,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )?;
    let bucket = (!single_css).then(|| css::file_map::canonical(filename));
    let element = if source.contains("const App=") {
        "App()"
    } else {
        "{target:Card,props:{ on: true }}"
    };
    let observed = execute_extracted(
        (filename, bucket.as_deref()),
        &output,
        &format!(
            "JSON.stringify(atoms(render({element})).filter(r=>r[0]==='padding').map(r=>[r[4],r[1]]).sort((a,b)=>a[0].localeCompare(b[0])));"
        ),
    );
    let orders: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::LayerOrder(order) => Some((
                order.file.as_str(),
                order.layers.iter().map(String::as_str).collect::<Vec<_>>(),
            )),
            _ => None,
        })
        .collect();
    // Then: exactly one actual-file record retains authored order and selected winners.
    assert_eq!(orders, vec![(filename, expected_order)]);
    assert_eq!(observed, expected_atoms);
    Ok(())
}

#[rstest]
#[case::record(
    "import {css} from '@devup-ui/react';const rules={'@layer':{a:{p:1},b:{p:2}}};export const mixin=css(rules);"
)]
#[case::style(
    "import {css} from '@devup-ui/react';export const mixin=css({'@layer':{a:{p:1},b:{p:2}}});"
)]
#[serial]
fn imported_layers_register_in_consumer_when_first_atom_is_overridden(
    #[case] module: &'static str,
    #[values(false, true)] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: imported Record or completed Style metadata from the real resolver.
    let _state = crate::test_state::TestStateGuard::new();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = move |source: &str, _: &str| {
        (source == "./mixin").then(|| crate::ResolvedModule {
            path: "/mixin.ts".to_string(),
            code: module.to_string(),
        })
    };
    // When: the consuming styled composition overrides a.
    let output = crate::extract_with_modules(
        "/consumer.ts",
        "import {styled} from '@devup-ui/react';import {mixin} from './mixin';const Card=styled.div(mixin,{'@layer':{a:{p:3}}});",
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let bucket = (!single_css).then(|| css::file_map::canonical("/consumer.ts"));
    let observed = execute_extracted(
        ("/consumer.ts", bucket.as_deref()),
        &output,
        "JSON.stringify(atoms(render({target:Card,props:{}})).filter(r=>r[0]==='padding').map(r=>[r[4],r[1]]).sort((a,b)=>a[0].localeCompare(b[0])));",
    );
    let orders: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::LayerOrder(order) => {
                Some((order.file.as_str(), order.layers.as_slice()))
            }
            _ => None,
        })
        .collect();
    // Then: imported partial records never compete with the consumer's consolidated order.
    assert_eq!(
        orders,
        vec![(
            "/consumer.ts",
            ["a".to_string(), "b".to_string()].as_slice()
        )]
    );
    assert_eq!(observed, serde_json::json!([["a", "12px"], ["b", "8px"]]));
    Ok(())
}
