use super::retained::run;
use serde_json::json;

#[rstest::rstest]
#[case("local")]
#[case("named")]
#[case("default")]
#[serial_test::serial]
fn style_props_keep_local_capability_when_named_or_anonymous_default_is_extended(
    #[case] form: &str,
) {
    // Given: both imported producers must match the independently styled local Box.
    let definition = "styled(Box)`color: ${(p)=>p.tone}; background: ${(p)=>p.theme.fill};`";
    let body = "const Wide=styled(Base,{w:'100%'});function tick(label,value){order.push(label);return value;}function view(rest){return <Wide as='section' forwardedAs='nav' tone='green' theme={{fill:'blue'}} ref={73} p={tick('p','8px')} opacity={tick('opacity','0.5')} {...rest} />;}export {view,Wide};";
    let local =
        format!("import {{Box,styled}} from '@devup-ui/react';const Base={definition};{body}");
    let named = format!(
        "import {{Box,styled}} from '@devup-ui/react';const Base={definition};export {{Base}};"
    );
    let default =
        format!("import {{Box,styled}} from '@devup-ui/react';export default {definition};");
    let observation = r"const view=modules['/src/app.tsx'].view;const rows=[{}, {p:undefined,opacity:null}, {p:'12px',opacity:0}].map(rest=>{order.length=0;frames.length=0;const first=view(rest);const e=render(first);const backgrounds=(e.props.className||'').split(/\s+/).flatMap(c=>rules[c]&&rules[c][0]==='background'?[rules[c][1]]:[]);return [e.target,e.props.ref,e.props.as,values(e,'padding').filter(v=>v!=null),values(e,'opacity').filter(v=>v!=null),values(e,'width'),values(e,'color'),backgrounds,order.slice(),Object.hasOwn(e.props,'p'),Object.hasOwn(e.props,'opacity'),[identities(e).length,new Set(identities(e)).size],first.props.theme.fill,!Object.hasOwn(e.props,'theme')];});JSON.stringify(rows);";
    let markers = if form == "local" { 1 } else { 2 };
    let expected = json!([
        [
            "section",
            73,
            "nav",
            ["8px"],
            ["0.5"],
            ["100%"],
            ["green"],
            ["var(--fill)"],
            ["p", "opacity"],
            false,
            false,
            [markers, markers],
            "blue",
            true
        ],
        [
            "section",
            73,
            "nav",
            ["8px"],
            ["0.5"],
            ["100%"],
            ["green"],
            ["var(--fill)"],
            ["p", "opacity"],
            true,
            true,
            [markers, markers],
            "blue",
            true
        ],
        [
            "section",
            73,
            "nav",
            ["8px"],
            ["0.5"],
            ["100%"],
            ["green"],
            ["var(--fill)"],
            ["p", "opacity"],
            true,
            true,
            [markers, markers],
            "blue",
            true
        ]
    ]);
    // When: execute each complete module graph through real extraction and Boa.
    let producer = match form {
        "local" => None,
        "named" => Some((named.as_str(), "import {Base} from './base';")),
        "default" => Some((default.as_str(), "import Base from './base';")),
        _ => panic!("unknown fixture"),
    };
    let app = producer.map_or_else(
        || local.clone(),
        |(_, import)| format!("import {{styled}} from '@devup-ui/react';{import}{body}"),
    );
    let mut files = Vec::new();
    if let Some((source, _)) = producer {
        files.push(("/src/base.tsx", source));
    }
    files.push(("/src/app.tsx", app.as_str()));
    let actual = run(&files, observation);
    // Then: absolute expectations prevent two equally broken imports passing together.
    assert_eq!(actual, expected, "producer={producer:?}");
}

#[rstest::rstest]
#[case("<Base {...rest} css={{margin:1}} />", "red", Some("1px"))]
#[case(
    "<Base {...rest} css={{margin:1}} css={{margin:2}} />",
    "red",
    Some("2px")
)]
#[case("<Base {...rest} css={{margin:1}} css={null} />", "red", None)]
#[serial_test::serial]
fn imported_css_keeps_whole_replacement_when_later_css_is_known(
    #[case] element: &str,
    #[case] color: &str,
    #[case] margin: Option<&str>,
) {
    // Given: the imported base has observable inherited declarations.
    let base = "import {styled} from '@devup-ui/react';const Base=styled('div',{color:'red',p:1});export {Base};";
    let app = format!(
        "/** @jsxImportSource @emotion/react */\nimport {{Base}} from './base';function view(rest){{return {element};}}export {{view}};"
    );
    // When: execute the complete imported component and caller, with the unknown spread before css.
    let actual = run(
        &[("/src/base.tsx", base), ("/src/app.tsx", &app)],
        "const e=render(modules['/src/app.tsx'].view({}));JSON.stringify([values(e,'color'),values(e,'padding'),values(e,'margin')]);",
    );
    // Then: later known css replaces the whole prior object; null does not erase base styles.
    assert_eq!(
        actual,
        json!([[color], ["4px"], margin.into_iter().collect::<Vec<_>>()])
    );
}

#[rstest::rstest]
#[case("{margin:2}")]
#[case("null")]
#[serial_test::serial]
fn imported_css_overlap_keeps_its_located_boundary_when_a_known_spread_follows(
    #[case] replacement: &str,
) {
    // Given: the inherited overlap/spread restriction is independent of unknown-spread refusal.
    let base = "import {styled} from '@devup-ui/react';const Base=styled('div',{color:'red',p:1});export {Base};";
    let app = format!(
        "/** @jsxImportSource @emotion/react */\nimport {{Base}} from './base';function view(rest){{return <Base {{...rest}} css={{{{color:'blue'}}}} {{...{{css:{replacement}}}}} />;}}export {{view}};"
    );
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| extractor::ResolvedModule {
            path: "/src/base.tsx".to_string(),
            code: base.to_string(),
        })
    };
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When: preserve both discovery fixtures rather than broadening the existing admission policy.
    let error = match extractor::extract_with_modules(
        "/src/app.tsx",
        &app,
        extractor::ExtractOption {
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                extractor::ImportAlias::NamedToNamed,
            )]),
            ..extractor::ExtractOption::default()
        },
        false,
        &resolver,
    ) {
        Err(error) => error.to_string(),
        Ok(output) => panic!("overlap fixture must refuse: {}", output.code),
    };
    // Then: refusal stays at the explicit css that overlaps the imported base.
    assert!(error.starts_with("/src/app.tsx:2:72:"), "{error}");
    assert!(
        error.contains("`css` on `<Base>` overrides styles `Base` sets"),
        "{error}"
    );
}
