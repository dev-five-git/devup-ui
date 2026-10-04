use crate::dead_properties_test_utils::{failure, position};
use crate::utils::{RESPONSIVE_ARRAY, build_time_error};
use crate::{ExtractOption, ImportAlias, ResolvedModule, extract, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn options() -> ExtractOption {
    ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case("export const bad = style({p:[[1,2],3]});", "style")]
#[case("export const bad = style({p:[([1,2] as const),3]});", "style")]
#[case(
    "export const bad = style({selectors:{'&:hover':{p:[[1,2],3]}}});",
    "style"
)]
#[case("export const bad = style({_hover:[{p:[1,2]},null]});", "style")]
#[case(
    "export const bad = style({selectors:{'&:hover':[[1,2],3]}});",
    "style"
)]
#[case("export const bad = style([{color:'red'},[{p:[[1,2],3]}]]);", "style")]
#[case("globalStyle('body',{p:[[1,2],3]});", "globalStyle")]
#[case("export const bad = keyframes({from:{p:[[1,2],3]}});", "keyframes")]
#[case(
    "export const bad = styleVariants({small:{p:[[1,2],3]}});",
    "styleVariants"
)]
#[serial]
fn literal_nested_arrays_report_the_authored_array(#[case] statement: &str, #[case] api: &str) {
    // Given an authored literal whose nested array is not at the call's column.
    reset_class_map();
    reset_file_map();
    let source = format!("import {{{api}}} from '@vanilla-extract/css';\n\n\n\n{statement}");
    let column = position(statement, "[1,2]") + 1;

    // When stylesheet declaration validation runs before evaluation.
    let error = failure(extract("literal.css.ts", &source, options()));

    // Then the error uses the inner array's authored position and expression.
    assert_eq!(
        error,
        format!(
            "literal.css.ts:5:{column}: {}",
            build_time_error(api, "[1, 2]", RESPONSIVE_ARRAY)
        )
    );
}

#[rstest]
#[case("const values=[[1,2],3];", concat!("{", "p:values", "}"), "p -> 0")]
#[case("const values=Array.of([1,2],3);", concat!("{", "p:values", "}"), "p -> 0")]
#[case("const values=() => [[1,2],3];", "{p:values()}", "p -> 0")]
#[case("const key='p';", "{[key]:[[1,2],3]}", "p -> 0")]
#[case("const values={p:[[1,2],3]};", "{...values}", "p -> 0")]
#[case(
    "const values=[[1,2],3];",
    concat!("{selectors:{'&:hover':", "{", "p:values", "}}}"),
    "selectors -> &:hover -> p -> 0"
)]
#[case(
    "const values=[[1,2],3];",
    "{selectors:{'&:hover':values}}",
    "selectors -> &:hover -> 0"
)]
#[case(
    "const values=[[1,2],3];",
    concat!("{'@media':{'(min-width: 600px)':", "{", "p:values", "}}}"),
    "@media -> (min-width: 600px) -> p -> 0"
)]
#[case(
    "const values=[{p:[1,2]},null];",
    "{_hover:values}",
    "_hover -> 0 -> p"
)]
#[serial]
fn evaluated_nested_arrays_report_the_authored_call_and_path(
    #[case] declaration: &str,
    #[case] argument: &str,
    #[case] path: &str,
) {
    // Given a value that acquires its responsive shape during evaluation.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\n{declaration}\n\n\nexport const bad = style({argument});"
    );

    // When the evaluated JSON is validated before generated css() lowering.
    let error = failure(extract("evaluated.css.ts", &source, options()));

    // Then neither coordinates nor the API name refer to the generated code.
    assert_eq!(
        error,
        format!(
            "evaluated.css.ts:5:20: {}",
            build_time_error("style", path, RESPONSIVE_ARRAY)
        )
    );
}

#[rstest]
#[case("values", "[[1,2],3]", concat!("{", "p:values", "}"))]
#[case("rules", "{selectors:{'&:hover':{p:[[1,2],3]}}}", "rules")]
#[serial]
fn imported_nested_arrays_report_the_consuming_stylesheet_call(
    #[case] export: &str,
    #[case] value: &str,
    #[case] argument: &str,
) {
    // Given imported data whose declaration is not an authored styling call.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\nimport {{{export}}} from './values';\n\n\nexport const bad = style({argument});"
    );
    let module = format!("export const {export}={value};");
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./values" => Some(ResolvedModule {
            path: "values.ts".to_string(),
            code: module.clone(),
        }),
        _ => None,
    };

    // When the importing stylesheet evaluates the declaration.
    let error = failure(extract_with_modules(
        "importer.css.ts",
        &source,
        options(),
        false,
        &resolver,
    ));

    // Then its styling call, not the data module or generated css(), owns it.
    let path = match export {
        "values" => "p -> 0",
        _ => "selectors -> &:hover -> p -> 0",
    };
    assert_eq!(
        error,
        format!(
            "importer.css.ts:5:20: {}",
            build_time_error("style", path, RESPONSIVE_ARRAY)
        )
    );
}

#[test]
#[serial]
fn imported_stylesheet_nested_arrays_keep_the_declaring_call_origin() {
    // Given an imported stylesheet containing the invalid styling call.
    reset_class_map();
    reset_file_map();
    let source = "import {style} from '@vanilla-extract/css';\nimport {bad} from './other.css';\nexport const result=style([bad,{color:'red'}]);";
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "./other.css" => Some(ResolvedModule {
            path: "other.css.ts".to_string(),
            code: concat!("import {style} from '@vanilla-extract/css';\nconst values=[[1,2],3];\n\n\nexport const bad = style(", "{", "p:values", "}", ");").to_string(),
        }),
        _ => None,
    }
    };

    // When its importer requests the exported style.
    let error = failure(extract_with_modules(
        "outer.css.ts",
        source,
        options(),
        false,
        &resolver,
    ));

    // Then the authored styling call remains in the imported stylesheet.
    assert_eq!(
        error,
        format!(
            "other.css.ts:5:20: {}",
            build_time_error("style", "p -> 0", RESPONSIVE_ARRAY)
        )
    );
}

#[rstest]
#[case("{p:[1,2,3]}")]
#[case("{p:[1,null,,3]}")]
#[case("{_hover:[{p:1},null,{p:3}]}")]
#[case("{p:([[1,2],[3,4]])[0]}")]
#[case("{p:({small:[1,2],large:[3,4]})['small']}")]
#[case("{p:(true ? [1,2] : [[3,4],5])}")]
#[case("{p:[false ? [1,2] : 3,4]}")]
#[case("{p:([1,null,3] as const)}")]
#[case("{selectors:{'&:hover':{p:[1,null,3]}}}")]
#[case("[{p:[1,2]},[{color:'red'}]]")]
#[serial]
fn valid_responsive_values_and_root_compositions_remain_extractable(
    #[case] argument: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given flat responsive values or arrays that compose entire styles.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\nexport const valid=style({argument});"
    );

    // When the stylesheet passes authored and evaluated validation.
    let output = extract("valid.css.ts", &source, options())?;

    // Then it still produces extracted styles instead of a blanket array error.
    assert_ne!(output.styles.len(), 0);
    Ok(())
}
