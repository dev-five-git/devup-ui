use rstest::rstest;

use super::{Barreled, rewrite};
use crate::ResolvedModule;

const PACKAGE: &str = "@devup-ui/react";

#[rstest]
#[case(
    "export { Box }; import { Box } from '@devup-ui/react';",
    "import { Box } from './ui'; const view = <Box />;"
)]
#[case(
    "export default Box; import { Box } from '@devup-ui/react';",
    "import Box from './ui'; const view = <Box />;"
)]
#[case(
    "const B = Box; export { B as Box }; import { Box } from '@devup-ui/react';",
    "import { Box } from './ui'; const view = <Box />;"
)]
fn barrel_import_is_followed_when_declared_after_its_export(
    #[case] barrel: &str,
    #[case] code: &str,
) {
    let barrel = barrel.to_string();
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "ui.ts".to_string(),
            code: barrel.clone(),
        })
    };

    let result = rewrite(code, "app.tsx", PACKAGE, Some(&resolver));

    let Barreled::Rewritten(result) = result else {
        panic!("barrel import was not redirected");
    };
    assert!(
        result.code.contains("from \"@devup-ui/react\""),
        "{}",
        result.code
    );
}

#[rstest]
#[case("function f(UI) { return UI.css({ color: 'blue' }); }")]
#[case("function f() { const UI = other; return <UI.Box />; }")]
#[case("function f(UI) { const D = UI; return D.css({ color: 'blue' }); }")]
#[case("namespace Local { import UI = Other.UI; export const x = UI.css({}); }")]
fn namespace_shadow_is_preserved_when_outer_namespace_is_rewritten(
    #[case] shadow: &str,
    #[values(PACKAGE, "./ui")] source: &str,
) {
    let code = format!("import * as UI from '{source}'; const outer = UI.css({{}}); {shadow}");
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "ui.ts".to_string(),
            code: "export { css, Box } from '@devup-ui/react';".to_string(),
        })
    };

    let result = rewrite(&code, "app.tsx", PACKAGE, Some(&resolver));

    let Barreled::Rewritten(result) = result else {
        panic!("outer namespace was not rewritten");
    };
    assert!(result.code.contains(shadow), "{}", result.code);
    assert!(
        result.code.contains("const outer = UI$css({});"),
        "{}",
        result.code
    );
}

#[rstest]
#[case("function f(css) { const local = css; return local({}); }")]
#[case("function f() { const css = other; const local = css; return local({}); }")]
#[case("namespace Local { import css = Other.css; export const value = css; }")]
fn named_shadow_is_preserved_when_import_alias_is_rewritten(#[case] shadow: &str) {
    let code = format!(
        "import {{ css }} from '@devup-ui/react'; function real() {{ const local = css; return local({{}}); }} {shadow}"
    );

    let result = rewrite(&code, "app.tsx", PACKAGE, None);

    let Barreled::Rewritten(result) = result else {
        panic!("import alias was not rewritten");
    };
    assert!(result.code.contains(shadow), "{}", result.code);
    assert!(
        result.code.contains("return local$css({});"),
        "{}",
        result.code
    );
}

#[test]
fn barrel_alias_shadow_is_preserved_when_same_spelling_occurs_in_multiple_scopes() {
    let shadow = "function shadow(css) { const local = css; return local({}); }";
    let code = format!(
        "import {{ css }} from './ui'; const top = css; function a() {{ const local = top; return local({{}}); }} function b() {{ const local = css; return local({{}}); }} {shadow}"
    );
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "ui.ts".to_string(),
            code: "import { css } from '@devup-ui/react'; function f(css) { const local = css; return local; } export { css };".to_string(),
        })
    };

    let result = rewrite(&code, "app.tsx", PACKAGE, Some(&resolver));

    let Barreled::Rewritten(result) = result else {
        panic!("barrel aliases were not rewritten");
    };
    assert!(result.code.contains(shadow), "{}", result.code);
    assert_eq!(result.code.matches("return local$css({});").count(), 2);
}

#[test]
fn nested_namespace_import_is_preserved_when_exporting_an_outer_alias() {
    let shadow = "namespace Local { import UI = Other.UI; export const B = UI.Box; }";
    let code = format!("import * as UI from '@devup-ui/react'; {shadow} export const B = UI.Box;");

    let result = rewrite(&code, "app.tsx", PACKAGE, None);

    let Barreled::Rewritten(result) = result else {
        panic!("outer export was not rewritten");
    };
    assert!(result.code.contains(shadow), "{}", result.code);
    assert!(
        result
            .code
            .contains("export { Box as B } from \"@devup-ui/react\";"),
        "{}",
        result.code
    );
}

#[test]
fn runtime_import_is_unchanged_when_exporting_its_alias() {
    let code =
        "import { useTheme } from '@devup-ui/react'; const theme = useTheme; export { theme };";

    let result = rewrite(code, "app.tsx", PACKAGE, None);

    assert!(matches!(result, Barreled::Unchanged));
}

#[rstest]
#[case("export { Box } from '@devup-ui/react';")]
#[case("export { 'Box' as Box } from '@devup-ui/react';")]
fn source_export_name_has_no_local_binding_when_it_is_not_a_reference(#[case] code: &str) {
    let allocator = oxc_allocator::Allocator::default();
    let program = oxc_parser::Parser::new(&allocator, code, oxc_span::SourceType::mjs())
        .parse()
        .program;
    let semantic = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic;
    let oxc_ast::ast::Statement::ExportFromDeclaration(export) = &program.body[0] else {
        panic!("expected a source export");
    };

    let symbol = super::export_symbol(&export.specifiers[0].local, &semantic);

    assert_eq!(symbol, None);
}

#[test]
fn unresolved_alias_is_preserved_when_exporting_a_compiled_import() {
    let code =
        "import { Box } from '@devup-ui/react'; const alias = unknown; export { Box, alias };";

    let result = rewrite(code, "app.tsx", PACKAGE, None);

    let Barreled::Rewritten(result) = result else {
        panic!("compiled export was not rewritten");
    };
    assert!(
        result.code.contains("const alias = unknown;"),
        "{}",
        result.code
    );
    assert!(result.code.contains("export { alias };"), "{}", result.code);
}
