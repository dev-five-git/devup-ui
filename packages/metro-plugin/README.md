# @devup-ui/metro-plugin

> **Experimental (0.x).** React Native support is in progress
> ([#411](https://github.com/dev-five-git/devup-ui/issues/411)). This package
> is not published yet, and its API may change.

Compiles Devup UI for React Native in [Metro](https://metrobundler.dev/), the
bundler of React Native and Expo. You keep importing `Box`, `Text`, ... from
`@devup-ui/react`; at build time each element becomes a React Native component
and its style props become style objects hoisted into one `StyleSheet.create`
per file. With one theme nothing runs at runtime; with more, a tiny runtime
(`@devup-ui/react/native`) only tells which theme shows.

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

## Themes

`$` tokens of devup.json (colors, `length` and `shadow`) and `typography`
resolve at build time to the values React Native takes. With one theme each
becomes its literal value, and no file imports a runtime:

```tsx
<Box bg="$surface" p={4} />
// const __s = StyleSheet.create({ a: { backgroundColor: "#fff", padding: 16 } })
// <View style={__s.a} />
```

With more themes, the styles of every theme are precomputed, and the element
picks one by the index of the theme that shows; what does not differ by theme
stays shared:

```tsx
// <View style={[__s.a, [__s.b, __s.c][__t]]} />
```

The themes are the color variants of devup.json, the default first. The app
starts on the theme that follows the system appearance (React Native's
`Appearance`): while it is dark, `dark`, or else the first theme `colorScheme`
makes dark; otherwise the default theme. `_themeDark`, `_themeSepia`, ...
apply their styles in that theme only.

```tsx
import { getTheme, setTheme, useTheme } from '@devup-ui/react'

const theme = useTheme() // renders again when the theme changes
setTheme('sepia') // wins over the system appearance
```

The build imports these from `@devup-ui/react/native`, a store read with
`useSyncExternalStore` that computes no style; web builds never import it.
`setTheme` does not save the choice across launches.

## Limits of this version

Only styles the build can decide are supported: literals, values read at
runtime, theme tokens, conditions between them and the platform and theme
blocks. These are build errors, reported by Metro at the file, line and
column:

- responsive arrays (`p={[1, 2]}`), and tokens or typography with a value at
  each breakpoint
- interaction states (`_hover`, `_active`, `_pressed`, `_focus`, ...)
- `css()`, `styled()`, `globalCss()`, `keyframes()` and `Grid`
- any prop React Native has no style for
- with `runtime: false`, a value that differs by theme, `_theme*` with more
  than one theme, and `useTheme`, `getTheme` and `setTheme`

The runtime that brings responsive values and `_pressed` comes in a later
version.

Metro reads devup.json when it starts, so restart Metro after changing it
(`--reset-cache` is not needed).
