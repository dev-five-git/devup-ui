use super::*;

pub(super) type TestResult = Result<(), Box<dyn Error>>;

pub(super) fn setup() {
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::class_map::reset_class_map();
}

pub(super) fn option(single: bool) -> ExtractOption {
    ExtractOption {
        single_css: single,
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

pub(super) fn run(
    entry: (&str, &str),
    modules: &[(&str, &str, &str)],
    single: bool,
) -> Result<ExtractGraphOutput, Box<dyn Error>> {
    let modules: Vec<_> = modules
        .iter()
        .map(|(specifier, path, code)| (specifier.to_string(), path.to_string(), code.to_string()))
        .collect();
    extract_graph(
        entry.0,
        entry.1,
        option(single),
        true,
        Some(&move |specifier, _| {
            modules
                .iter()
                .find(|(name, _, _)| name == specifier)
                .map(|(_, path, code)| ResolvedModule {
                    path: path.clone(),
                    code: code.clone(),
                })
        }),
    )
}

pub(super) fn has(output: &ExtractOutput, property: &str, value: &str) -> bool {
    output.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(style)
        if style.property == property && style.value == value)
    })
}

pub(super) fn producer(values: (&str, &str, &str)) -> String {
    let (space, body, opacity) = values;
    format!(
        "import * as ve from '@vanilla-extract/css';let runs=0;(runs++,ve.createVar());export const [theme,vars]=(runs++,ve.createTheme({{space:'{space}'}}));export const base=(runs++,ve.style({{color:'red',padding:8}}));export const spin=(runs++,ve.keyframes({{to:{{opacity:{opacity}}}}}));(runs++,ve.globalStyle('body',{{color:'{body}'}}));export function count(){{return runs;}}export const make=ve.style;const browser=window.document;"
    )
}

pub(super) const ENTRY: &str = "import {make,base,count} from './left';import {vars,spin} from './right';export const box=make([base,{color:'blue',margin:vars.space,animationName:spin,zIndex:count()}]);const browser=window.document;";

pub(super) fn payload(output: &ExtractOutput, owner: &str, values: (&str, &str, &str)) {
    let (space, body, opacity) = values;
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Static(style)
        if style.property.starts_with("--space-") && style.value == space
            && matches!(&style.selector, Some(StyleSelector::Global(selector, raw))
                if raw == owner && selector.starts_with(".theme-"))))
    );
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Static(style)
        if style.property == "color" && style.value == body
            && matches!(&style.selector, Some(StyleSelector::Global(selector, raw))
                if raw == owner && selector == "body")))
    );
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Keyframes(frames)
        if frames.keyframes.get("to").is_some_and(|styles| styles.iter()
            .any(|style| style.property == "opacity" && style.value == opacity))))
    );
}
