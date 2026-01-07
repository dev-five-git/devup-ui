# DEVUP UI - PROJECT KNOWLEDGE BASE

**Generated:** 2026-01-07
**Commit:** 06887f92
**Branch:** main

## OVERVIEW

Zero-runtime CSS-in-JS preprocessor. Rust core (Oxc parser) → WASM → TypeScript plugins for Vite/Next/Webpack/Rsbuild/Bun. Build-time extraction, no client JS for styling.

## STRUCTURE

```
devup-ui/
├── libs/               # Rust core - CSS extraction engine (SEE libs/AGENTS.md)
│   ├── extractor/      # JSX→CSS transformation (9k lines)
│   ├── sheet/          # CSS generation + theming
│   └── css/            # Utilities, selectors, optimization
├── bindings/           # WASM bridge (SEE bindings/devup-ui-wasm/AGENTS.md)
│   └── devup-ui-wasm/  # wasm-bindgen exports to JS
├── packages/           # NPM packages
│   ├── react/          # @devup-ui/react - Components + API (SEE packages/react/AGENTS.md)
│   ├── components/     # @devup-ui/components - Button, Input, Select...
│   ├── plugin-utils/   # Shared config loading
│   ├── *-plugin/       # Build tool plugins (vite, next, webpack, rsbuild, bun)
│   ├── eslint-plugin/  # Lint rules
│   └── reset-css/      # CSS reset
├── apps/               # Demo apps (vite, next, rsbuild, landing)
└── benchmark/          # Performance comparisons vs Tailwind, StyleX, etc.
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add CSS property | `libs/css/src/constant.rs` | Property mappings |
| Add pseudo selector | `packages/react/src/types/props/selector/` | TypeScript types |
| Modify extraction | `libs/extractor/src/lib.rs` | Core logic + tests |
| Theme system | `libs/sheet/src/theme.rs` | Color/typography |
| Plugin behavior | `packages/*-plugin/src/plugin.ts` | All follow same pattern |
| Component API | `packages/react/src/components/` | Box, Flex, Text... |
| WASM exports | `bindings/devup-ui-wasm/src/lib.rs` | JS-exposed functions |

## CODE MAP

### Rust Core (libs/)

| Module | File | Lines | Role |
|--------|------|-------|------|
| extractor | `lib.rs` | 9,094 | Main extraction + tests |
| sheet | `lib.rs` | 1,821 | CSS output generation |
| theme | `theme.rs` | 1,526 | Color/typography system |
| css_utils | `css_utils.rs` | 1,239 | Template literal parsing |
| visit | `visit.rs` | 669 | AST visitor pattern |

### TypeScript Packages

| Package | Entry | Exports |
|---------|-------|---------|
| @devup-ui/react | `src/index.ts` | Box, Flex, Text, styled, css, globalCss, keyframes |
| @devup-ui/components | `src/index.ts` | Button, Input, Select, Checkbox, Toggle, Stepper |
| @devup-ui/wasm | `pkg/index.js` | codeExtract, getCss, registerTheme |

## CONVENTIONS

### Rust
- `oxc` crate for JS/TS parsing (NOT swc)
- Snapshot testing with `insta`
- Parameterized tests with `rstest`
- Serial tests with `serial_test` when touching globals

### TypeScript
- Bun as package manager AND test runner
- 100% coverage thresholds enforced
- ESM-first, dual CJS/ESM builds
- `bun-test-env-dom` for component tests

### Styling Props
- Shorthand: `bg`, `p`, `m`, `w`, `h` (NOT `background`, `padding`...)
- Responsive: `prop={[mobile, tablet, desktop]}`
- Theme tokens: `$primary`, `$text` prefix
- Selectors: `_hover`, `_focus`, `_before`

### Class Naming
- Base-37 encoding: `a`, `b`...`aa`, `ab`
- Prefix configurable via `setPrefix()`
- Minimal output size prioritized

## ANTI-PATTERNS (THIS PROJECT)

- **NEVER** use runtime styling - all processed at build time
- **NEVER** use `as any` or `@ts-ignore` - strict types enforced
- **NEVER** suppress Rust warnings - `clippy -D warnings` in CI
- **NEVER** skip tests - 100% coverage required
- **AVOID** adding dependencies - minimal footprint goal

## UNIQUE STYLES

### Zero-Runtime Components
All React components throw `Error('Cannot run on the runtime')` - they're compile-time only placeholders that get replaced with `<div className="..." />`.

### Theme Configuration
`devup.json` at project root:
```json
{
  "theme": {
    "colors": { "default": {...}, "dark": {...} },
    "typography": { "heading": {...} }
  }
}
```

### Plugin Pattern
All plugins wrap bundler config:
```ts
// vite.config.ts
export default DevupUI(viteConfig, { singleCss: true })
```

## COMMANDS

```bash
# Development
bun install              # Install deps
bun run build            # Build all packages (WASM first)
bun run dev              # Dev mode all packages

# Testing
bun test                 # JS/TS tests
cargo test               # Rust tests
cargo tarpaulin          # Rust coverage

# Linting
bun run lint             # ESLint + cargo fmt + clippy
bun run lint:fix         # Auto-fix

# Benchmarks
bun benchmark            # Compare vs other CSS-in-JS libs
```

## NOTES

- **Build order matters**: WASM → plugin-utils → react → plugins
- **Turbopack compatible**: Use `singleCss: true` for best perf
- **RSC ready**: No client-side JS, works with React Server Components
- **TypeScript theme types**: Auto-generated from `devup.json`
