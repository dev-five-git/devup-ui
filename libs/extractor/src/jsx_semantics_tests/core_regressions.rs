use super::*;
use serial_test::serial;

mod capture_contract;
mod order_reads;

#[test]
#[serial]
fn core_regression_when_css_call_takes_rule_text_emits_atoms_and_class() {
    let source = "import {css} from '@devup-ui/react'; const c=css('color: green; margin: 0');";
    let result = output(source);
    let mut declarations: Vec<_> = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    declarations.sort_unstable();
    assert_eq!(declarations, vec![("color", "green"), ("margin", "0")]);
    let evaluated = whole::evaluate_code(&result.code, "c");
    assert!(
        !evaluated
            .element
            .as_str()
            .required("css result must be a class string")
            .contains(':')
    );
    assert_ne!(evaluated.element, "");
}

#[test]
#[serial]
fn core_regression_when_css_template_argument_is_evaluated_emits_computed_color() {
    let source = "import {css} from '@devup-ui/react'; import {PRIMARY} from './tokens'; function darken(amount,color){return color===PRIMARY?'#112233':color} const g=css(`color: ${darken(.1,PRIMARY)};`);";
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/tokens.ts".to_string(),
            code: "export const PRIMARY='#336699';".to_string(),
        })
    };
    reset_class_map();
    reset_file_map();
    let result = extract_with_modules(
        "/consumer.tsx",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .required("resolver-backed CSS template must compile");
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "#123")), "{:?}\n{}", result.styles, result.code);
    let evaluated = whole::evaluate_code(&result.code, "g");
    assert!(
        !evaluated
            .element
            .as_str()
            .required("computed CSS result must be a class string")
            .contains(':')
    );
    assert_ne!(evaluated.element, "");
}

#[test]
#[serial]
fn core_regression_when_css_argument_is_an_external_class_keeps_it_opaque() {
    let source =
        "import {css} from '@devup-ui/react'; const c=(external)=>css(external,{color:'blue'});";
    let result = output(source);
    let evaluated = whole::evaluate_code(&result.code, "c('hover:color:red')");
    assert!(
        evaluated
            .element
            .as_str()
            .required("external class composition must remain a string")
            .contains("hover:color:red")
    );
    assert_eq!(result.styles.iter().filter(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color")).count(), 1);
}

#[test]
#[serial]
fn core_regression_when_known_component_keys_are_computed_preserves_selectors() {
    let source = "import styled from '@emotion/styled'; const Child=styled.div({color:'red'}); const Parent=styled.div({[Child]:{color:'green'},[`&:hover ${Child}`]:{color:'blue'}});";
    let result = compile_with(
        source,
        ExtractOption {
            import_aliases: HashMap::from([(
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            )]),
            ..ExtractOption::default()
        },
    )
    .required("known component selector keys must compile");
    let green = result
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Static(style) if style.value() == "green" => {
                style.selector().map(ToString::to_string)
            }
            _ => None,
        })
        .required("green child-selector atom must exist");
    let blue = result
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Static(style) if style.value() == "blue" => {
                style.selector().map(ToString::to_string)
            }
            _ => None,
        })
        .required("blue hover-child-selector atom must exist");
    assert!(green.starts_with("& ."), "{green}");
    assert_eq!(blue, green.replacen('&', "&:hover", 1));
}

#[test]
#[serial]
fn core_regression_when_component_selector_key_is_unknown_still_errors() {
    let source = "import {styled} from '@devup-ui/react'; const Parent=styled.div({[unknown]:{color:'green'}});";
    assert!(compile(source).is_err());
}
