use super::*;

#[test]
#[serial]
fn partial_boa_evaluation_keeps_original_error_location() {
    let error = compile("import * as E from '@emotion/css';\nfunction double(n){return n*2}\nexport const a=E.css({padding:double(4),color:unknown});").unwrap_err();
    assert!(error.contains("namespace.tsx:3:16:"), "{error}");
    assert!(error.contains("unknown"), "{error}");
    assert!(!error.contains("__emotion_"), "{error}");
}

#[test]
#[serial]
fn source_map_preserves_original_source_when_namespace_and_boa_edit_it() {
    let code = "import * as E from '@emotion/css';\nfunction double(n){return n*2}\nexport const a=E.css({padding:double(4)});";
    reset_class_map();
    reset_file_map();
    let output = extract_with_source_map(
        "namespace.tsx",
        code,
        ExtractOption {
            import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
        true,
        None,
    )
    .unwrap();
    let map: serde_json::Value = serde_json::from_str(output.map.as_deref().unwrap()).unwrap();
    assert_eq!(map["sourcesContent"][0], code);
    assert_eq!(map["sources"][0], "namespace.tsx");
}

fn written_position(output: &ExtractOutput, needle: &str) -> (u32, u32) {
    let map = oxc_sourcemap::SourceMap::from_json_string(output.map.as_deref().unwrap()).unwrap();
    let (line, generated) = output
        .code
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .unwrap();
    let column = generated.find(needle).unwrap();
    let token = map
        .get_tokens()
        .find(|token| token.get_dst_line() as usize == line && token.get_dst_col() as usize == column)
        .unwrap();
    (token.get_src_line(), token.get_src_col())
}

#[test]
#[serial]
fn namespace_only_edits_keep_a_map_to_the_original_when_nothing_is_extracted() {
    // Given: the import is removed, so every later line moves up
    let code = "import * as E from '@emotion/css';\nconst N=E;\nthrow new Error('boom');\nexport const after=1;";
    reset_class_map();
    reset_file_map();

    // When
    let output = compile(code).unwrap();

    // Then
    let map = oxc_sourcemap::SourceMap::from_json_string(output.map.as_deref().unwrap()).unwrap();
    assert_eq!(map.get_source_content(0), Some(code));
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert_eq!(written_position(&output, "N ="), (1, 6));
    assert_eq!(written_position(&output, "throw"), (2, 0));
    assert_eq!(written_position(&output, "after"), (3, 13));
}

#[test]
#[serial]
fn longer_replacements_keep_later_positions_when_namespace_stays_for_runtime_reads() {
    // Given: the destructured source grows into an object of the kept members
    let code = "import * as E from '@emotion/css';\nconst {flush = 1} = E;\nexport const after = flush;";
    reset_class_map();
    reset_file_map();

    // When
    let output = compile(code).unwrap();

    // Then
    assert!(output.code.contains("@emotion/css"), "{}", output.code);
    assert_eq!(written_position(&output, "after"), (2, 13));
}

#[test]
#[serial]
fn namespace_only_edits_skip_the_map_when_it_is_not_requested() {
    // Given
    let code = "import * as E from '@emotion/css';\nconst N=E;\nthrow new Error('boom');";
    reset_class_map();
    reset_file_map();

    // When
    let output = extract_without_source_map(
        "namespace.tsx",
        code,
        ExtractOption {
            import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .unwrap();

    // Then
    assert_eq!(output.map, None);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn error_text_restores_generated_names_when_only_comments_and_strings_mention_them() {
    for source in [
        "// __emotion_cx_2\nexport const a=E.cx`a b`;",
        "const t='__emotion_cx_3';\nexport const a=E.cx`a b`;",
    ] {
        // Given: no binding or reference of the program has the generated name
        // When
        let error = compile(&format!("import * as E from '@emotion/css';\n{source}")).unwrap_err();

        // Then
        assert!(error.contains("E.cx`a b`"), "{error}");
        assert!(!error.contains("__emotion_cx_"), "{error}");
    }
}

#[test]
#[serial]
fn disabled_emotion_alias_preserves_calls_when_other_styles_are_evaluated() {
    reset_class_map();
    reset_file_map();
    let output = extract("namespace.tsx", "import {css as devup} from '@devup-ui/react'; import {css as emotion} from '@emotion/css'; function double(n){return n*2} export const a=devup({padding:double(4)}); export const b=emotion({padding:8});", ExtractOption::default()).unwrap();
    assert!(output.code.contains("padding: 8"), "{}", output.code);
    assert!(output.code.contains("emotion({"), "{}", output.code);
}
