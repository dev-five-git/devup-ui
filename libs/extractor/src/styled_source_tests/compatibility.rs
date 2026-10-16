use super::*;
use oxc_codegen::Codegen;

#[test]
#[serial]
fn attrs_input_remains_when_filtered_prepared_component_retargets_then_extends() {
    // Given: inherited captures and attrs needing a prop the render filter withholds.
    let source = format!(
        "{IMPORT}const Base=styled.div.attrs(p=>({{'data-tone':p.tone}})).withConfig({{shouldForwardProp:p=>p!=='tone'&&p!=='blocked'}})({{color:'red',...(on?{{color:'blue'}}:{{}})}});const Mid=Base.withComponent('section');const Card=styled(Mid)({{p:1}});const element=<Card tone='olive' blocked='drop' ref={{callerRef}} children='child' as='article' forwardedAs='nav' />;"
    );
    // When: JSX extraction filters explicit caller attributes.
    let output = compile("test.tsx", &source, true);
    // Then: the attrs input survives while the unread denied prop is removed, with React/target props intact.
    assert!(output.contains("const element = <Card tone=\"olive\" ref={callerRef} children=\"child\" as=\"article\" forwardedAs=\"nav\" />;"), "{output}");
}

#[rstest]
#[case("tsx")]
#[case("jsx")]
#[serial]
fn output_is_byte_identical_when_source_accepts_jsx(
    #[case] extension: &str,
    #[values(true, false)] single_css: bool,
) {
    // Given: the JSX baseline with polymorphic rendering from #710.
    let filename = format!("test.{extension}");
    let source = "import { styled } from '@devup-ui/react';\nexport const Card = styled('article', { p: 2, color: 'olive' });";
    let expected = if single_css {
        "import \"@devup-ui/react/devup-ui.css\";\nimport { forwardRef as __devupForwardRef } from \"react\";\nexport const Card = __devupForwardRef((__devupRefProps, __devupRef) => (({ style, className, as: DevupAs = \"article\", forwardedAs, ...rest }) => <DevupAs {...(({ \"theme\": __devupOmit0, ...__devupDom }) => __devupDom)(rest)} as={forwardedAs} className={[\"a--a a b\", className].filter(Boolean).join(\" \")} style={style} />)({\n\t...__devupRefProps,\n\tref: __devupRef\n}));\n"
    } else {
        "import \"@devup-ui/react/devup-ui-0.css\";\nimport { forwardRef as __devupForwardRef } from \"react\";\nexport const Card = __devupForwardRef((__devupRefProps, __devupRef) => (({ style, className, as: DevupAs = \"article\", forwardedAs, ...rest }) => <DevupAs {...(({ \"theme\": __devupOmit0, ...__devupDom }) => __devupDom)(rest)} as={forwardedAs} className={[\"a--a a-a a-b\", className].filter(Boolean).join(\" \")} style={style} />)({\n\t...__devupRefProps,\n\tref: __devupRef\n}));\n"
    };
    // When: the real extractor compiles the control.
    let output = compile(&filename, source, single_css);
    // Then: the entire module still parses and retains its exact baseline bytes.
    let arena = Allocator::default();
    parse(&arena, &filename, &output);
    assert_eq!(output, expected);
}

#[rstest]
#[case("export const keep = 1;")]
#[case("import { styled } from '@devup-ui/react'; export const keep = 1;")]
#[case("import { css } from '@devup-ui/react'; export const keep = css({ color: 'red' });")]
#[serial]
fn helper_is_absent_when_module_has_no_compiled_styled(
    #[case] source: &str,
    #[values(true, false)] single_css: bool,
) {
    // Given: valid non-JSX source without a styled renderer.
    // When: the real extractor processes it.
    let output = compile("test.ts", source, single_css);
    // Then: no React element factory import is introduced.
    let arena = Allocator::default();
    let program = parse(&arena, "test.ts", &output);
    assert_eq!(jsx_imports(&program), Vec::<String>::new());
}

#[rstest]
#[serial]
fn types_are_preserved_when_styled_is_lowered(#[values(true, false)] single_css: bool) {
    // Given: declarations unrelated to the consumed styled call.
    let declarations = "import type { External } from 'types';\ninterface Keep { value: External }\ntype KeepValue = string | number;\nenum KeepKind { A, B }\nconst keepValue: KeepValue = 'value' satisfies KeepValue;\nfunction keepIdentity<T>(value: T): T { return value; }";
    let source = format!("{IMPORT}{declarations}\nconst Card = styled('div', {{ color: 'red' }});");
    let original_arena = Allocator::default();
    let original = parse(&original_arena, "test.ts", declarations);
    let expected = Codegen::new().build(&original).code;
    // When: extraction produces the full non-JSX module.
    let output = compile("test.ts", &source, single_css);
    // Then: the unrelated declaration AST still emits exactly the same TypeScript.
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output);
    program.body.retain(|statement| match statement {
        Statement::ImportDeclaration(import) => import.import_kind.is_type(),
        Statement::VariableDeclaration(declaration) => {
            declaration.declarations.iter().any(|declarator| {
                declarator
                    .id
                    .get_binding_identifier()
                    .is_some_and(|id| id.name == "keepValue")
            })
        }
        _ => true,
    });
    assert_eq!(Codegen::new().build(&program).code, expected);
}
