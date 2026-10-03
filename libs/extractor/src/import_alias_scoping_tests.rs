use super::*;

fn aliases() -> HashMap<String, ImportAlias> {
    HashMap::from([
        (EMOTION_REACT.into(), ImportAlias::NamedToNamed),
        (
            "@emotion/styled".into(),
            ImportAlias::DefaultToNamed("styled".into()),
        ),
        (
            "styled-components".into(),
            ImportAlias::DefaultToNamed("styled".into()),
        ),
        ("@vanilla-extract/css".into(), ImportAlias::NamedToNamed),
    ])
}

#[test]
fn numbers_stay_numeric_when_api_bindings_are_shadowed() {
    for (import, name, call) in [
        (
            "import { css } from '@emotion/react';",
            "css",
            "css({ top: 2 })",
        ),
        (
            "import { keyframes } from 'styled-components';",
            "keyframes",
            "keyframes({ from: { top: 2 } })",
        ),
        (
            "import { style as s } from '@vanilla-extract/css';",
            "s",
            "s({ top: 2 })",
        ),
        (
            "import { globalStyle } from '@vanilla-extract/css';",
            "globalStyle",
            "globalStyle('body', { top: 2 })",
        ),
        (
            "import styled from '@emotion/styled';",
            "styled",
            "styled.div({ top: 2 })",
        ),
        (
            "import * as styled from 'styled-components';",
            "styled",
            "styled('div').attrs({})({ top: 2 })",
        ),
    ] {
        let code = format!("{import}\n{call}; function local({name}) {{ {call}; }}");
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(
            output.contains(&format!("function local({name}) {{ {call}; }}")),
            "{output}"
        );
        assert!(output.contains("top: \"2px\""), "{output}");
    }
}

#[test]
fn global_styles_stay_numeric_when_component_binding_is_shadowed() {
    let code = "import { Global as G } from '@emotion/react';\nconst real = <G styles={{ top: 2 }} />; function local(G) { return <G styles={{ top: 2 }} />; }";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert!(output.contains("const real = <G styles={{ top: \"2px\" }} />"));
    assert!(output.contains("function local(G) { return <G styles={{ top: 2 }} />; }"));
}

#[test]
fn jsx_props_stay_numeric_when_factory_binding_is_shadowed() {
    for import in [
        "import { jsx as h } from '@emotion/react';",
        "import { jsx as h } from 'react/jsx-runtime';",
    ] {
        let code = format!(
            "{import}\nconst real = h('div', {{ css: {{ top: 2 }} }}); function local(h) {{ return h('div', {{ css: {{ top: 2 }} }}); }}"
        );
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(output.contains("const real = h('div', { css: { top: \"2px\" } })"));
        assert!(output.contains("function local(h) { return h('div', { css: { top: 2 } }); }"));
    }
}

#[test]
fn devup_css_props_stay_numeric_when_component_root_is_shadowed() {
    for (import, root, tag, ty) in [
        ("import { Box as B } from '@devup-ui/react';", "B", "B", "B"),
        (
            "import * as UI from '@devup-ui/react';",
            "UI",
            "UI.Box",
            "UI.Box",
        ),
    ] {
        let code = format!(
            "{import}\nimport {{ jsx as h }} from 'react/jsx-runtime';\nconst real = <{tag} css={{{{ top: 2 }}}} />; const made = h({ty}, {{ css: {{ top: 2 }} }}); function local({root}) {{ return [<{tag} css={{{{ top: 2 }}}} />, h(({ty}), {{ css: {{ top: 2 }} }})]; }}"
        );
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(output.contains(&format!("function local({root}) {{ return [<{tag} css={{{{ top: 2 }}}} />, h(({ty}), {{ css: {{ top: 2 }} }})]; }}")), "{output}");
        assert_eq!(output.matches("\"2px\"").count(), 2, "{output}");
    }
}

#[test]
fn class_names_css_uses_child_binding_identity_when_nested_locals_shadow_it() {
    for (child, pixels) in [
        (
            "({ cx, css: rule }) => <>{rule({ top: 2 })}{((rule) => rule({ top: 3 }))(other)}<C>{({ css: rule }) => rule({ top: 4 })}</C>{rule({ top: 5 })}</>",
            3,
        ),
        (
            "function ({ css: rule }) { return <>{rule({ top: 2 })}{((rule) => rule({ top: 3 }))(other)}</>; }",
            1,
        ),
    ] {
        let code = format!(
            "import {{ ClassNames as C }} from '@emotion/react';\nconst real = <C>{{{child}}}</C>; function local(C) {{ return <C>{{({{ css: rule }}) => rule({{ top: 6 }})}}</C>; }}"
        );
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(output.contains("rule({ top: \"2px\" })"), "{output}");
        assert!(output.contains("rule({ top: 3 })"), "{output}");
        assert!(output.contains("rule({ top: 6 })"), "{output}");
        assert_eq!(output.matches("px\"").count(), pixels, "{output}");
    }
}

#[test]
fn type_only_bindings_do_not_register_number_apis() {
    for import in [
        "import type { css, Global, ClassNames, jsx } from '@emotion/react';",
        "import { type css, type Global, type ClassNames, type jsx } from '@emotion/react';",
        "import type styled from '@emotion/styled';",
        "import type * as styled from 'styled-components';",
    ] {
        let body = "css({ top: 2 }); styled.div({ top: 2 }); jsx('div', { css: { top: 2 } }); const a = <Global styles={{ top: 2 }} />; const b = <ClassNames>{({ css }) => css({ top: 2 })}</ClassNames>;";
        let code = format!("{import}\n{body}");
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(output.ends_with(body), "{output}");
    }
}

#[test]
fn named_styled_binding_pixelifies_chains_without_touching_block_locals() {
    let code = "import { styled as s } from 'styled-components';\nconst real = s('div').withConfig({}).attrs({})({ top: 2 }); { const s = other; s.div({ top: 3 }); }";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert!(output.contains("attrs({})({ top: \"2px\" })"));
    assert!(output.ends_with("{ const s = other; s.div({ top: 3 }); }"));
}

#[test]
fn malformed_class_names_children_do_not_register_css_calls() {
    for child in [
        "text",
        "{other}",
        "{({ css }) => css({ top: 2 })}{other}",
        "{() => css({ top: 2 })}",
        "{(rules) => css({ top: 2 })}",
        "{({ cx }) => css({ top: 2 })}",
        "{({ css, unknown }) => css({ top: 2 })}",
        "{({ css = other }) => css({ top: 2 })}",
    ] {
        let code = format!(
            "import {{ ClassNames as C }} from '@emotion/react';\nconst view = <C>{child}</C>;"
        );
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert!(
            output.ends_with(&format!("const view = <C>{child}</C>;")),
            "{output}"
        );
    }
}

#[test]
fn type_only_devup_components_do_not_take_css_props() {
    for import in [
        "import type { Box as B } from '@devup-ui/react';",
        "import { type Box as B } from '@devup-ui/react';",
    ] {
        let code = format!("{import}\nconst view = <B css={{{{ top: 2 }}}} />;");
        let output = transform_import_aliases(&code, "test.tsx", "@devup-ui/react", &aliases());
        assert_eq!(output, code);
    }
}

#[test]
fn number_edits_map_back_to_original_offsets_when_imports_are_rewritten() {
    let code = "import { css as c } from '@emotion/react';\nc({ top: 23 }); function local(c) { c({ top: 24 }); }";
    let output =
        transform_import_aliases_with_edits(code, "test.tsx", "@devup-ui/react", &aliases());
    let Some(pixel) = output.code.find("\"23px\"") else {
        panic!("missing pixelified number: {}", output.code);
    };
    let Some(original_pixel) = code.find("23") else {
        panic!("missing original number");
    };
    assert_eq!(source_offset(&output.edits, pixel + 3), original_pixel);
    let Some(local) = output.code.find("24") else {
        panic!("missing local number: {}", output.code);
    };
    let Some(original_local) = code.find("24") else {
        panic!("missing original local number");
    };
    assert_eq!(source_offset(&output.edits, local), original_local);
}

#[test]
fn devup_root_guards_follow_nested_members_but_not_other_element_types() {
    let code = "import * as UI from '@devup-ui/react'; import { jsx as h } from 'react/jsx-runtime'; const real = [<UI.parts.Box css={{ top: 2 }} />, h(UI['Box'], { css: { top: 2 } })]; const other = [<this.Box css={{ top: 3 }} />, h(other(), { css: { top: 3 } })]; function local(UI) { return h(UI['Box'], { css: { top: 3 } }); }";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert_eq!(output.matches("\"2px\"").count(), 2, "{output}");
    assert_eq!(output.matches("top: 3").count(), 3, "{output}");
}

#[test]
fn inline_types_keep_modifiers_when_other_specifiers_are_retained() {
    let code = "import { styleVariants, type style as Rule, style as s, type keyframes } from '@vanilla-extract/css';\ns({ top: 2 }); Rule({ top: 3 }); keyframes({ from: { top: 4 } });";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert_eq!(
        output,
        "import { css as s } from '@devup-ui/react'; import { styleVariants, type style as Rule, type keyframes } from '@vanilla-extract/css';\ns({ top: \"2px\" }); Rule({ top: 3 }); keyframes({ from: { top: 4 } });"
    );
}

#[test]
fn nested_member_factory_css_uses_the_imported_namespace_binding() {
    let code = "import * as UI from '@devup-ui/react'; import { jsx as h } from 'react/jsx-runtime';\nconst real = h((UI.parts.Box), { css: { top: 2 } }); function local(UI) { return h((UI.parts.Box), { css: { top: 3 } }); }";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert_eq!(output, code.replace("top: 2", "top: \"2px\""));
}

#[test]
fn global_styles_rewrite_conditional_rules_without_rewriting_shadowed_styles() {
    let code = "import { Global as G } from '@emotion/react';\nconst real = <G styles={flag ? { top: 2 } : { left: 3 }} css={flag ? { right: 4 } : { bottom: 5 }} />; function local(G) { return <G styles={flag ? { top: 6 } : { left: 7 }} />; }";
    let output = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases());
    assert!(output.contains("styles={flag ? { top: \"2px\" } : { left: \"3px\" }} css={flag ? { right: \"4px\" } : { bottom: \"5px\" }}"), "{output}");
    assert!(
        output.ends_with(
            "function local(G) { return <G styles={flag ? { top: 6 } : { left: 7 }} />; }"
        ),
        "{output}"
    );
}
