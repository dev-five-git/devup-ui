pub(super) use super::cache6_test_support::*;
pub(super) use super::counter_compiler::{after_finish, code_extract_counter};
pub(super) use extractor::compiler_policy::{
    CompilerInput, CounterCompileError, with_counter_extract,
};

pub(super) fn required<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

pub(super) fn input<'a>(file: &'a str, code: &'a str) -> CompilerInput<'a> {
    CompilerInput {
        filename: file,
        code,
        option: ExtractOption::default(),
        source_map: true,
        resolver: None,
    }
}
pub(super) fn compile(file: &str, code: &str) -> ObservedOutput {
    let output = code_extract_counter(input(file, code), false).unwrap_or_else(|e| panic!("{e}"));
    ObservedOutput::capture(&output, file)
}
pub(super) const PARENT: &str = "import {tone} from './child.css'; import {style} from '@devup-ui/react'; export const root=style({color:tone});";
pub(super) const RESERVED_CHILD: &ModuleResolver = &|_, _| {
    Some(ResolvedModule { path: "child.css.ts".into(), code: "import {style} from '@devup-ui/react'; export const tone='red'; export const unused=style({color:'magenta'});".into() })
};
pub(super) fn reserved_input() -> CompilerInput<'static> {
    let mut source = input("parent.css.ts", PARENT);
    source.resolver = Some(RESERVED_CHILD);
    source
}

#[rstest]
#[case(false, false)]
#[case(true, false)]
#[case(false, true)]
#[serial]
fn discarded_child_css_is_absent_when_resolver_visits_without_naming_demand(
    #[case] collapsed: bool,
    #[case] single: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        if collapsed {
            import_canonical_map_internal(HashMap::from([
                ("child.css.ts".into(), "delivery.css.ts".into()),
                ("parent.css.ts".into(), "delivery.css.ts".into()),
            ]));
        }
        let resolver = |_: &str, _: &str| {
            Some(ResolvedModule {
            path: "child.css.ts".into(),
            code: "import {globalStyle} from '@devup-ui/react'; export const tone='red'; globalStyle('.w31z-child-only',{color:'magenta'});".into(),
        })
        };
        let source = "import { tone } from './child.css'; import { globalStyle } from '@devup-ui/react'; globalStyle('.w31z-parent-only', { color: tone });";
        let make_input = || {
            let mut source = input("parent.css.ts", source);
            source.option.single_css = single;
            source.resolver = Some(&resolver);
            source
        };
        let observed = with_counter_extract(make_input(), |_, batch| {
            batch.consume(|v| {
                let child = v
                    .associations
                    .invocations
                    .iter()
                    .find(|i| i.raw == "child.css.ts")
                    .unwrap_or_else(|| panic!("real child invocation"));
                assert!(
                    v.associations
                        .operands
                        .iter()
                        .any(|a| a.invocation == child.id)
                );
                assert!(
                    v.associations
                        .naming
                        .iter()
                        .all(|n| n.invocation != child.id)
                );
                assert!(v.witnesses.iter().all(|w| w.invocation != child.id));
                Err::<(), _>(())
            })
        });
        assert!(matches!(observed, Err(CounterCompileError::Consumer(()))));
        // When
        let output = code_extract_counter(make_input(), false).unwrap_or_else(|e| panic!("{e}"));
        // Then
        let (global, local) = with_style_sheet(|sheet| {
            (
                sheet.create_css(None, false),
                sheet.create_css(Some(&canonical("parent.css.ts")), false),
            )
        });
        assert!(format!("{global}{local}").contains(".w31z-parent-only"));
        assert!(format!("{global}{local}").contains("color:red"));
        for css in [global, local, output.css().unwrap_or_default()] {
            assert!(!css.contains(".w31z-child-only"));
            assert!(!css.contains("magenta"));
        }
    });
}

#[rstest]
#[serial]
fn real_surfaces_agree_when_compiled_with_admitted_routes(
    #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)] fixture: usize,
    #[values(false, true)] single: bool,
    #[values(0, 1, 2)] mode: u8,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        set_prefix(Some("proof".into()));
        set_debug(mode == 1);
        css::atom_hoist::set_atom_hoist((mode == 2).then_some(2));
        import_canonical_map_internal(HashMap::from([(
            "raw/../a.tsx".into(),
            "delivery.tsx".into(),
        )]));
        css::file_routes::set_file_routes(HashMap::from([(
            "raw/../a.tsx".into(),
            std::collections::HashSet::from([0, 1]),
        )]));
        let sources = [
            "import {Box} from '@devup-ui/react'; const a=<Box color='red'/>;",
            "import {Box} from '@devup-ui/react'; const a=<Box styleOrder={flag?1:2} color={[read(),other()]} bg='red'/>;",
            "import {styled} from '@devup-ui/react'; const A=styled.div({width:state.size});",
            "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{color:'red',opacity:0},to:{color:'blue',opacity:1}}); const b=keyframes({from:{color:'red',opacity:0},to:{color:'blue',opacity:1}});",
            "import {Box} from '@devup-ui/react'; const a=<Box color={`${tone} !important`}/>;",
            "import {Box} from '@devup-ui/react'; const a=<Box typography='heading'/>;",
            "import {Box} from '@devup-ui/react'; const a=<Box styleOrder={0} color='red'/>;",
            "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'red'}}); globalCss({body:{color:'red'}});",
            "import {Box} from '@devup-ui/react'; const a=<Box color='$missing'/>;",
            "import * as stylex from '@stylexjs/stylex'; const s=stylex.create({base:{color:'red'}}); const a=stylex.props(s.base);",
            "import * as stylex from '@stylexjs/stylex'; const s=stylex.create({base:(height)=>({'height':height})}); const a=stylex.props(s.base(value));",
            "import {css} from '@devup-ui/react'; const a=css({typography:'heading',fontSize:'18px'});",
            "\u{feff}import {Box} from '@devup-ui/react';\r\nconst a=<Box color='red'/>;",
            "import styled from '@emotion/styled'; const A=styled.div({color:'red'});",
        ];
        let mut source = input("raw/../a.tsx", sources[fixture]);
        source.option.import_aliases.insert(
            "@emotion/styled".into(),
            ImportAlias::DefaultToNamed("styled".into()),
        );
        source.option.single_css = single;
        source.source_map = fixture.is_multiple_of(2);
        // Independent authentic graph observation aborts; it supplies no consumer authority.
        let observed = (fixture == 10).then(|| {
            let mut probe = input(source.filename, source.code);
            probe.option.single_css = single;
            let result = with_counter_extract(probe, |_, batch| {
                batch.consume(|v| {
                    let u = v.associations.variables.first().unwrap_or_else(|| panic!());
                    let w = v.witnesses.first().unwrap_or_else(|| panic!());
                    let n = v.associations.naming.first().unwrap_or_else(|| panic!());
                    let extractor::extract_style::compiler_receipts::ProducedData::Dynamic(d) =
                        &w.produced
                    else {
                        panic!("dynamic receipt")
                    };
                    assert!(u.route.is_none() && u.scope.is_none());
                    assert_eq!(u.receipts, vec![w.id]);
                    assert_eq!((n.receipt, &n.route), (w.id, &w.route));
                    assert_eq!(w.route.as_deref(), (!single).then_some("delivery.tsx"));
                    assert!(d.site.is_some() && d.variable_allocation.is_none());
                    let context = &d.class.context;
                    assert_eq!(context.file.is_none(), single);
                    let delivery = context.delivery.as_ref().map(|d| d.canonical.as_str());
                    assert_eq!(delivery, (!single).then_some("delivery.tsx"));
                    Err::<(), _>(d.clone())
                })
            });
            match result {
                Err(CounterCompileError::Consumer(observed)) => observed,
                Err(CounterCompileError::Compile(e)) => panic!("{e}"),
                Ok(()) => panic!("observation must abort"),
            }
        });
        // When
        let output = code_extract_counter(source, true)
            .unwrap_or_else(|e| panic!("fixture={fixture} mode={mode} single={single}: {e}"));
        // Then
        assert_eq!(output.map().is_some(), fixture.is_multiple_of(2));
        assert_eq!(css::file_map::original_id("raw/../a.tsx"), Some(0));
        assert_eq!(output.css().is_some(), fixture != 5);
        assert_ne!(encoded().len(), 0);
        if let Some(d) = observed {
            let class = &d.class.allocation.name;
            let variable = &d.variable;
            assert!(output.code().contains(class) && output.code().contains(variable));
            with_style_sheet(|s| {
                let route = (!single).then_some("delivery.tsx");
                assert_eq!(output.css(), Some(s.create_css(route, true)));
                let css = s.create_css(if mode == 2 { None } else { route }, false);
                let declaration = format!("height:var({variable})");
                assert!(css.contains(&format!(".{class}")) && css.contains(&declaration));
            });
        }
        if mode == 2 {
            assert_eq!(
                css::atom_hoist::atom_plan(),
                Some(BTreeSet::from(["delivery.tsx".into()]))
            );
        }
    });
}

#[test]
#[serial]
fn nonempty_real_js6_is_adopted_when_import_precedes_recompile() {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        let source = "import {Box,keyframes} from '@devup-ui/react'; const k=keyframes({from:{opacity:0},to:{opacity:1}}); const a=<Box color='red' width={size} typography='heading' animation={`${k} 1s`}/>;";
        let clean = compile("a.tsx", source);
        assert_ne!(with_style_sheet(|s| s.properties.len()), 0);
        assert_ne!(with_style_sheet(|s| s.keyframes.len()), 0);
        let bytes = encoded();
        let expected_maps = cache6_session::Maps::capture();
        with_style_sheet_mut(|s| s.source_ids = css::file_map::get_original_ids());
        let expected_sheet = with_style_sheet(LiveCheckpoint::capture);
        fresh();
        configure_typography();
        let (before_maps, before_sheet) = cache6_session::Restore::before();
        // When
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        // Then: equality precedes any recompile.
        assert_eq!(cache6_session::Maps::capture(), expected_maps);
        assert_eq!(with_style_sheet(LiveCheckpoint::capture), expected_sheet);
        let expected = cache6_session::Restore {
            session: Some(cache6_session::Session::installed(
                before_maps,
                before_sheet,
            )),
            ..Default::default()
        };
        cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &expected));
        let replay = compile("a.tsx", source);
        fresh();
        configure_typography();
        compile("a.tsx", source);
        assert_eq!(compile("a.tsx", source), replay);
        assert_ne!(clean, replay);
    });
}
