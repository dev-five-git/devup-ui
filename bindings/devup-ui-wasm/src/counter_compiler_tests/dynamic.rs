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
