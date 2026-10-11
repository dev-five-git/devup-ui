use crate::{ExtractOption, vanilla_extract::Stylesheet};

#[test]
fn malformed_authored_source_when_compiling_is_left_for_the_original_parser() -> Result<(), String>
{
    let source = "import {css} from '@devup-ui/react'; export const box=css({ color: ; });";
    let stylesheet = Stylesheet {
        filename: "/broken.css.ts",
        code: source,
        source,
        edits: &[],
    };
    let compiled = super::compile(stylesheet, &ExtractOption::default(), None)?;
    assert_eq!(compiled.code, None);
    assert_eq!(compiled.edits, vec![]);
    assert_eq!(compiled.styles.into_iter().collect::<Vec<_>>(), vec![]);
    Ok(())
}
