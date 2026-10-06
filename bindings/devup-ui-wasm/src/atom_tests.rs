use super::*;
use serial_test::serial;
use std::collections::HashSet;

mod lifecycle;

fn setup() {
    reset_build_state_internal();
    css::debug::set_debug(false);
    register_theme_internal(sheet::theme::Theme::default());
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(HashMap::from([
        ("a.tsx".to_string(), HashSet::from([0, 1])),
        ("b.tsx".to_string(), HashSet::from([0, 1])),
        ("private.tsx".to_string(), HashSet::from([0])),
        ("styles.css.ts".to_string(), HashSet::from([0, 1])),
    ]));
    seed_file_map(vec!["a.tsx".into(), "b.tsx".into(), "private.tsx".into()]);
}

fn compile(file: &str, source: &str) -> Output {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".to_string(),
        false,
        false,
        false,
        HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

fn emitted(file: &str) -> String {
    with_style_sheet(|sheet| {
        format!(
            "{}{}",
            sheet.create_css(None, false),
            sheet.create_css(Some(file), false)
        )
    })
}

fn assert_references_resolve(code: &str, css: &str) {
    // Read generated payloads, not import paths, identifiers or CSS property keys.
    let literals =
        css::utils::compile_regex(r#""(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`"#);
    let class = css::utils::compile_regex(
        r"^(?:du-)?(?:(?:(?:[a-z_][a-z0-9_]*|[a-z0-9_]*a-d|F(?:L[a-z0-9_]{0,16}|H[a-z0-9_]{16}))-)?[OR](?:L[a-z0-9_-]+|H[a-z0-9_]{16})|(?:[a-z_][a-z0-9_]*|[a-z0-9_]*a-d)-(?:[a-z_][a-z0-9_]*|[a-z0-9_]*a-d))$",
    );
    let animation = css::utils::compile_regex(r"^(?:du-)?K(?:L[a-z0-9_-]+|H[a-z0-9_]{16})$");
    let variable = css::utils::compile_regex(
        r"^---(?:du-)?S(?:[a-z_][a-z0-9_]*|U[a-z0-9_]+)-[a-z_][a-z0-9_]*(?:-[a-z_][a-z0-9_]*)?$",
    );
    let mut classes = 0;
    for literal in literals.find_iter(code) {
        let before = code[..literal.start()].trim_end();
        if before.ends_with("import") || before.ends_with("from") {
            continue;
        }
        let quoted = literal.as_str();
        let property_key = code[literal.end()..].trim_start().starts_with(':');
        for token in quoted[1..quoted.len() - 1].split_whitespace() {
            if variable.is_match(token) {
                assert!(
                    css.contains(&format!("var({token})")),
                    "unemitted variable {token}"
                );
            } else if animation.is_match(token) {
                assert!(
                    css.contains(&format!("@keyframes {token}{{")),
                    "unemitted keyframes {token}"
                );
            } else if class.is_match(token) && !property_key {
                let selector = format!(".{token}");
                assert!(
                    css.match_indices(&selector).any(|(at, _)| {
                        matches!(
                            css[at + selector.len()..].chars().next(),
                            Some('{' | ':' | '[' | '.' | '#' | ' ' | ',' | '>' | '+' | '~')
                        )
                    }),
                    "unemitted class {token}\n{css}"
                );
                classes += 1;
            }
        }
    }
    assert!(classes > 0, "fixture must generate classes: {code}");
}

#[rstest::rstest]
#[case(r#"<div className="OLcolor-vred c-a"/>"#)]
#[case(r#"<div className="OLcolor-vred du-a-d-b"/>"#)]
#[case(r#"<div className="OLcolor-vred RLcolor-vblue"/>"#)]
#[case(r#"<div className="OLcolor-vred du-RHaaaaaaaaaaaaaaaa"/>"#)]
#[case(r#"<div className="OLcolor-vred a-d-RHaaaaaaaaaaaaaaaa"/>"#)]
#[case(r#"const fade="KHaaaaaaaaaaaaaaaa"; const cls="OLcolor-vred";"#)]
#[case(r#"const fade="du-KLsfrom-e-"; const cls="OLcolor-vred";"#)]
#[case(r#"<div className="OLcolor-vred" style={{"---du-Sa-b-c":tone}}/>"#)]
#[should_panic(expected = "unemitted")]
fn reference_checker_rejects_dangling_payloads_when_another_class_resolves(#[case] code: &str) {
    // Given / When / Then: a valid class cannot hide another unresolved reference.
    assert_references_resolve(code, &format!(".{}{{color:red}}", "OLcolor-vred"));
}

#[test]
fn reference_checker_ignores_nonpayload_names_when_valid_classes_are_present() {
    // Given
    let code = r#"import "devup-ui"; import "df/devup-ui/a.css";
        const OLmissing=()=>({"font-family":"Arial"});
        export const cls="du-c-a du-OLcolor-vred";"#;
    // When / Then
    assert_references_resolve(
        code,
        &format!(
            ".{}{{padding:0}}.{}{{color:red}}",
            "du-c-a", "du-OLcolor-vred"
        ),
    );
}

#[test]
#[should_panic(expected = "fixture must generate classes")]
fn reference_checker_rejects_fixtures_when_no_class_is_generated() {
    assert_references_resolve("export const x=<div/>;", "");
}

const SOURCE_A: &str = r#"
import { Box, css, styled, keyframes } from '@devup-ui/react';
const fade = keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:1}});
const cls = css({color:'red', _hover:{color:'blue'}, p:['1px',null,'2px']});
const Card = styled('div', {display:'flex', color:'red'});
export const view = <><Box className={cls} styleOrder={1} bg="red"
 color={tone} borderColor={`${other} !important`} animation={`${fade} 1s`}
 _media={{'(hover:hover)':{color:'green'}}}/><Card/></>;
"#;

const SOURCE_B: &str = r#"
import { Box, css, keyframes } from '@devup-ui/react';
const fade = keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:1}});
const cls = css({color:'red', _hover:{color:'blue'}, p:['1px',null,'2px']});
export const view = <Box className={cls} styleOrder={2} bg="blue"
 color={differentRuntimeCode} animation={`${fade} 1s`}
 _supports={{'(display:grid)':{display:'grid'}}}/>;
"#;

#[test]
#[serial]
fn atom_references_and_output_are_stable_when_extraction_order_changes() {
    // Given
    let build = |reverse: bool| {
        setup();
        let mut sources = [("a.tsx", SOURCE_A), ("b.tsx", SOURCE_B)];
        if reverse {
            sources.reverse();
        }
        let mut output = BTreeMap::new();
        // When
        for (file, source) in sources {
            let result = compile(file, source);
            assert!(result.updated_base_style);
            output.insert(file, result.code);
        }
        // Then
        for (file, code) in &output {
            assert_references_resolve(code, &emitted(file));
        }
        let css = with_style_sheet(|sheet| sheet.create_css(None, false));
        assert!(css.contains("@layer o1,o2"), "{css}");
        assert!(css.contains("!important"));
        assert!(!css.contains("differentRuntimeCode"));
        assert!(
            !with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false))
                .contains("background:red")
        );
        (output, css, emitted("a.tsx"), emitted("b.tsx"))
    };
    assert_eq!(build(false), build(true));
    reset_build_state_internal();
}

#[test]
#[serial]
fn atom_names_and_emission_are_stable_when_debug_changes_with_prefix() {
    // Given
    let build = |debug| {
        setup();
        set_prefix(Some("du-".to_string()));
        css::debug::set_debug(debug);
        // When
        let output = compile("a.tsx", SOURCE_A);
        let css = emitted("a.tsx");
        // Then
        assert_references_resolve(&output.code, &css);
        (output.code, css)
    };
    assert_eq!(build(false), build(true));
    reset_build_state_internal();
    css::debug::set_debug(false);
}

#[test]
#[serial]
fn same_atom_stays_local_when_a_private_bucket_also_uses_it() {
    // Given
    setup();
    let source = r#"import {Box} from '@devup-ui/react'; export const x=<Box color="red"/>;"#;
    // When
    let shared = compile("a.tsx", source);
    let private = compile("private.tsx", source);
    // Then
    assert_references_resolve(&shared.code, &emitted("a.tsx"));
    assert_references_resolve(&private.code, &emitted("private.tsx"));
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false));
    assert_eq!(global.matches("color:red").count(), 1);
    assert_eq!(local.matches("color:red").count(), 1);
    assert_ne!(shared.code, private.code);
    reset_build_state_internal();
}

#[test]
#[serial]
fn compat_apis_reference_emitted_atoms_when_bucket_is_predeclared() {
    // Given
    setup();
    let sources = [
        (
            "a.tsx",
            r"import * as stylex from '@stylexjs/stylex';
          const s=stylex.create({root:{color:'red',':hover':{color:'blue'}}});
          export const x=<div {...stylex.props(s.root)}/>;",
        ),
        (
            "styles.css.ts",
            r"import {style,layer,keyframes} from '@vanilla-extract/css';
          const l=layer('cards'); const fade=keyframes({from:{opacity:0},to:{opacity:1}});
          export const card=style({'@layer':{[l]:{color:'red',animation:`${fade} 1s`}},
            selectors:{'&:hover':{color:'blue'}}});",
        ),
    ];
    // When
    for (file, source) in sources {
        let output = compile(file, source);
        // Then
        assert_references_resolve(&output.code, &emitted(file));
    }
    reset_build_state_internal();
}

#[test]
#[serial]
fn local_ordered_styles_signal_shared_layer_order_changes() {
    // Given
    setup();
    let plain = compile(
        "private.tsx",
        r#"import {Box} from '@devup-ui/react'; export const x=<Box color="red"/>;"#,
    );
    assert!(!plain.updated_base_style);
    let source = r#"import {Box} from '@devup-ui/react'; export const x=<Box color="blue" styleOrder={1}/>;"#;
    // When
    let output = compile("private.tsx", source);
    // Then
    assert!(output.updated_base_style);
    assert_references_resolve(&output.code, &emitted("private.tsx"));
    assert!(with_style_sheet(|sheet| sheet.create_css(None, false)).contains("@layer o1;"));
    assert!(!compile("private.tsx", source).updated_base_style);
    reset_build_state_internal();
}
