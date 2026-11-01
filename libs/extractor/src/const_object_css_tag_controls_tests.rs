use super::*;
use oxc_ast::ast::Expression;
use oxc_ast_visit::Visit;

#[rstest]
#[case::named_alias(("import { css as makeCss } from '@devup-ui/react';", "", "makeCss`${rules}`", &["color-0-blue-_a__c_hover-255"][..]))]
#[case::own_namespace(("import * as ui from '@devup-ui/react';", "", "ui.css`${rules}`", &["color-0-blue-_a__c_hover-255"][..]))]
#[case::plain(("import { css } from '@devup-ui/react';", "", "css`color: red; margin: 0;`", &["color-0-red--255", "margin-0-0--255"][..]))]
#[case::scalar_only(("import { css } from '@devup-ui/react';", "const tone = 'red'; const space = 0;", "css`color: ${tone}; margin: ${space};`", &["color-0-red--255", "margin-0-0--255"][..]))]
#[case::scalar_value_before_object(("import { css } from '@devup-ui/react';", "const tone = 'red';", "css`color: ${tone}; ${rules}; margin: 0;`", &["color-0-red--255", "color-0-blue-_a__c_hover-255", "margin-0-0--255"][..]))]
#[case::scalar_statement_before_object(("import { css } from '@devup-ui/react';", "const lead = 'color: red;';", "css`${lead} ${rules}; margin: 0;`", &["color-0-red--255", "color-0-blue-_a__c_hover-255", "margin-0-0--255"][..]))]
#[serial]
fn card_links_expected_styles_when_own_tags_take_recognition_or_fast_paths(
    #[case] sample: (&str, &str, &str, &[&str]),
    #[values(false, true)] single_css: bool,
) {
    // Given: expected atoms are authored independently of extraction output.
    let _state = crate::test_state::TestStateGuard::new();
    css::debug::set_debug(true);
    let (import, binding, expression, expected) = sample;
    let filename = "/src/tag-controls.ts";
    let source = format!(
        "{import}\nconst rules = {{ selectors: {{ '&:hover': {{ color: 'blue' }} }} }};\n{binding}\nexport const card = {expression};"
    );

    // When
    let output = extract(
        filename,
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"));

    // Then: a selected card atom must also link to an actual emitted style.
    assert_composed_export_atoms(&output.code, "card", expected);
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let Statement::ExportDeclaration(export) = parsed.program.body.last().unwrap() else {
        panic!("expected exported card");
    };
    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("expected card declaration");
    };
    let Expression::StringLiteral(classes) = declaration.declarations[0].init.as_ref().unwrap()
    else {
        panic!("expected compiled class string, not an object or tagged template");
    };
    for class in classes.value.as_str().unwrap().split_whitespace() {
        assert!(output.styles.iter().any(|style| {
            matches!(style.extract((!single_css).then_some(filename)), Some(StyleProperty::ClassName(name)) if name == class)
        }), "selected card class {class} has no emitted style: {output:?}");
    }
    assert_eq!(classes.value.as_str().unwrap().split_whitespace().count(), expected.len());
}

#[rstest]
#[case::shadowed(("", "css", "css"))]
#[case::unrelated(("import { html } from './markup';", "", "html"))]
#[case::shadowed_namespace(("import * as ui from '@devup-ui/react';", "ui", "ui.css"))]
#[serial]
fn unrelated_tags_retain_template_reads_when_own_css_is_in_scope(
    #[case] sample: (&str, &str, &str),
    #[values(false, true)] single_css: bool,
) {
    struct RetainedTag<'s> {
        name: &'s str,
        count: usize,
    }
    impl<'a> Visit<'a> for RetainedTag<'_> {
        fn visit_tagged_template_expression(
            &mut self,
            tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
        ) {
            self.count += 1;
            if self.name == "ui.css" {
                let Expression::StaticMemberExpression(member) = &tagged.tag else {
                    panic!("namespace tag changed shape");
                };
                let Expression::Identifier(namespace) = &member.object else {
                    panic!("namespace root changed shape");
                };
                assert_eq!(namespace.name, "ui");
                assert_eq!(member.property.name, "css");
            } else {
                let Expression::Identifier(name) = &tagged.tag else {
                    panic!("unrelated tag changed shape");
                };
                assert_eq!(name.name, self.name);
            }
            let quasis: Vec<_> = tagged
                .quasi
                .quasis
                .iter()
                .map(|quasi| quasi.value.raw.as_str())
                .collect();
            assert_eq!(quasis, ["before ", " after"]);
            assert_eq!(tagged.quasi.expressions.len(), 1);
            let Expression::Identifier(read) = &tagged.quasi.expressions[0] else {
                panic!("ordinary object read was transformed");
            };
            assert_eq!(read.name, "rules");
        }
    }
    // Given: an unresolved object leaf must remain ordinary JavaScript here.
    let _state = crate::test_state::TestStateGuard::new();
    let (import, parameter, tag) = sample;
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{import}\nconst rules = {{ color: globalThis.tone }};\nexport function render({parameter}) {{ return {tag}`before ${{rules}} after`; }}"
    );

    // When
    let output = extract(
        "/src/tag-controls.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"));

    // Then: inspect the retained tag and interpolation, not a code dump.
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut observer = RetainedTag {
        name: tag,
        count: 0,
    };
    observer.visit_program(&parsed.program);
    assert_eq!(observer.count, 1);
    assert_eq!(output.styles.len(), 0);
}
