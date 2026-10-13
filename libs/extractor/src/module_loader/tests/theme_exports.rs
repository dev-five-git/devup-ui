use super::artifacts::{active_values, exported};
use crate::{ExtractOption, ResolvedModule, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("export {base, vars};", "import {base, vars} from './producer.css';")]
#[case(
    "export {base as renamed}; export default vars;",
    "import vars, {renamed as base} from './producer.css';"
)]
#[case(
    "export default {base,vars};",
    "import data from './producer.css'; const {base,vars}=data;"
)]
#[serial]
fn stylesheet_values_preserve_export_forms_when_consumers_compose_them(
    #[case] exports: &str,
    #[case] imports: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    let producer = format!(
        "import {{style,createTheme}} from '@devup-ui/react'; const [theme,vars]=createTheme({{space:'8px'}}); const base=style({{color:'red',padding:8}}); {exports}"
    );
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/producer.css.ts".into(),
            code: producer.clone(),
        })
    };
    let code = format!(
        "import {{style}} from '@devup-ui/react'; {imports} export const button=style([base,{{color:'blue',margin:vars.space}}]);"
    );
    let output = extract_with_modules(
        "/caller.css.ts",
        &code,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    assert_eq!(
        active_values(&output, &exported(&output, "button")?),
        vec![
            ("color".into(), "blue".into()),
            ("margin".into(), "var(--space-0-1)".into()),
            ("padding".into(), "8px".into()),
        ]
    );
    Ok(())
}

#[test]
#[serial]
fn default_style_capture_avoids_author_binding_collisions() -> Result<(), Box<dyn std::error::Error>>
{
    css::file_map::reset_file_map();
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
        path: "/default.css.ts".into(),
        code: "import {style} from '@devup-ui/react'; const __default__='author'; export default style({color:'red'});".into(),
    })
    };
    let output = extract_with_modules(
        "/caller.css.ts",
        "import {style} from '@devup-ui/react';import base from './default.css';export const button=style([base,{color:'blue'}]);",
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    assert_eq!(
        active_values(&output, &exported(&output, "button")?),
        vec![("color".into(), "blue".into())]
    );
    Ok(())
}
