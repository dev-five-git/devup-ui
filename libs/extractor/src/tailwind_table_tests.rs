//! Every keyword of the lookup tables of the Tailwind utilities, compiled
//! through `parse_class` and written out as the declarations it yields

use insta::assert_snapshot;
use serial_test::serial;

use crate::tailwind::{declarations_of, merge_classes, parse_class};

/// Each class on a line with the declarations it compiles to
fn compiled(classes: &[&str]) -> String {
    classes
        .iter()
        .map(|class| {
            let declarations: Vec<String> = declarations_of(class)
                .unwrap_or_default()
                .iter()
                .map(|(property, value)| format!("{property}: {value}"))
                .collect();
            format!("{class} => {}", declarations.join("; "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The condition each class applies under, written out
fn conditions(classes: &[&str]) -> String {
    classes
        .iter()
        .map(|class| {
            let conditions: Vec<String> = parse_class(class)
                .into_iter()
                .flat_map(|class| class.conditions)
                .map(|condition| condition.map_or_else(String::new, |c| c.to_string()))
                .collect();
            format!("{class} => {}", conditions.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
#[serial]
fn layout_keywords() {
    assert_snapshot!(compiled(&[
        "inline",
        "table",
        "inline-table",
        "table-caption",
        "table-cell",
        "table-column",
        "table-column-group",
        "table-footer-group",
        "table-header-group",
        "table-row-group",
        "table-row",
        "flow-root",
        "inline-grid",
        "contents",
        "list-item",
        "static",
        "visible",
        "invisible",
        "collapse",
        "box-border",
        "box-content",
        "float-start",
        "float-end",
        "float-right",
        "float-left",
        "float-none",
        "clear-start",
        "clear-end",
        "clear-left",
        "clear-right",
        "clear-both",
        "clear-none",
        "isolate",
        "isolation-auto",
        "object-contain",
        "object-cover",
        "object-fill",
        "object-none",
        "object-scale-down",
        "object-bottom",
        "object-center",
        "object-left",
        "object-left-bottom",
        "object-left-top",
        "object-right",
        "object-right-bottom",
        "object-right-top",
        "object-top",
        "overflow-auto",
        "overflow-clip",
        "overflow-visible",
        "overflow-scroll",
        "overflow-x-auto",
        "overflow-y-auto",
        "overflow-x-hidden",
        "overflow-y-hidden",
        "overflow-x-clip",
        "overflow-y-clip",
        "overflow-x-visible",
        "overflow-y-visible",
        "overflow-x-scroll",
        "overflow-y-scroll",
        "overscroll-auto",
        "overscroll-contain",
        "overscroll-none",
        "overscroll-x-auto",
        "overscroll-x-contain",
        "overscroll-x-none",
        "overscroll-y-auto",
        "overscroll-y-contain",
        "overscroll-y-none",
    ]));
}

#[test]
#[serial]
fn flex_grid_keywords() {
    assert_snapshot!(compiled(&[
        "basis-auto",
        "basis-full",
        "flex-row-reverse",
        "flex-col-reverse",
        "flex-wrap-reverse",
        "flex-nowrap",
        "flex-auto",
        "flex-initial",
        "flex-none",
        "grow",
        "grow-0",
        "shrink",
        "shrink-0",
        "order-last",
        "order-none",
        "grid-cols-none",
        "grid-cols-subgrid",
        "grid-rows-none",
        "grid-rows-subgrid",
        "col-auto",
        "col-span-full",
        "row-auto",
        "row-span-full",
        "grid-flow-row",
        "grid-flow-col",
        "grid-flow-dense",
        "grid-flow-row-dense",
        "grid-flow-col-dense",
        "auto-cols-auto",
        "auto-cols-min",
        "auto-cols-max",
        "auto-cols-fr",
        "auto-rows-auto",
        "auto-rows-min",
        "auto-rows-max",
        "auto-rows-fr",
        "justify-normal",
        "justify-start",
        "justify-end",
        "justify-around",
        "justify-evenly",
        "justify-stretch",
        "justify-items-start",
        "justify-items-end",
        "justify-items-center",
        "justify-items-stretch",
        "justify-self-auto",
        "justify-self-start",
        "justify-self-end",
        "justify-self-center",
        "justify-self-stretch",
        "content-normal",
        "content-center",
        "content-start",
        "content-end",
        "content-between",
        "content-around",
        "content-evenly",
        "content-baseline",
        "content-stretch",
        "items-end",
        "items-baseline",
        "items-stretch",
        "self-auto",
        "self-start",
        "self-end",
        "self-center",
        "self-stretch",
        "self-baseline",
        "place-content-center",
        "place-content-start",
        "place-content-end",
        "place-content-between",
        "place-content-around",
        "place-content-evenly",
        "place-content-baseline",
        "place-content-stretch",
        "place-items-start",
        "place-items-end",
        "place-items-center",
        "place-items-baseline",
        "place-items-stretch",
        "place-self-auto",
        "place-self-start",
        "place-self-end",
        "place-self-center",
        "place-self-stretch",
    ]));
}

#[test]
#[serial]
fn sizing_and_typography_keywords() {
    assert_snapshot!(compiled(&[
        "max-w-none",
        "max-w-0",
        "max-w-xs",
        "max-w-md",
        "max-w-lg",
        "max-w-full",
        "min-h-0",
        "min-h-full",
        "min-h-screen",
        "max-h-none",
        "max-h-full",
        "max-h-screen",
        "text-left",
        "text-right",
        "text-justify",
        "not-italic",
        "overline",
        "no-underline",
        "lowercase",
        "capitalize",
        "normal-case",
        "text-ellipsis",
        "text-clip",
        "text-wrap",
        "text-nowrap",
        "text-balance",
        "text-pretty",
        "break-normal",
        "break-words",
        "break-all",
        "break-keep",
        "resize-none",
        "resize-y",
        "resize-x",
        "resize",
        "outline-(length:--width)",
        "decoration-(length:--width)",
        "shadow-inner",
    ]));
}

#[test]
#[serial]
fn misc_keywords() {
    assert_snapshot!(compiled(&[
        "field-sizing-fixed",
        "table-auto",
        "caption-top",
        "transform-flat",
        "backface-visible",
        "forced-color-adjust-auto",
        "scheme-normal",
        "scheme-dark",
        "scheme-light",
        "scheme-only-dark",
        "scheme-only-light",
        "wrap-break-word",
        "wrap-normal",
        "decoration-solid",
        "decoration-double",
        "decoration-dotted",
        "decoration-dashed",
        "decoration-auto",
        "mask-subtract",
        "mask-exclude",
        "mask-alpha",
        "mask-type-luminance",
        "mask-auto",
        "mask-contain",
        "mask-top",
        "mask-top-right",
        "mask-bottom",
        "mask-bottom-left",
        "mask-bottom-right",
        "mask-left",
        "mask-right",
        "mask-repeat",
        "mask-no-repeat",
        "mask-repeat-x",
        "mask-repeat-y",
        "mask-repeat-space",
        "mask-clip-border",
        "mask-clip-content",
        "mask-clip-fill",
        "mask-clip-stroke",
        "mask-clip-view",
        "mask-origin-border",
        "mask-origin-padding",
        "mask-origin-content",
        "mask-origin-fill",
        "mask-origin-stroke",
        "font-stretch-normal",
        "font-stretch-extra-condensed",
        "font-stretch-condensed",
        "font-stretch-semi-condensed",
        "font-stretch-semi-expanded",
        "font-stretch-expanded",
        "font-stretch-extra-expanded",
        "font-stretch-ultra-expanded",
    ]));
}

#[test]
#[serial]
fn container_variant_widths() {
    assert_snapshot!(conditions(&[
        "@2xs:p-4", "@xs:p-4", "@xl:p-4", "@2xl:p-4", "@3xl:p-4", "@4xl:p-4", "@5xl:p-4",
        "@6xl:p-4",
    ]));
}

#[test]
#[serial]
fn merge_reads_the_nested_declarations() {
    assert_eq!(
        merge_classes("outline-hidden outline-hidden"),
        "outline-hidden"
    );
    assert_eq!(
        merge_classes("outline-hidden outline-none"),
        "outline-hidden outline-none"
    );
}
