use super::*;

#[rstest]
#[case::alias(("import { Box as t, css as i } from '@devup-ui/react';", "", "i"))]
#[case::namespace(("import { Box as t } from '@devup-ui/react'; import * as ui from '@devup-ui/react';", "", "ui.css"))]
#[case::shadowed(("import { Box as t, css as i } from '@devup-ui/react';", "i", "i"))]
#[serial]
fn conditional_factory_class_links_styles_without_compiling_shadowed_helpers(
    #[case] sample: (&str, &str, &str),
    #[values(false, true)] single_css: bool,
) {
    // Given: the factory/conditional shape emitted for the built Button.
    let _state = crate::test_state::TestStateGuard::new();
    let (imports, parameter, callee) = sample;
    let source = format!(
        "{imports} import {{ jsx as o }} from 'react/jsx-runtime';
        function render(h, {parameter}) {{ return o(t, {{ className: h ? {callee}({{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}) : void 0 }}); }}"
    );
    // When
    let output = extract(
        "/src/built-button.js",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"));
    // Then: execute generated code and match selected classes to emitted styles.
    let filename = (!single_css).then_some("/src/built-button.js");
    if parameter == "i" {
        assert_eq!(output.styles.len(), 0);
        assert_eq!(
            render_observation(
                &output,
                "render(true, rules => { trace.push(rules.overflow); return 'external'; })",
                filename
            ),
            r#"[["hidden"],[]]"#
        );
    } else {
        assert_eq!(output.styles.len(), 3);
        let observed = render_observation(&output, "render(true)", filename);
        let (trace, mut declarations): (Vec<String>, Vec<String>) =
            serde_json::from_str(&observed).unwrap();
        declarations.sort();
        assert_eq!(trace.len(), 0);
        assert_eq!(declarations, ["ellipsis", "hidden", "nowrap"]);
        assert_eq!(
            render_observation(&output, "render(false)", filename),
            "[[],[]]"
        );
    }
}

mod native {
    include!("ci_factory_alias_native_tests.rs");
}
