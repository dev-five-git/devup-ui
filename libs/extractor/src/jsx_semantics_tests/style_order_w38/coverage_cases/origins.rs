use super::*;

#[test]
#[serial]
fn ordinary_bags_when_explicit_keep_reserved_spellings_and_zero() {
    // Given: both component syntaxes with an explicit ordinary props bag.
    for usage in [
        "<Box color='red' props={bag}/>",
        "jsx(Box,{color:'red',props:bag})",
    ] {
        let source = format!(
            "{BOX}{JSX_RUNTIME}const symbol=Symbol('ordinary');const bag={{get styleOrder(){{trace.push('order');return 0}},get ['style-order'](){{trace.push('kebab');return 0}},get title(){{trace.push('title');return 'ordinary'}},get [symbol](){{trace.push('symbol');return 7}},className:'ordinary-class',style:{{opacity:0.4}}}};const a={usage};"
        );
        // When: the public extractor's generated code forwards the bag.
        let actual = whole::evaluate(
            &source,
            "[a.props.styleOrder,a.props['style-order'],a.props.title,a.props[symbol],a.props.className,a.props.style.opacity,Object.getOwnPropertyDescriptor(bag,'styleOrder').get!==undefined]",
        );
        // Then: ordinary data wins unchanged, with exactly one getter read.
        assert_eq!(
            actual.trace,
            serde_json::json!(["order", "kebab", "title", "symbol"])
        );
        assert_eq!(
            actual.element,
            serde_json::json!([0, 0, "ordinary", 7, "ordinary-class", 0.4, true])
        );
    }
}

#[test]
#[serial]
fn mixed_origins_when_bag_follows_metadata_keep_ordinary_winners() {
    // Given: a getter-bearing styling spread followed by an ordinary bag.
    for usage in [
        "<Box {...config} styleOrder={3} color='red' props={bag}/>",
        "jsx(Box,{...config,styleOrder:3,color:'red',props:bag})",
    ] {
        let source = format!(
            "{BOX}{JSX_RUNTIME}const symbol=Symbol('ordinary');const config={{get styleOrder(){{trace.push('order');return 2}},get ['style-order'](){{trace.push('kebab');return 4}},get title(){{trace.push('title');return 'kept'}},get [symbol](){{trace.push('symbol');return 7}},get className(){{trace.push('class');return 'source-class'}},get style(){{trace.push('style');return {{opacity:0.4}}}}}};const bag={{styleOrder:0,'style-order':0}};const a={usage};"
        );
        // When: source reads and the forwarded copy pass through extraction.
        let actual = whole::evaluate(
            &source,
            "[a.props.styleOrder,a.props['style-order'],a.props.title,a.props[symbol],a.props.className.trim(),a.props.style.opacity,Object.getOwnPropertyDescriptor(config,'styleOrder').get!==undefined,config.styleOrder]",
        );
        // Then: filtering applies before the bag, without changing source descriptors.
        assert_eq!(
            actual.trace,
            serde_json::json!([
                "order", "kebab", "title", "class", "style", "symbol", "order"
            ])
        );
        assert_eq!(
            actual.element,
            serde_json::json!([
                0,
                0,
                "kept",
                7,
                "source-class color-0-red--3-a",
                0.4,
                true,
                2
            ])
        );
    }
}

#[test]
#[serial]
fn computed_metadata_when_width_requires_evaluation_keeps_synthesized_absence() {
    // Given: the main real-WASM reproducer, with a genuinely computed width.
    let source = "import {css} from '@devup-ui/react';\nfunction make(n) { return n + 'px'; }\nconst a = css({[`style${'Order'}`]: false && 2, w: make(2)});";
    // When: the public fallback computes the ordinary value.
    let actual = output(source);
    // Then: the only rule is 2px at default order; metadata emits no CSS.
    let rules: Vec<_> = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property(), style.value(), style.style_order()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(rules, vec![("width", "2px", None)]);
}

#[test]
#[serial]
fn computed_metadata_when_key_is_a_constant_validates_both_arms() {
    // Given: metadata whose computed key becomes readable by constant inlining.
    let source = "import {css} from '@devup-ui/react';\nconst k='styleOrder';const a=css({[k]:true?2:'01',color:'red'});";
    let column = source
        .lines()
        .nth(1)
        .required("fixture line")
        .find("'01'")
        .required("invalid literal")
        + 1;
    // When: the public extractor substitutes the key.
    let actual = error(source);
    // Then: the invalid alternate is still rejected at its original location.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn ordinary_bags_when_protected_fixture_compiles_keep_b2_code_bytes() {
    // Given: the durable order-data.tsx fixture and its b2 emitted code.
    let source = "import { css, Box } from '@devup-ui/react'\nexport const plain = css({ color: 'red', props: { styleOrder: 0 }, styleVars: { '--x': { styleOrder: 0 } } })\nexport const App = <Box color=\"red\" props={{ styleOrder: 0 }} />\n";
    reset_class_map();
    reset_file_map();
    let debug = css::debug::is_debug();
    css::debug::set_debug(false);
    // When: the public Rust entrypoint compiles the exact protected input.
    let actual = extract(
        "order-data.tsx",
        source,
        ExtractOption {
            single_css: true,
            import_main_css: true,
            ..ExtractOption::default()
        },
    );
    css::debug::set_debug(debug);
    let actual = actual.required("protected fixture must compile");
    // Then: its code is byte-identical to the saved b2 output.
    assert_eq!(
        actual.code,
        "import \"@devup-ui/react/devup-ui.css\";\nexport const plain = \"a\";\nexport const App = <div className=\"a\" {...{ styleOrder: 0 }} />;\n"
    );
}

#[test]
#[serial]
fn factory_non_object_props_when_opaque_omit_only_order_metadata() {
    // Given: the source-derived non-object factory candidate, not a fabricated AST.
    let source = format!(
        "{BOX}{JSX_RUNTIME}const symbol=Symbol('ordinary');const config={{get styleOrder(){{trace.push('order');return 2}},get ['style-order'](){{trace.push('kebab');return 4}},get title(){{trace.push('title');return 'kept'}},get className(){{trace.push('class');return 'ordinary-class'}},get style(){{trace.push('style');return {{opacity:0.4}}}},get color(){{trace.push('color');return 'blue'}},get [symbol](){{trace.push('symbol');return 7}}}};const render=props=>jsx(Box,props);const a=render(config);"
    );
    // When: the extracted factory forwards the opaque parameter.
    let actual = whole::evaluate(
        &source,
        "({data:[a.props.title,a.props[symbol],Object.getOwnPropertyDescriptor(config,'styleOrder').get!==undefined,a.props.className,a.props.style.opacity,a.props.color],metadata:[Object.hasOwn(a.props,'styleOrder'),Object.hasOwn(a.props,'style-order')]})",
    );
    // Then: the independently observable native reads remain once-only.
    assert_eq!(
        actual.trace,
        serde_json::json!([
            "order", "kebab", "title", "class", "style", "color", "symbol"
        ])
    );
    assert_eq!(
        actual.element["data"],
        serde_json::json!(["kept", 7, true, "ordinary-class", 0.4, "blue"])
    );
    assert_eq!(
        actual.element["metadata"],
        serde_json::json!([false, false])
    );
}
