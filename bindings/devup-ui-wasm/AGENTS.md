# @DEVUP-UI/WASM - WASM BRIDGE

Rust → WebAssembly → JavaScript bridge. Exposes extraction engine to build plugins.

## EXPORTS (JS API)

### Core Functions
- `codeExtract(filename, code, pkg, cssDir, singleCss, importMainCss, importCssInCss)` → Output
- `hasDevupUI(filename, code, package)` → boolean
- `getCss(fileNum?, importMainCss)` → string

### Theme Functions
- `registerTheme(themeObject)` → void
- `getDefaultTheme()` → string | null
- `getThemeInterface(pkg, colorIface, typoIface, themeIface)` → string

### State Management
- `importSheet()/exportSheet()` - StyleSheet state
- `importClassMap()/exportClassMap()` - Class mappings
- `importFileMap()/exportFileMap()` - File tracking

### Configuration
- `setPrefix()/getPrefix()` - Class name prefix
- `setDebug()/isDebug()` - Debug mode

## OUTPUT STRUCTURE

```typescript
interface Output {
  code: string           // Transformed source
  css: string | null     // Generated CSS (if changed)
  map: string | null     // Source map
  cssFile: string | null // CSS filename
  updatedBaseStyle: bool // Whether base CSS changed
}
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add WASM export | `src/lib.rs` | `#[wasm_bindgen]` |
| Modify Output | `src/lib.rs` | `Output` struct |
| Change extraction | `../libs/extractor/` | Core logic |
| Update types | `pkg/index.d.ts` | Auto-generated |

## BUILD PROCESS

```bash
# In bindings/devup-ui-wasm/
wasm-pack build --target nodejs
node script.js  # Post-process package.json
```

Output goes to `pkg/`:
- `index.js` - JS wrapper
- `index_bg.wasm` - WASM binary
- `index.d.ts` - TypeScript types

## CONVENTIONS

- All Rust functions have `_internal` variants for testing
- Use `serial_test` for tests touching global state
- Export JSON with `serde_wasm_bindgen`
- Return `Result<T, JsValue>` for fallible ops

## PLUGIN USAGE

```typescript
import init, { codeExtract, getCss } from '@devup-ui/wasm'

await init()
const { code, css, updatedBaseStyle } = codeExtract(
  'file.tsx', source, '@devup-ui/react', 
  'devup-ui', true, false, true
)

if (updatedBaseStyle) {
  await writeFile('devup-ui.css', getCss(null, false))
}
```
