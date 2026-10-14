use super::*;

#[rstest]
#[serial]
fn installed_session_is_restored_when_real_resolver_reentry_then_outer_fails(
    #[values(false, true)] shadow: bool,
    #[values(false, true)] inner_error: bool,
    #[values(false, true)] unwind: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        compile(
            "cached.css.ts",
            "import {css} from '@devup-ui/react'; const a=css({color:'green'});",
        );
        let bytes = encoded();
        fresh();
        configure_typography();
        let (maps, sheet) = cache6_session::Restore::before();
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        let restore = std::rc::Rc::new(cache6_session::Restore {
            session: Some(cache6_session::Session::installed(maps, sheet)),
            ..Default::default()
        });
        let before = authority();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed_calls = calls.clone();
        let expected_restore = restore.clone();
        let resolver = move |_: &str, _: &str| {
            calls.set(calls.get() + 1);
            let depth_one = authority();
            let nested_source = "import {Box} from '@devup-ui/react'; const a=<Box color='blue'/>;";
            if inner_error {
                let nested = after_finish(input("nested.tsx", nested_source), || Err("depth two"));
                assert!(matches!(
                    nested,
                    Err(CounterCompileError::Consumer(UpdateError::Output(
                        "depth two"
                    )))
                ));
                assert_eq!(authority(), depth_one);
            } else if shadow {
                let shadow = required(extractor::extract(
                    "shadow.tsx",
                    nested_source,
                    ExtractOption::default(),
                ));
                assert!(shadow.code.contains("className"));
                assert_eq!(css::file_map::original_id("shadow.tsx"), None);
            } else {
                compile("nested.tsx", nested_source);
            }
            cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, restore.as_ref()));
            Some(ResolvedModule {
                path: "values.ts".into(),
                code: "export const tone='red';".into(),
            })
        };
        let source = "import { tone } from './values'; import { css } from '@devup-ui/react'; const a=css({color:tone});";
        let mut request = input("outer.css.ts", source);
        request.resolver = Some(&resolver);
        // When
        let result = catch_unwind(AssertUnwindSafe(|| {
            after_finish(request, || {
                assert!(!unwind, "outer unwind");
                Err("outer error")
            })
        }));
        // Then
        match result {
            Ok(result) => assert!(matches!(
                result,
                Err(CounterCompileError::Consumer(UpdateError::Output(
                    "outer error"
                )))
            )),
            Err(payload) => assert_eq!(payload.downcast_ref::<&str>(), Some(&"outer unwind")),
        }
        assert_ne!(observed_calls.get(), 0);
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, expected_restore.as_ref()));
        let retry = compile(
            "retry.css.ts",
            "import {css} from '@devup-ui/react';const a=css({color:'purple'});",
        );
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        configure_typography();
        compile(
            "cached.css.ts",
            "import {css} from '@devup-ui/react'; const a=css({color:'green'});",
        );
        assert_eq!(
            compile(
                "retry.css.ts",
                "import {css} from '@devup-ui/react';const a=css({color:'purple'});"
            ),
            retry
        );
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
    });
}
