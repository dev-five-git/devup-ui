/// Whether an import names the package itself or one of its subpaths.
pub(crate) fn is_package(source: &str, package: &str) -> bool {
    source == package
        || source
            .strip_prefix(package)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serial_test::serial;

    use crate::{ExtractOption, ExtractStyleValue, ResolvedModule, extract, extract_with_modules};

    #[rstest]
    #[case("@devup-ui/react", "@devup-ui/react", true)]
    #[case("@devup-ui/react/compat", "@devup-ui/react", true)]
    #[case("@devup-ui/react/", "@devup-ui/react", true)]
    #[case("@devup-ui/react-values", "@devup-ui/react", false)]
    #[case("@devup-ui/reactive", "@devup-ui/react", false)]
    #[case("@devup-ui/react-values/colors", "@devup-ui/react", false)]
    #[case("@devup-ui/reac", "@devup-ui/react", false)]
    #[case("prefix/@devup-ui/react", "@devup-ui/react", false)]
    #[case("@my/ui", "@my/ui", true)]
    #[case("@my/ui/tokens", "@my/ui", true)]
    #[case("@my/ui-tokens", "@my/ui", false)]
    #[case("@my/ui?query", "@my/ui", false)]
    #[case("@my/ui#fragment", "@my/ui", false)]
    #[case("@my/ui\\tokens", "@my/ui", false)]
    #[case("@emotion/react", "@emotion/react", true)]
    #[case("@emotion/react/jsx-runtime", "@emotion/react", true)]
    #[case("@emotion/react-values", "@emotion/react", false)]
    #[case("@emotion/styled/base", "@emotion/styled", true)]
    #[case("@emotion/styled-extra", "@emotion/styled", false)]
    #[case("@stylexjs/stylex", "@stylexjs/stylex", true)]
    #[case("@stylexjs/stylex/subpath", "@stylexjs/stylex", true)]
    #[case("@stylexjs/stylex-extra", "@stylexjs/stylex", false)]
    #[case("", "@devup-ui/react", false)]
    fn package_boundary_matches_only_package_or_slash_suffix(
        #[case] source: &str,
        #[case] package: &str,
        #[case] expected: bool,
    ) {
        // Given a source and package, when comparing, then require an exact or slash-delimited boundary.
        assert_eq!(super::is_package(source, package), expected);
    }

    #[rstest]
    #[case(("@devup-ui/react", "@devup-ui/react-values"))]
    #[case(("@my/ui", "@my/ui-tokens"))]
    #[serial]
    fn package_boundary_sibling_values_are_static(
        #[case] (package, sibling): (&'static str, &'static str),
        #[values(("/shared.css.ts", "css({ color: value })"), ("/shared.tsx", "css({ color: value })"), ("/shared.tsx", "<Box color={value}/>"))]
        (filename, consumer): (&str, &str),
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let imported = if consumer.starts_with('<') {
            "Box"
        } else {
            "css"
        };
        let source = format!(
            "import {{{imported}}} from '{package}';import {{value}} from '{sibling}';export const result={consumer};"
        );
        let resolver = move |specifier: &str, _: &str| {
            (specifier == sibling).then(|| ResolvedModule {
                path: "/values.ts".to_string(),
                code: "export const value='red';".to_string(),
            })
        };
        let option = ExtractOption {
            package: package.to_string(),
            ..ExtractOption::default()
        };
        // When
        let output = extract_with_modules(filename, &source, option, false, &resolver)?;
        // Then
        assert!(output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.property == "color" && style.value == "red"
        )), "{:?}", output.styles);
        assert!(!output.code.contains("--"), "{}", output.code);
        assert_eq!(output.dependencies, vec!["/values.ts"]);
        Ok(())
    }

    #[rstest]
    #[case(("@devup-ui/react", "@devup-ui/react"))]
    #[case(("@devup-ui/react", "@devup-ui/react/styles"))]
    #[case(("@devup-ui/react", "@devup-ui/react/compat"))]
    #[case(("@my/ui", "@my/ui"))]
    #[case(("@my/ui", "@my/ui/styles"))]
    #[case(("@my/ui", "@my/ui/compat"))]
    #[serial]
    fn package_boundary_real_api_computes_styles(
        #[case] (package, api): (&str, &str),
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let source = format!(
            "import {{css}} from '{api}';const twice=(n)=>n*2;export const result=css({{width:twice(3)}});"
        );
        let option = ExtractOption {
            package: package.to_string(),
            ..ExtractOption::default()
        };
        // When
        let output = extract("/control.tsx", &source, option)?;
        // Then
        let extracts = api == package || api == format!("{package}/compat");
        assert_eq!(output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.property == "width" && style.value == "24px"
        )), extracts, "{:?}", output.styles);
        Ok(())
    }

    #[rstest]
    #[case("@devup-ui/react-values", false)]
    #[case("@devup-ui/react", true)]
    #[case("@devup-ui/react/styles", true)]
    #[case("@devup-ui/react/compat", true)]
    fn package_boundary_external_css_is_not_precomputed(
        #[case] source: &str,
        #[case] expected: bool,
    ) {
        // Given
        let code = format!(
            "import {{css}} from '{source}';const twice=(n)=>n*2;export const result=css({{width:twice(3)}});"
        );
        // When
        let computed = crate::build_time_values::has_build_time_values(
            "/control.tsx",
            &code,
            &ExtractOption::default(),
            None,
        );
        // Then
        assert_eq!(computed, expected);
    }

    #[test]
    fn package_boundary_external_css_arguments_are_left_untouched() {
        // Given
        let code = "import {css} from '@devup-ui/react-values';const twice=(n)=>n*2;export const result=css({width:twice(3)});";
        // When
        let computed = crate::build_time_values::evaluate_located(
            code,
            "/control.tsx",
            &ExtractOption::default(),
            None,
            &crate::imported_constants::Unknown::default(),
        )
        .unwrap_or_else(|error| panic!("{error:?}"));
        // Then
        assert_eq!(computed, None);
    }

    #[rstest]
    #[case("@devup-ui/react-values", true)]
    #[case("@devup-ui/react", false)]
    #[case("@devup-ui/react/styles", false)]
    #[case("@devup-ui/react/compat", false)]
    fn package_boundary_side_effect_import_is_kept(#[case] source: &str, #[case] expected: bool) {
        // Given
        let option = ExtractOption::default();
        let code = format!("import '{source}';export const value='red';");
        let unit = crate::module_loader::Unit::generated("/entry.css.ts", &code);
        let mut loader = crate::module_loader::ModuleLoader::new(None, &option);
        // When
        crate::module_loader::module_script(&unit, &mut loader, true)
            .unwrap_or_else(|error| panic!("{error}"));
        // Then
        assert_eq!(
            loader.kept_imports,
            if expected { vec![source] } else { vec![] }
        );
    }

    #[test]
    #[serial]
    fn package_boundary_sibling_css_export_is_not_stolen() -> Result<(), Box<dyn std::error::Error>>
    {
        // Given
        let source = "import {Box} from '@devup-ui/react';import {rules} from './helper';export const result=<Box className={rules} color='green'/>;";
        let resolver = |specifier: &str, _: &str| {
            match specifier {
            "./helper" => Some(ResolvedModule {
                path: "/helper.ts".to_string(),
                code: "import {css} from '@devup-ui/react-values';export const rules=css({color:'blue'});".to_string(),
            }),
            "@devup-ui/react-values" => Some(ResolvedModule {
                path: "/values.ts".to_string(),
                code: "export function css(rules){return 'external-'+rules.color;}".to_string(),
            }),
            _ => None,
        }
        };
        // When
        let output = extract_with_modules(
            "/entry.tsx",
            source,
            ExtractOption::default(),
            false,
            &resolver,
        )?;
        // Then
        assert_eq!(output.styles.len(), 1, "{:?}", output.styles);
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "color" && style.value == "green")));
        assert!(output.code.contains("rules"), "{}", output.code);
        Ok(())
    }

    #[rstest]
    #[case("@emotion/react", true)]
    #[case("@emotion/react/jsx-runtime", true)]
    #[case("@emotion/styled", true)]
    #[case("@emotion/styled/base", true)]
    #[case("@emotion/react-values", false)]
    #[case("@emotion/styled-extra", false)]
    fn package_boundary_emotion_recognition_is_unchanged(
        #[case] source: &str,
        #[case] expected: bool,
    ) {
        // Given an import specifier, when detecting Emotion, then only its package or subpaths qualify.
        assert_eq!(crate::css_prop::is_emotion(source), expected);
    }
}
