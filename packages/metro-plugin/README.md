# @devup-ui/metro-plugin

> **Experimental (0.x).** React Native support is in progress
> ([#411](https://github.com/dev-five-git/devup-ui/issues/411)). This package
> is not published yet, and its API may change.

Compiles Devup UI for React Native in [Metro](https://metrobundler.dev/), the
bundler of React Native and Expo. You keep importing `Box`, `Text`, ... from
`@devup-ui/react`; at build time each element becomes a React Native component
and its style props become style objects hoisted into one `StyleSheet.create`
per file. Nothing runs at runtime.

## Setup

```js
// metro.config.js (Expo)
const { getDefaultConfig } = require('expo/metro-config')
const { withDevupUI } = require('@devup-ui/metro-plugin')

module.exports = withDevupUI(getDefaultConfig(__dirname))
```

```js
// metro.config.js (React Native CLI)
const { getDefaultConfig } = require('@react-native/metro-config')
const { withDevupUI } = require('@devup-ui/metro-plugin')

module.exports = withDevupUI(getDefaultConfig(__dirname))
```

```tsx
import { Box, Flex, Text } from '@devup-ui/react'

export function Card() {
  return (
    <Flex bg="#fff" borderRadius="12px" gap={2} p={4}>
      <Text color="#333" fontSize="16px">
        Hello
      </Text>
      <Box bg="red" h="8px" w="8px" />
    </Flex>
  )
}
```

`withDevupUI` sets `transformer.babelTransformerPath` to its own transformer.
The transformer compiles each file that imports Devup UI for the platform
Metro builds (`ios`, `android` or `web`) and then hands the code to the
transformer the config had: Expo's, React Native's, or the one you name.

| Element | React Native |
| --- | --- |
| `Box`, `Flex`, `VStack`, `Center` | `View` (`Flex`, `VStack` and `Center` add their flex styles) |
| `Text` | `Text` |
| `Image` | `Image` (`src` becomes `source`) |
| `Input` | `TextInput` |
| `Button` | `Pressable` |

`_ios`, `_android` and `_web` apply their styles on that platform only.

## Options

```js
withDevupUI(config, {
  // The package Devup UI is imported from. Default: '@devup-ui/react'
  package: '@devup-ui/react',
  // devup.json, from the project root. Default: 'devup.json'
  devupFile: 'devup.json',
  // Packages in node_modules that ship uncompiled Devup UI. Default: []
  include: [],
  // Whether styles that need the React Native runtime may import it.
  // Default: true. false is a strict zero-runtime mode.
  runtime: true,
  // 'native' (default) or 'wasm'
  engine: 'native',
  // The Babel transformer the compiled code goes to.
  // Default: the config's babelTransformerPath, else Expo's, else React Native's.
  upstreamTransformer: undefined,
})
```

## Limits of this version

Only styles the build can decide are supported: literals, values read at
runtime, conditions between them and the platform blocks. These are build
errors, reported by Metro at the file, line and column:

- responsive arrays (`p={[1, 2]}`), theme tokens (`color="$primary"`),
  typography and `_dark`/`_light`/`_theme*`
- interaction states (`_hover`, `_active`, `_pressed`, `_focus`, ...)
- `css()`, `styled()`, `globalCss()`, `keyframes()` and `Grid`
- any prop React Native has no style for

The runtime that brings responsive values, themes and `_pressed` comes in a
later version.

Metro reads devup.json when it starts, so restart Metro after changing it
(`--reset-cache` is not needed).
