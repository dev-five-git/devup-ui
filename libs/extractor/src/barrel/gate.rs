use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Program, Statement,
    VariableDeclarationKind,
};

pub(super) fn has_export_candidate(program: &Program<'_>) -> bool {
    program.body.iter().any(|statement| match statement {
        Statement::ExportNamedDeclaration(export) => {
            !export.export_kind.is_type()
                && export
                    .specifiers
                    .iter()
                    .any(|specifier| !specifier.export_kind.is_type())
        }
        Statement::ExportDefaultDeclaration(export) => {
            matches!(
                export.declaration,
                ExportDefaultDeclarationKind::Identifier(_)
            )
        }
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => {
                declaration.kind == VariableDeclarationKind::Const
                    && declaration.declarations.iter().any(|declarator| {
                        matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                            && matches!(
                                declarator.init,
                                Some(
                                    Expression::Identifier(_)
                                        | Expression::StaticMemberExpression(_)
                                )
                            )
                    })
            }
            _ => false,
        },
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    use super::has_export_candidate;
    use crate::ResolvedModule;
    use crate::barrel::{Barreled, Link, analyze, rewrite};

    #[rstest]
    #[case("export { Box };", true)]
    #[case("export { type Props, css as style };", true)]
    #[case("export { type Props };", false)]
    #[case("export type { Box };", false)]
    #[case("export {};", false)]
    #[case("export default Box;", true)]
    #[case("export default function Page() { return <Box />; }", false)]
    #[case("export default () => <Box />;", false)]
    #[case("export const B = /* comment */ Box;", true)]
    #[case("export const c = UI.css;", true)]
    #[case("export const other = 1, B = Box;", true)]
    #[case("export const Page = <Box />;", false)]
    #[case("export const value = 1;", false)]
    #[case("export const style = css({});", false)]
    #[case("export const { Box } = UI;", false)]
    #[case("export let B = Box;", false)]
    #[case("export const B = Box['name'];", false)]
    #[case("export function Page() { return <Box />; }", false)]
    #[case("export { Box } from '@devup-ui/react';", false)]
    #[case("export * from '@devup-ui/react';", false)]
    #[case("const text = 'export';", false)]
    fn export_shape_selects_analysis_candidates(#[case] code: &str, #[case] expected: bool) {
        let allocator = Allocator::default();
        let program = Parser::new(&allocator, code, SourceType::tsx())
            .parse()
            .program;

        let candidate = has_export_candidate(&program);

        assert_eq!(candidate, expected, "{code}");
    }

    #[rstest]
    #[case("const B = /* comment */ Box; export { B };")]
    #[case("const B = /* comment */ Box; export default B;")]
    #[case("export const B = /* comment */ Box;")]
    fn exported_alias_is_rewritten_when_assignment_text_does_not_match(#[case] exported: &str) {
        let code = format!("import {{ Box }} from '@devup-ui/react'; {exported}");

        let result = rewrite(&code, "ui.tsx", "@devup-ui/react", None);

        let Barreled::Rewritten(result) = result else {
            panic!("exported alias was not rewritten");
        };
        assert!(
            result.code.contains("from \"@devup-ui/react\";"),
            "{}",
            result.code
        );
        assert!(result.code.contains("export { Box as"), "{}", result.code);
    }

    #[test]
    fn inline_type_import_is_not_a_runtime_origin_when_value_imports_are_mixed() {
        let module = ResolvedModule {
            path: "ui.ts".to_string(),
            code: "export { Box, css }; import { type Box, css } from '@devup-ui/react';"
                .to_string(),
        };

        let exports = analyze(&module, "@devup-ui/react");

        assert!(matches!(exports.named.get("Box"), Some(Link::Own)));
        assert!(
            matches!(exports.named.get("css"), Some(Link::From { source, imported: Some(name) }) if source == "@devup-ui/react" && name == "css")
        );
    }
}
