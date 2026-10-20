use super::*;
use serial_test::serial;
use std::collections::HashSet;

mod lifecycle;
mod references;
mod resumption;
mod single_css;
use references::assert_references_resolve;

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
        None,
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

fn reserve(sources: &[(&str, &str)]) {
    let inputs: Vec<_> = sources
        .iter()
        .map(|(file, source)| ReservationInput {
            filename: (*file).into(),
            code: (*source).into(),
            css_dir: "df".into(),
            source_type: None,
        })
        .collect();
    reserve_class_slots_internal(
        &inputs,
        ExtractOption {
            import_aliases: HashMap::from([(
                "@vanilla-extract/css".into(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
        None,
    );
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
        let mut sources = [
            ("a.tsx", SOURCE_A),
            ("b.tsx", SOURCE_B),
            ("private.tsx", SOURCE_B),
        ];
        reserve(&sources);
        if reverse {
            sources.reverse();
        }
        let mut output = BTreeMap::new();
        // When
        for (file, source) in sources {
            let result = compile(file, source);
            if file != "private.tsx" {
                assert!(result.updated_base_style);
            }
            output.insert(file, result.code);
        }
        // Then
        for (file, code) in &output {
            assert_references_resolve(code, &emitted(file));
        }
        let css = with_style_sheet(|sheet| sheet.create_css(None, false));
        assert_eq!(get_unreserved_class_slot_count(), 0);
        assert!(css.contains("@layer o1,o2"), "{css}");
        assert!(css.contains("!important"));
        assert!(!css.contains("differentRuntimeCode"));
        assert!(
            !with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false))
                .contains("background:red")
        );
        (
            output,
            css,
            emitted("a.tsx"),
            emitted("b.tsx"),
            emitted("private.tsx"),
        )
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
        reserve(&[("a.tsx", SOURCE_A)]);
        let reserved = export_class_map_internal().unwrap_or_else(|error| panic!("{error}"));
        reserve(&[("a.tsx", SOURCE_A)]);
        assert_eq!(
            export_class_map_internal().unwrap_or_else(|error| panic!("{error}")),
            reserved
        );
        assert!(
            css::class_map::get_class_map()
                .values()
                .map(HashMap::len)
                .sum::<usize>()
                > 0
        );
        // When
        let output = compile("a.tsx", SOURCE_A);
        let css = emitted("a.tsx");
        // Then
        assert_references_resolve(&output.code, &css);
        assert_eq!(get_unreserved_class_slot_count(), 0);
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
    let shared_peer = compile("b.tsx", source);
    let private = compile("private.tsx", source);
    // Then
    assert_references_resolve(&shared.code, &emitted("a.tsx"));
    assert_references_resolve(&shared_peer.code, &emitted("b.tsx"));
    assert_references_resolve(&private.code, &emitted("private.tsx"));
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false));
    assert_eq!(global.matches("color:red").count(), 1);
    assert_eq!(local.matches("color:red").count(), 1);
    assert!(shared.code.contains("className=\"a\""), "{}", shared.code);
    assert!(
        private.code.contains("className=\"c-a\""),
        "{}",
        private.code
    );
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
