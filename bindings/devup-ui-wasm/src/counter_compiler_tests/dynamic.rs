use super::*;

#[rstest]
#[case(
    "import {styled} from '@devup-ui/react'; const A=styled.div({width:state.size});",
    "let reads=0; const state={get size(){reads++;return '13px'}};",
    "const created=reads; const first=A({}); const second=A({}); return [created,reads,first.className===second.className];",
    "[1,1,true]"
)]
#[case(
    "import {Box} from '@devup-ui/react'; let reads=0; const value=()=>{reads++;return 'red'}; const a=<Box color={value()}/>;",
    "",
    "return [reads,Object.values(a.style)];",
    "[1,[\"red\"]]"
)]
#[serial]
fn generated_js_reads_once_when_executed_by_bun(
    #[case] source: &str,
    #[case] setup: &str,
    #[case] assertion: &str,
    #[case] expected: &str,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let output = code_extract_counter(input("eval.tsx", source), false)
            .unwrap_or_else(|e| panic!("{e}"));
        let driver = r#"const [code,setup,check]=process.argv.slice(-3);const React={createElement:(type,props)=>({type,...props}),forwardRef:f=>props=>f(props,null)};const source=code.replace(/import\s*\{\s*forwardRef\s+as\s+(\w+)\s*\}\s*from\s*['\"]react['\"];?/g,'const $1=React.forwardRef;').replace(/import[\s\S]*?;/g,'');const t=new Bun.Transpiler({loader:'tsx',tsconfig:JSON.stringify({compilerOptions:{jsx:'react',jsxFactory:'React.createElement'}})});console.log(JSON.stringify(new Function('React',setup+t.transformSync(source)+check)(React)));"#;
        // When
        let evaluated = std::process::Command::new("bun")
            .args(["--eval", driver, &output.code(), setup, assertion])
            .output()
            .unwrap_or_else(|e| panic!("Bun launch: {e}"));
        // Then
        assert!(
            evaluated.status.success(),
            "{}",
            String::from_utf8_lossy(&evaluated.stderr)
        );
        assert_eq!(
            String::from_utf8(evaluated.stdout)
                .unwrap_or_else(|e| panic!("Bun UTF8: {e}"))
                .trim(),
            expected
        );
    });
}

#[rstest]
#[serial]
fn numeric_roles_and_importance_are_exact_when_responsive_assignments_are_compiled(
    #[values(false, true)] important: bool,
    #[values(0, 1, 2)] mode: u8,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        set_prefix(Some("role".into()));
        set_debug(mode == 1);
        set_atom_hoist((mode == 2).then_some(2));
        let value = if important {
            "`${tone} !important`"
        } else {
            "tone"
        };
        let source = format!(
            "import {{Box}} from '@devup-ui/react';const a=<Box color={{[{value},{value}]}}/>;"
        );
        let observed = with_counter_extract(input("roles.tsx", &source), |_, batch| {
            batch.consume(|v| {
                let mut variables = Vec::new();
                for witness in v.witnesses {
                    let extractor::extract_style::compiler_receipts::ProducedData::Dynamic(d) =
                        &witness.produced
                    else {
                        panic!("dynamic")
                    };
                    let site = d.site.as_ref().unwrap_or_else(|| panic!("numeric site"));
                    assert_eq!(site.file, SourceFile::D9(0));
                    assert_eq!(d.important, important);
                    assert_eq!(d.variable_allocation, None);
                    let at = source
                        .find(&format!("[{value}"))
                        .unwrap_or_else(|| panic!("input offset"));
                    assert_eq!(site.at, at);
                    variables.push((
                        site.role,
                        d.variable.clone(),
                        d.class.allocation.name.clone(),
                    ));
                }
                assert_eq!(
                    variables.iter().map(|v| v.0).collect::<Vec<_>>(),
                    vec![0, 1]
                );
                Err::<(), _>(variables)
            })
        });
        let expected = match observed {
            Err(CounterCompileError::Consumer(v)) => v,
            Err(CounterCompileError::Compile(e)) => panic!("{e}"),
            Ok(()) => panic!("abort"),
        };
        // When
        let output = required(code_extract_counter(input("roles.tsx", &source), false));
        // Then
        let css = with_style_sheet(|s| s.create_css(Some("roles.tsx"), false));
        for (_, variable, class) in expected {
            assert!(output.code().contains(&variable) && output.code().contains(&class));
            assert!(css.contains(&format!(
                "color:var({variable}){}",
                if important { " !important" } else { "" }
            )));
        }
    });
}

#[test]
#[serial]
fn insertion_uses_length_slot_when_real_seeded_namespace_has_an_existing_record() {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let red = FAMILY_SOURCES[0];
        let blue = "import {css} from '@devup-ui/react';const a=css({color:'blue'});";
        compile("a.tsx", red);
        let before = css::class_map::get_class_map();
        let observed = with_counter_extract(input("a.tsx", blue), |_, batch| {
            batch.consume(|v| {
                let witness = v
                    .witnesses
                    .first()
                    .unwrap_or_else(|| panic!("inserted class"));
                Err::<(), _>(Box::new(
                    witness.produced.class().allocation.address.clone(),
                ))
            })
        });
        let address = match observed {
            Err(CounterCompileError::Consumer(a)) => *a,
            Err(CounterCompileError::Compile(e)) => panic!("{e}"),
            Ok(()) => panic!("abort"),
        };
        // When
        let output = required(code_extract_counter(input("a.tsx", blue), false));
        // Then
        match address {
            NameAddress::Counter {
                namespace,
                legacy_key,
                slot,
            } => {
                let prior = before
                    .get(&namespace)
                    .unwrap_or_else(|| panic!("namespace"));
                assert_eq!(slot, prior.len());
                let mut expected = before;
                expected
                    .get_mut(&namespace)
                    .unwrap_or_else(|| panic!("namespace"))
                    .insert(legacy_key, slot);
                assert_eq!(css::class_map::get_class_map(), expected);
            }
            NameAddress::Baseline { .. } => panic!("Counter"),
        }
        assert!(output.css().unwrap_or_default().contains("color:blue"));
    });
}
