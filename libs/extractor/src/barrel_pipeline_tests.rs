use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, ImportAlias, ResolvedModule, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn resolver(specifier: &str, _: &str) -> Option<ResolvedModule> {
    match specifier {
        "./ui" => Some(ResolvedModule {
            path: "/src/ui.ts".to_string(),
            code: "export { css, Box } from '@devup-ui/react';".to_string(),
        }),
        _ => None,
    }
}

#[test]
#[serial]
fn companion_import_is_rejected_when_barrel_edits_are_needed() {
    // Given: rewriting the first import must not bypass companion validation.
    reset_class_map();
    reset_file_map();
    let source = "import { Box } from './ui';\nimport { recipe } from '@vanilla-extract/recipes';\nexport const a = <Box />;";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    // When
    let error = failure(extract_with_modules(
        "/src/app.tsx",
        source,
        option,
        false,
        &resolver,
    ));

    // Then: the original import site, not the rewritten source, is reported.
    assert!(
        error.starts_with("/src/app.tsx:2:1: `@vanilla-extract/recipes`"),
        "{error}"
    );
}

#[rstest]
#[case("import { css } from './ui';", "css")]
#[case("import * as UI from './ui';", "UI.css")]
#[serial]
fn literal_dead_declaration_keeps_its_key_when_only_barrel_edits_exist(
    #[case] import: &str,
    #[case] api: &str,
) {
    // Given: no Boa value evaluation is needed for an authored dead property.
    reset_class_map();
    reset_file_map();
    let statement = format!("export const a = {api}({{ boxOrient: 'vertical' }});");
    let source = format!("{import}\n{statement}");
    let column = position(&statement, "boxOrient") + 1;

    // When
    let error = failure(extract_with_modules(
        "/src/app.tsx",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    ));

    // Then: mapping edits alone must not snap a key diagnostic to its call.
    assert!(
        error.starts_with(&format!("/src/app.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("boxOrient"), "{error}");
}

#[rstest]
#[case("import { css } from './ui';", "css")]
#[case("import * as UI from './ui';", "UI.css")]
#[serial]
fn computed_dead_declaration_keeps_its_call_when_barrel_and_boa_edit_source(
    #[case] import: &str,
    #[case] api: &str,
) {
    // Given: the dead property exists only after evaluating the style argument.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "{import}\nfunction rules() {{ return {{ boxOrient: 'vertical' }}; }}\nexport const a = {api}(rules());"
    );

    // When
    let error = failure(extract_with_modules(
        "/src/app.tsx",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    ));

    // Then: the evaluated declaration belongs to its authored styling call.
    assert!(error.starts_with("/src/app.tsx:3:18:"), "{error}");
    assert!(error.contains("boxOrient"), "{error}");
}

#[test]
#[serial]
fn source_map_and_units_survive_barrel_namespace_alias_and_boa_layers()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: both factories require evaluation, but retain their own number units.
    reset_class_map();
    reset_file_map();
    let source = "import * as UI from './ui';\nimport * as E from '@emotion/css';\nfunction double(n) { return n * 2; }\nexport const a = UI.css({ p: double(4) });\nexport const b = E.css({ padding: double(4) });\nexport const after = 1;";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    // When
    let output = extract_with_modules("/src/app.tsx", source, option, true, &resolver)?;

    // Then: both computed atoms, dependencies and authored map coordinates survive.
    let mut padding: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            crate::ExtractStyleValue::Static(style) if style.property() == "padding" => {
                Some(style.value())
            }
            _ => None,
        })
        .collect();
    padding.sort_unstable();
    assert_eq!(padding, vec!["32px", "8px"]);
    assert_eq!(output.dependencies, vec!["/src/ui.ts"]);
    let map =
        oxc_sourcemap::SourceMap::from_json_string(output.map.as_deref().ok_or("missing map")?)?;
    assert_eq!(map.get_source_content(0), Some(source));
    let (line, generated) = output
        .code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("after"))
        .ok_or("missing generated declaration")?;
    let column = position(generated, "after");
    let token = map
        .get_tokens()
        .find(|token| {
            usize::try_from(token.get_dst_line()) == Ok(line)
                && usize::try_from(token.get_dst_col()) == Ok(column)
        })
        .ok_or("missing declaration token")?;
    assert_eq!((token.get_src_line(), token.get_src_col()), (5, 13));
    Ok(())
}
