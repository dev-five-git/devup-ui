use super::*;
use css::{content_hash::FingerprintBits, style_origin::RealLocation};
use serial_test::serial;
use std::fmt::Write;

fn extract(file: &str, source: &str) -> extractor::ExtractOutput {
    extractor::extract(
        file,
        source,
        extractor::ExtractOption {
            single_css: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[serial]
fn pair_keeps_distinct_content_names_when_native_calls_share_one_boa_position() {
    // Given
    reset_build_state_internal();
    let source = "import {style} from '@devup-ui/react';\nfunction pair(a,b){return [a,b];}\nexport const pairResult=pair(style({color:'red'}),style({color:'blue'}));";
    // When
    let output = code_extract_internal(
        "pair.css.ts",
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(output.code().contains("OLcolor-vred"), "{}", output.code());
    assert!(output.code().contains("OLcolor-vblue"), "{}", output.code());
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 2));
    reset_build_state_internal();
}

#[test]
#[serial]
fn resolver_direct_and_opaque_factories_keep_identical_names_when_environment_order_reverses() {
    // Given
    let source = "import {style} from '@devup-ui/react';import {make} from './factory';export const cls=make({fontFamily:'abcdefghijklmnopqrstuvwx'});";
    let bodies = [
        "import {style} from '@devup-ui/react';export const make=(rule)=>style(rule);",
        "import {style} from '@devup-ui/react';export const make=(rule)=>((factory)=>factory)(style)(rule);",
    ];
    // When
    let mut outputs = Vec::new();
    for order in [[0, 1], [1, 0]] {
        reset_build_state_internal();
        let mut results = BTreeMap::new();
        for index in order {
            let resolver = move |specifier: &str, _: &str| {
                (specifier == "./factory").then(|| ResolvedModule {
                    path: "factory.ts".into(),
                    code: bodies[index].into(),
                })
            };
            let output = code_extract_with_modules_internal(
                "resolver.css.ts",
                source,
                "@devup-ui/react",
                "df".into(),
                true,
                false,
                false,
                HashMap::new(),
                &resolver,
            )
            .unwrap_or_else(|error| panic!("{error}"));
            results.insert(index, output.code().clone());
        }
        outputs.push(results);
    }
    // Then
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[0][&0], outputs[0][&1]);
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 1));
    reset_build_state_internal();
}

#[test]
#[serial]
fn opaque_factory_preserves_this_arguments_and_direct_eval_when_called_through_a_helper() {
    // Given
    reset_build_state_internal();
    let source = "import {style} from '@devup-ui/react';const adapt=(factory)=>function(value){if(this.tag!==value||arguments.length!==1)throw new Error('changed call');return factory({fontFamily:eval('value')});};const make=adapt(style);export const cls=make.call({tag:'abcdefghijklmnopqrstuvwx'},'abcdefghijklmnopqrstuvwx');";
    // When
    let output = code_extract_internal(
        "semantics.css.ts",
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(
        !output.code().contains("__devup_style_origin"),
        "{}",
        output.code()
    );
    with_style_sheet(|sheet| {
        assert!(
            sheet
                .names
                .values()
                .any(|claim| claim.content.contains("abcdefghijklmnopqrstuvwx")),
            "{:?}",
            sheet.names
        );
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn forced_fingerprint_collision_reports_each_real_location_tier_when_width_is_an_input() {
    // Given
    let bits = FingerprintBits::new(1).unwrap_or_else(|| panic!("valid width"));
    let modes = [("style", "OH"), ("keyframes", "KH")]
        .into_iter()
        .flat_map(|(factory, domain)| {
            ["exact", "call", "export"].map(|mode| (factory, domain, mode))
        });
    // When / Then
    for (factory, domain, mode) in modes {
        reset_build_state_internal();
        let mut source = format!(
            "import {{{factory}}} from '@devup-ui/react';\nconst identity=(factory)=>factory;const native=identity({factory});\n"
        );
        for index in 0..3 {
            let declaration = format!("{{fontFamily:'abcdefghijklmnopqrstuvwx{index}'}}");
            let rule = if factory == "keyframes" {
                format!("{{from:{declaration}}}")
            } else {
                declaration
            };
            let call = match mode {
                "exact" => format!("{factory}({rule})"),
                "call" => format!("identity({factory})({rule})"),
                "export" => format!(
                    "eval({})",
                    serde_json::to_string(&format!("native({rule})"))
                        .unwrap_or_else(|error| panic!("{error}"))
                ),
                _ => unreachable!(),
            };
            writeln!(
                source,
                "const local{index}={call};export {{local{index} as actual{index}}};"
            )
            .unwrap_or_else(|error| panic!("{error}"));
        }
        let output = extract("tiers.css.ts", &source);
        let sheet = StyleSheet::default();
        let error = sheet
            .preflight_styles_with_bits(&output.styles, ("tiers.css.ts", true), bits)
            .err()
            .unwrap_or_else(|| panic!("three distinct atoms must collide in one bit"));
        assert_eq!(sheet.names.len(), 0);
        assert_ne!(error.first.descriptor, error.second.descriptor);
        assert!(error.name.starts_with(domain), "{error}");
        let text = error.to_string();
        for claim in [&error.first, &error.second] {
            assert!(text.contains(&claim.content), "{text}");
            match (&claim.location, mode) {
                (RealLocation::Exact(origin), "exact")
                | (RealLocation::ProducedByCall(origin), "call") => {
                    let offset = source
                        .find(&origin.expression)
                        .unwrap_or_else(|| panic!("not authored: {origin:?}"));
                    let before = &source[..offset];
                    assert_eq!(origin.file, "tiers.css.ts");
                    assert_eq!(
                        origin.line,
                        before.bytes().filter(|byte| *byte == b'\n').count() + 1
                    );
                    assert_eq!(
                        origin.column,
                        before
                            .rsplit('\n')
                            .next()
                            .unwrap_or_default()
                            .chars()
                            .count()
                            + 1
                    );
                }
                (
                    RealLocation::ModuleExport {
                        file,
                        binding: Some(binding),
                    },
                    "export",
                ) => {
                    assert_eq!(file, "tiers.css.ts");
                    assert!(binding.starts_with("actual"), "{binding}");
                    assert!(source.contains(&format!("as {binding}")), "{binding}");
                    assert!(
                        text.contains("position unavailable (produced inside eval or a callback)"),
                        "{text}"
                    );
                }
                _ => panic!("wrong tier for {mode}: {claim:?}"),
            }
        }
        assert!(text.contains("change one declaration"), "{text}");
        if mode == "call" {
            assert!(
                text.contains("produced by the call at tiers.css.ts:"),
                "{text}"
            );
        }
    }
    reset_build_state_internal();
}

#[test]
#[serial]
fn cached_opaque_claims_still_reject_exact_collisions_when_no_witness_exists() {
    // Given
    reset_build_state_internal();
    let bits = FingerprintBits::new(1).unwrap_or_else(|| panic!("valid width"));
    let mut sheet = StyleSheet::default();
    let mut collision = None;
    // When
    for index in 0..3 {
        let file = format!("cached{index}.css.ts");
        let source = format!(
            "import {{style}} from '@devup-ui/react';const native=style;export const actual=eval(\"native({{fontFamily:'abcdefghijklmnopqrstuvwx{index}'}})\");"
        );
        let output = extract(&file, &source);
        match sheet.preflight_styles_with_bits(&output.styles, (&file, true), bits) {
            Ok(claims) => {
                sheet.names.extend(claims);
                let json = serde_json::to_string(&sheet.export_snapshot())
                    .unwrap_or_else(|error| panic!("{error}"));
                sheet = serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}"));
            }
            Err(error) => {
                collision = Some(error);
                break;
            }
        }
    }
    // Then
    let error = collision.unwrap_or_else(|| panic!("one-bit collision was not detected"));
    for claim in [&error.first, &error.second] {
        assert_eq!(claim.origin, None);
        assert!(
            matches!(&claim.location, RealLocation::ModuleExport { binding: Some(binding), .. } if binding == "actual"),
            "{claim:?}"
        );
    }
    reset_build_state_internal();
}
