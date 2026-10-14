use super::*;
use rstest::rstest;
use serial_test::serial;

mod cache;

fn compile(file: &str, source: &str) -> Result<Output, String> {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
}

fn fresh() {
    reset_build_state_internal();
    css::debug::set_debug(false);
    register_theme_internal(sheet::theme::Theme::default());
}

#[test]
#[serial]
fn full_output_repeats_when_100_kb_unnumbered_source_runs_in_fresh_builds() {
    // Given: 100 KB of comments before multiple dynamic assignments.
    let source = format!(
        "{}import {{Box}} from '@devup-ui/react';\nexport const View=(p)=><Box color={{p.c}} w={{p.w}}/>;",
        "// filler\n".repeat(10_240)
    );
    let run = || {
        fresh();
        let output = compile("/real/large.tsx", &source).unwrap_or_else(|error| panic!("{error}"));
        let css = with_style_sheet(|sheet| sheet.create_css(None, false));
        let names = with_style_sheet(|sheet| sheet.names.clone());
        (
            output.code,
            output.css,
            output.css_file,
            output.map,
            output.updated_base_style,
            output.dependencies,
            css,
            names,
        )
    };
    // When: the same received source runs twice without retained compiler state.
    let first = run();
    let second = run();
    // Then: complete generated output and claims repeat, with bounded emitted names.
    assert_eq!(first, second);
    let variables: Vec<_> = first
        .0
        .split('"')
        .filter(|text| text.starts_with("---SUH"))
        .collect();
    assert_eq!(variables.len(), 2);
    assert!(variables.iter().all(|name| name.len() <= 50));
    assert!(variables.iter().all(|name| first.6.contains(name)));
    let source_claims: Vec<_> = first
        .7
        .values()
        .filter(|claim| claim.content == source)
        .collect();
    assert_eq!(source_claims.len(), 1);
    assert_eq!(source_claims[0].descriptor, source.as_bytes());
    reset_build_state_internal();
}

#[rstest]
#[case("/real/a.tsx", "/real/b.tsx")]
#[case("/real/b.tsx", "/real/a.tsx")]
#[serial]
fn generated_output_ignores_filenames_when_received_source_is_identical(
    #[case] first: &str,
    #[case] second: &str,
) {
    // Given: an unnumbered source with distinct source-ordered responsive roles.
    let source = "import {Box} from '@devup-ui/react';\nexport const View=(p)=><Box h={[p.a,p.b]} color={p.c}/>;";
    fresh();
    let before = compile(first, source)
        .unwrap_or_else(|error| panic!("{error}"))
        .code();
    let css_before = with_style_sheet(|sheet| sheet.create_css(None, false));
    // When: another real filename supplies exactly the same received source.
    let after = compile(second, source)
        .unwrap_or_else(|error| panic!("{error}"))
        .code();
    // Then: names and whole compiled code agree, and no duplicate CSS is emitted.
    assert_eq!(after, before);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        css_before
    );
    let variables: std::collections::BTreeSet<_> = after
        .split('"')
        .filter(|text| text.starts_with("---SUH"))
        .collect();
    assert_eq!(variables.len(), 3);
    reset_build_state_internal();
}

#[rstest]
#[case("")]
#[case("Mixed-SUH-prefix-")]
#[serial]
fn full_generated_output_agrees_when_received_source_uses_lf_crlf_or_bom(#[case] prefix: &str) {
    // Given: equivalent received text with runtime branches and responsive roles.
    let lf = "import {Box} from '@devup-ui/react';\nexport const View=(p)=><Box w={p.ok?p.a:p.b} h={[p.h,p.more]} color={p.c}/>;\n";
    let sources = [
        lf.to_string(),
        lf.replace('\n', "\r\n"),
        format!("\u{feff}{lf}"),
        format!("\u{feff}{}", lf.replace('\n', "\r\n")),
    ];
    // When: each normalization variant runs through the actual native binding path.
    let outputs: Vec<_> = sources
        .iter()
        .map(|source| {
            fresh();
            css::set_prefix(Some(prefix.into()));
            let output =
                compile("/real/normalized.tsx", source).unwrap_or_else(|error| panic!("{error}"));
            (
                output.code,
                output.css,
                output.css_file,
                output.map,
                output.updated_base_style,
                output.dependencies,
                with_style_sheet(|sheet| sheet.create_css(None, false)),
            )
        })
        .collect();
    // Then: full generated JS/CSS/output metadata are byte-identical, not just names.
    for output in &outputs[1..] {
        assert_eq!(output, &outputs[0]);
    }
    reset_build_state_internal();
}
