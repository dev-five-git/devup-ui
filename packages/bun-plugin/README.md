<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor for Bun
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/bun-plugin'>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/bun-plugin">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/bun-plugin.svg?style=flat"/>
</a>
<a href="https://badgen.net/github/stars/dev-five-git/devup-ui">
<img alt="Github Stars" src="https://badgen.net/github/stars/dev-five-git/devup-ui" />
</a>
<a href="https://discord.gg/8zjcGc7cWh">
<img alt="Discord" src="https://img.shields.io/discord/1321362173619994644.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</a>
</div>

---

## Install

```sh
bun add @devup-ui/react @devup-ui/bun-plugin
```

## Usage

Add the zero-config entry to Bun's preload list:

```toml
# bunfig.toml
[test]
preload = ["@devup-ui/bun-plugin"]
```

## Custom Shorthands

To configure custom shorthands, preload a local module instead of the
zero-config entry:

```toml
# bunfig.toml
[test]
preload = ["./devup-ui.preload.ts"]
```

```ts
// devup-ui.preload.ts
import { register } from '@devup-ui/bun-plugin/register'

await register({
  shorthands: {
    insetX: ['left', 'right'],
    scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
  },
})
```

Custom shorthands are plugin configuration, not theme tokens. Every target
receives the same value, and target names may use camelCase or kebab-case. The
generated `df/theme.d.ts` provides type completion for component props,
responsive values, and selectors. Restart Bun after changing the option.
If `tsconfig.json` only includes your source directory, add `df/*.d.ts` to
`include`.

```tsx
<Box insetX={[0, null, 'auto']} _hover={{ insetX: 4 }} />
```

## Features

- Zero Config
- Zero FOUC (Flash of Unstyled Content)
- Zero Runtime
- Full Bun bundler integration
- TypeScript support with generated theme types
- CSS extraction at build time

## Theme Configuration

Create a `devup.json` file in your project root:

```json
{
  "theme": {
    "colors": {
      "default": {
        "text": "#000",
        "background": "#fff"
      },
      "dark": {
        "text": "#fff",
        "background": "#000"
      }
    }
  }
}
```

The plugin will generate `df/theme.d.ts` with TypeScript types for your theme.

## How It Works

The plugin transforms your JSX/TSX code at build time:

```tsx
// Before
<Box bg="red" color="$text" />

// After
<div className="d0 d1" />
```

Generated CSS:

```css
.d0 {
  background-color: red;
}
.d1 {
  color: var(--text);
}
```

## License

Apache-2.0
