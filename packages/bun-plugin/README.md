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

### With Bun.build()

```typescript
import { DevupUI, getDevupDefine } from '@devup-ui/bun-plugin'

await Bun.build({
  entrypoints: ['./src/index.tsx'],
  outdir: './dist',
  plugins: [DevupUI()],
  define: getDevupDefine(),
})
```

### Configuration Options

```typescript
import { DevupUI } from '@devup-ui/bun-plugin'

DevupUI({
  // Package to import components from (default: '@devup-ui/react')
  package: '@devup-ui/react',

  // CSS directory path (default: 'df/devup-ui')
  cssDir: 'df/devup-ui',

  // Theme configuration file path (default: 'devup.json')
  devupFile: 'devup.json',

  // Distribution directory for generated files (default: 'df')
  distDir: 'df',

  // Enable CSS extraction and transformation (default: true)
  extractCss: true,

  // Enable debug logging (default: false)
  debug: false,

  // Additional packages to include in processing (default: [])
  include: [],

  // Merge all CSS into single file (default: false)
  singleCss: false,

  // CSS class name prefix (default: undefined)
  prefix: 'my-prefix',
})
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
