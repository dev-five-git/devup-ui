use super::*;

#[rstest]
#[serial]
fn genuine_raw_cache_is_silent_cold_when_version_shape_or_record_is_damaged(
    #[values(0, 1, 2, 3, 4, 5, 6, 7)] kind: u8,
) {
    with_admission(|| {
        // Given: expected output comes from undamaged real JS, never candidate data.
        let _guard = Guard::new();
        let source = "import {css} from '@devup-ui/react';const a=css({color:'red'});";
        let clean = compile("a.css.ts", source);
        let clean_maps = cache6_session::Maps::capture();
        let bytes = encoded();
        let mut value: serde_json::Value = required(serde_json::from_slice(&bytes));
        let bytes = match kind {
            0 => {
                fresh();
                required(exact_test_support::compile("current.css.ts", source));
                required(export_sheet_internal()).into_bytes()
            }
            1 | 2 => {
                value["atomNamingVersion"] = serde_json::json!(if kind == 1 { 5 } else { 7 });
                required(serde_json::to_vec(&value))
            }
            3 => b"{truncated".to_vec(),
            4 => required(String::from_utf8(bytes))
                .replacen(
                    "\"atomNamingVersion\":6",
                    "\"atomNamingVersion\":6,\"atomNamingVersion\":6",
                    1,
                )
                .into_bytes(),
            5 | 6 => {
                if kind != 6 {
                    value["properties"] = serde_json::json!({});
                }
                if kind != 5 {
                    value["evidence"]["counters"] = serde_json::json!({});
                }
                required(serde_json::to_vec(&value))
            }
            7 => {
                value["properties"]["a.css.ts"]["255"]["0"][0]["v"] = serde_json::json!("damaged");
                value["evidence"]["counters"]["D9-0"]["0"][0]["proof"]["emission"]["expansion"]["Static"]
                    [0]["Property"]["record"]["v"] = serde_json::json!("damaged");
                required(serde_json::to_vec(&value))
            }
            _ => panic!("kind"),
        };
        fresh();
        let cold = authority();
        // When
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        // Then
        assert_eq!(authority(), cold);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &cache6_session::Restore::default()));
        assert_eq!(cache_names::check(), Ok(()));
        // Given: the rejected cache leaves cold authority before a genuine compile error.
        let broken = input(
            "broken.tsx",
            "import {css} from '@devup-ui/react';\ncss`color: ${x};`;",
        );
        // When
        let error = code_extract_counter(broken, false)
            .err()
            .unwrap_or_else(|| panic!("runtime interpolation must fail"));
        // Then: error transport keeps the authored location and rolls back original registration.
        assert!(error.starts_with("broken.tsx:2:1:"), "{error}");
        assert!(error.contains("`css()` cannot use `x` at build time"));
        assert_eq!(authority(), cold);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &Default::default()));
        assert_eq!(compile("a.css.ts", source), clean);
        assert_eq!(cache6_session::Maps::capture(), clean_maps);
    });
}
