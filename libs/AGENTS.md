# LIBS - RUST CORE

CSS extraction engine. Parses JS/TS with Oxc, extracts styles, generates optimized CSS.

## STRUCTURE

```
libs/
├── extractor/          # Main extraction (9k lines)
│   ├── src/lib.rs      # Entry + comprehensive tests
│   ├── src/visit.rs    # AST visitor (DevupVisitor)
│   └── src/extract_style/  # Style value extraction
├── sheet/              # CSS output (3.3k lines)
│   ├── src/lib.rs      # StyleSheet struct
│   └── src/theme.rs    # Theme system
└── css/                # Utilities (3k lines)
    ├── src/lib.rs      # Class naming, optimization
    └── src/constant.rs # Property mappings
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add CSS property | `css/src/constant.rs` | `STYLE_PROP_MAP` |
| Modify extraction logic | `extractor/src/lib.rs` | `extract()` fn |
| AST transformation | `extractor/src/visit.rs` | `DevupVisitor` |
| Theme colors | `sheet/src/theme.rs` | `ColorTheme` |
| Responsive typography | `sheet/src/theme.rs` | `Typography` array |
| Class name generation | `css/src/lib.rs` | `num_to_nm_base()` |
| Selector ordering | `css/src/style_selector.rs` | `StyleSelector` |

## MODULE DEPENDENCIES

```
extractor ──→ css (property mappings)
    │
    ↓
  sheet ──→ css (class naming)
    │
    ↓
devup-ui-wasm (WASM exports)
```

## CONVENTIONS

- **Oxc** for parsing (NOT swc) - version `0.106.0`
- **insta** for snapshot tests - `assert_debug_snapshot!`
- **rstest** for parameterized tests - `#[rstest]` `#[case]`
- **serial_test** when touching globals - `#[serial]`
- All public functions have `_internal` variants for testing without JsValue

## KEY ALGORITHMS

### Base-37 Class Naming (`css/src/num_to_nm_base.rs`)
Encodes integers to minimal CSS class names: `0→a, 1→b, 36→aa`

### Style Extraction (`extractor/src/lib.rs`)
1. Parse JSX with Oxc
2. Visit nodes with `DevupVisitor`
3. Extract static styles → CSS classes
4. Extract dynamic styles → CSS variables
5. Transform JSX → className + style

### Theme CSS Generation (`sheet/src/theme.rs`)
- Colors: CSS variables with `light-dark()` function
- Typography: Responsive arrays → media queries
- Supports `default`, `light`, `dark` theme variants

## ANTI-PATTERNS

- **NEVER** use `unsafe` without justification
- **NEVER** ignore clippy warnings
- **NEVER** commit with failing tests
- **AVOID** `clone()` when `&` suffices
- **AVOID** `.unwrap()` in library code (use `?`)

## TESTING

```bash
cargo test                    # All tests
cargo test --package extractor  # Single crate
cargo test -- --nocapture     # See output
cargo tarpaulin               # Coverage report
```

Snapshot tests in `src/snapshots/` - run `cargo insta review` to update.
