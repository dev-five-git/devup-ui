//! Emotion renders `styled(A)` on the base `A` renders, without the class
//! `A`'s `target` option gives it, while `A.withComponent(tag)` keeps it:
//! compiled modules render the same classes, wherever `A` comes from

use super::namespace_module_execution as modules;
use rstest::rstest;
use serial_test::serial;

const BASE: &str = "import { styled } from '@devup-ui/react';\nexport const A = styled('div', { target: 'tA' })({ color: 'red' });";

const LINK: &str = "export const Link = (props) => <a {...props} />;";

const LINK_BASE: &str = "import { styled } from '@devup-ui/react';\nimport { Link } from './link.tsx';\nexport const A = styled(Link, { target: 'tA' })({ color: 'red' });";

const BINDING_BASE: &str = "import { styled } from '@devup-ui/react';\nlet tone = 'red';\nexport const setTone = (next) => { tone = next; };\nexport const A = styled('div', { target: 'tA' })({ color: tone });";

const DYNAMIC_BASE: &str = "import { styled } from '@devup-ui/react';\nimport { Link } from './link.tsx';\nexport const A = styled(Link, { target: 'tA' })(({ $c }) => ({ color: $c }));";

const PLAIN_BASE: &str =
    "export const A = (props) => <section {...props} className={`tA ${props.className}`} />;";

const EXT: &str = "import { styled } from '@devup-ui/react';\nimport { A } from './base.tsx';\nexport const B = styled(A)({ width: '100%' });\nexport const C = styled(A, { target: 'tC' })({ width: '100%' });\nexport const D = styled(B)({ height: '1px' });\nexport const App = () => [<A />, <B />, <C />, <D />];";

const SAME: &str = "import { styled } from '@devup-ui/react';\nexport const A = styled('div', { target: 'tA' })({ color: 'red' });\nexport const B = styled(A)({ width: '100%' });\nexport const C = styled(A, { target: 'tC' })({ width: '100%' });\nexport const D = styled(B)({ height: '1px' });\nexport const W = A.withComponent('span');\nexport const App = () => [<A />, <B />, <C />, <D />, <W />];";

const OBSERVE: &str = "import {App} from '/t/ext.tsx';globalThis.observed=App().map(e=>{const r=render(e);const c=r.props.className.split(' ');return [r.target,c.includes('tA'),c.includes('tC')];});";

/// What `A`, `B = styled(A)`, `C = styled(A, { target: 'tC' })` and
/// `D = styled(B)` render, as `[element, has tA, has tC]`
fn emotion(element: &str) -> serde_json::Value {
    serde_json::json!([
        [element, true, false],
        [element, false, false],
        [element, false, true],
        [element, false, false]
    ])
}

#[rstest]
#[case::cross_file(&[("/t/base.tsx", BASE)], "div")]
#[case::runtime_base(&[("/t/link.tsx", LINK), ("/t/base.tsx", LINK_BASE)], "a")]
#[case::module_binding(&[("/t/base.tsx", BINDING_BASE)], "div")]
#[case::dynamic(&[("/t/link.tsx", LINK), ("/t/base.tsx", DYNAMIC_BASE)], "a")]
#[serial]
fn extension_when_base_is_imported_renders_without_its_target(
    #[case] base: &[(&str, &str)],
    #[case] element: &str,
) {
    // Given: an Emotion component with a target, imported by the extensions
    let mut files = base.to_vec();
    files.push(("/t/ext.tsx", EXT));
    // When: the modules compile together and render
    let observed = modules::execute(&modules::compile(&files, false), OBSERVE);
    // Then: only the component its target belongs to renders it
    assert_eq!(observed, emotion(element));
}

#[test]
#[serial]
fn extension_when_base_is_unknown_to_the_build_renders_without_its_target() {
    // Given: the extensions compiled without the module defining their base
    let mut outputs = modules::compile(&[("/t/base.tsx", BASE)], false);
    outputs.extend(modules::compile(&[("/t/ext.tsx", EXT)], false));
    // When: the modules render together
    let observed = modules::execute(&outputs, OBSERVE);
    // Then: the base holds itself without its target, as Emotion's base
    assert_eq!(observed, emotion("div"));
}

#[test]
#[serial]
fn extension_when_base_is_in_the_same_file_renders_without_its_target() {
    // Given: the base, its extensions and `withComponent` in one file
    let outputs = modules::compile(&[("/t/ext.tsx", SAME)], false);
    // When: they render
    let observed = modules::execute(&outputs, OBSERVE);
    // Then: extensions drop the target, `withComponent` keeps it
    let mut expected = emotion("div");
    expected
        .as_array_mut()
        .unwrap_or_else(|| panic!("expected array"))
        .push(serde_json::json!(["span", true, false]));
    assert_eq!(observed, expected);
}

#[test]
#[serial]
fn extension_when_base_is_not_styled_renders_its_classes() {
    // Given: a plain component giving its own class, as Emotion renders it
    let files = [("/t/base.tsx", PLAIN_BASE), ("/t/ext.tsx", EXT)];
    // When: the extensions render it
    let observed = modules::execute(&modules::compile(&files, false), OBSERVE);
    // Then: its own class stays, as it does under Emotion
    assert_eq!(
        observed,
        serde_json::json!([
            ["section", true, false],
            ["section", true, false],
            ["section", true, true],
            ["section", true, false]
        ])
    );
}
