use rstest::rstest;
use serial_test::serial;

use super::consumer_support::{exact, located_failure, modules, owner};
use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case(
    "import {createTheme,globalStyle} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});\nglobalStyle('body',{color:window.name});",
    "vars.space",
    "window.name"
)]
#[case(
    "import {createTheme} from '@vanilla-extract/css';\nexport const [theme,vars]=createTheme({space:window.name});",
    "vars.space",
    "window.name"
)]
#[case(
    "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});\nexport const COLORS={color:'blue',unused:window.name};",
    "({...COLORS}).color",
    "window.name"
)]
#[case(
    "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space:space()});",
    "vars.space",
    "window.name"
)]
#[serial]
fn required_unknowns_error_at_original_owner_when_only_devup_styling_consumes_them(
    #[case] producer: &str,
    #[case] read: &str,
    #[case] unknown: &str,
) -> TestResult {
    // Given
    reset();
    let tokens = "export function space(){return window.name;}const browser=window.document;";
    let (path, original) = if producer.contains("space()") {
        ("/tokens.ts", tokens)
    } else {
        ("/failed-producer.ts", producer)
    };
    let place = crate::locate(
        path,
        original,
        original.find(unknown).ok_or("fixture unknown missing")?,
    );
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{vars,COLORS}} from './producer';export const box=css({{color:{read}}});"
    );
    // When
    let result = run(
        "/required.ts",
        &source,
        &[
            ("./producer", "/failed-producer.ts", producer),
            ("./tokens", "/tokens.ts", tokens),
        ],
    );
    // Then: Err exposes no successful code, CSS, or readback publication.
    located_failure(
        result,
        &place,
        &format!("cannot use `{unknown}` at build time"),
    );
    Ok(())
}

#[test]
#[serial]
fn narrow_color_succeeds_when_whole_color_record_would_require_browser_data() -> TestResult {
    // Given
    reset();
    let expected = owner("/producer.ts")?;
    // When
    let output = run(
        "/narrow.ts",
        "import {css} from '@devup-ui/react';import {vars,COLORS} from './producer';export const box=css({margin:vars.space,color:COLORS.fg});",
        &modules("/producer.ts"),
    )?;
    // Then
    exact(&output, &expected);
    assert!(has_static(&output, "color", "blue"));
    Ok(())
}

#[test]
#[serial]
fn rewritten_jsx_unknown_is_located_when_crlf_unicode_precede_both_barrels() -> TestResult {
    // Given
    reset();
    let source = concat!(
        "const 한글='😀';\r\nimport {Box} from './ui';\r\nimport {make} from './api';\r\nconst ignored=window.document,native=make({",
        "color:missing});\r\nexport const view=<Box color='blue' onClick={()=>document.title}>{window.name}</Box>;"
    );
    let place = crate::locate(
        "/mapped.tsx",
        source,
        source.find("missing").ok_or("fixture missing")?,
    );
    let graph = [
        ("./ui", "/ui.ts", "export {Box} from '@devup-ui/react';"),
        (
            "./api",
            "/api.ts",
            "export {style as make} from '@vanilla-extract/css';",
        ),
    ];
    // When
    let result = run("/mapped.tsx", source, &graph);
    // Then
    located_failure(result, &place, "cannot use `missing` at build time");
    Ok(())
}

#[test]
#[serial]
fn surviving_handler_mapping_uses_original_source_when_both_barrels_are_rewritten() -> TestResult {
    // Given
    reset();
    let source = "const 한글='😀';\r\nimport {Box} from './ui';\r\nimport {make} from './api';\r\nimport {vars} from './producer';\r\nconst native=make({color:'blue'});\r\nexport const view=<Box m={vars.space} onClick={()=>document.title}>{window.name}</Box>;";
    let mut graph = modules("/producer.ts").to_vec();
    graph.extend([
        ("./ui", "/ui.ts", "export {Box} from '@devup-ui/react';"),
        (
            "./api",
            "/api.ts",
            "export {style as make} from '@vanilla-extract/css';",
        ),
    ]);
    let graph: Vec<_> = graph
        .into_iter()
        .map(|(name, path, code)| (name.to_string(), path.to_string(), code.to_string()))
        .collect();
    // When
    let output = crate::extract_with_modules(
        "/mapped.tsx",
        source,
        super::option(),
        true,
        &move |specifier, _| {
            graph.iter().find_map(|(name, path, code)| {
                (name == specifier).then(|| crate::ResolvedModule {
                    path: path.clone(),
                    code: code.clone(),
                })
            })
        },
    )?;
    // Then
    let map = oxc_sourcemap::SourceMap::from_json_string(
        output.map.as_deref().ok_or("ordinary JSX map missing")?,
    )?;
    assert_eq!(map.get_source_content(0), Some(source));
    let (line, generated) = output
        .code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("document.title"))
        .ok_or("handler missing")?;
    let column = generated
        .find("document")
        .ok_or("generated handler missing")?;
    let line = u32::try_from(line)?;
    let column = u32::try_from(column)?;
    let token = map
        .get_tokens()
        .find(|token| token.get_dst_line() == line && token.get_dst_col() == column)
        .ok_or("handler mapping missing")?;
    let original = source
        .lines()
        .nth(usize::try_from(token.get_src_line())?)
        .ok_or("original handler line missing")?;
    assert!(
        original[usize::try_from(token.get_src_col())?..].starts_with("document.title"),
        "{original}"
    );
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    assert!(has_static(&output, "color", "blue"));
    Ok(())
}
