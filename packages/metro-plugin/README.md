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

## Responsive values

Responsive arrays, and `length`, `shadow` and `typography` tokens with a value
at each breakpoint, use the `breakpoints` of devup.json (the web's
`[0, 480, 768, 992, 1280, 1600]` by default), measured against the window
width in density-independent pixels. As on the web, each value applies from
its breakpoint on and `null` keeps the one before:

```tsx
<Box p={[1, null, 3]} bg="$surface" />
// <View style={[[__s.a, __s.b][__t], __b < 2 ? __s.c : __s.d]} />
```

The build precomputes one style object for each run of breakpoints that
share their styles, only for the styles that differ; a value that differs by
theme and breakpoint picks by both. The runtime listens to the window once
and renders a component again only when the window crosses into another
breakpoint. `_portrait` and `_landscape` apply their styles while the window
is in that orientation.

A value read at runtime in an array (`p={[x, 2]}`) applies from its
breakpoint on as it is.

## Press styles

`_active` (or its React Native name, `_pressed`) applies its styles while the
element is pressed. React Native tells only a `Pressable` whether it is
pressed, so the build passes it a function for its `style`:

```tsx
<Box onPress={open} bg="$primary" _active={{ opacity: 0.6 }} />
// renders
<Pressable onPress={open} style={({ pressed }) => [styles.a, pressed && styles.b]} />
```

A `Button` is already a `Pressable`. A `Box`, `Flex`, `Center` or `VStack`
with a press style becomes one, so it takes `onPress` and the other props of
`Pressable`. A press style works with `runtime: false` too, as it needs no
runtime. It is a build error on a `Text`, an `Image` or an `Input`, and with
`as`.

### Hover, focus and disabled

`_hover` and `_focus` apply while the pointer is over the element and while
it has focus. A `Pressable` tells these to its style function too
(`({ hovered, focused }) => ...`). React Native reports them where the
platform has a pointer or keyboard focus, such as the web through
react-native-web, and never on a phone's touch screen. An `Input` with
`_focus` renders through the runtime's `Focused`, which follows its
`onFocus` and `onBlur` and still calls yours. `_hover` on an `Input`, and any
of these on a `Text` or an `Image`, is a build error.

`_disabled` applies while the element's `disabled` prop is true:
`<Button disabled={busy} _disabled={{ opacity: 0.4 }} />` compiles to
`busy && styles.a`. On an element without `disabled` it never applies, and in
a `styled()` component it reads the component's `disabled` prop. In `css()`,
`_hover`, `_focus`, `_active` and `_disabled` are build errors: give them to
the element.

## Animations and transitions

[react-native-reanimated](https://docs.swmansion.com/react-native-reanimated/)
4 or later runs `keyframes()` and the `animation*` and `transition*` props,
as its CSS animations and transitions. Devup UI does not depend on it:
install it in the app (`npx expo install react-native-reanimated`). Without
it, they are build errors that say so.

```tsx
const fadeIn = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })

<Box animationName={fadeIn} animationDuration="400ms" />
<Image opacity={on ? 1 : 0.6} transition="opacity 300ms" src={logo} />
```

- `keyframes()` compiles to the keyframes object Reanimated reads, with each
  style converted as a prop is. A keyframe's styles cannot differ by theme,
  breakpoint, orientation or state.
- An element taking an animation or a transition renders Reanimated's
  animated component (`Animated.View`, `Animated.Text`, `Animated.Image`,
  or one created with `createAnimatedComponent`).
- A number of milliseconds stays a number: `transitionDuration={300}`.
- `transition` and `transitionProperty` name CSS properties, which compile
  to React Native's: `background-color` becomes `backgroundColor`.
- `animation` itself is a build error: write `animationName` and the other
  longhands.
- An element taking a press, hover or focus style cannot take an animation:
  animate an element inside it.
- With `as`, `css()` and a `styled()` component of a component other than
  Devup UI's, the component given must be animated, such as `Animated.View`.

## `css()` and `className`

`css()` compiles to the style it describes, the value React Native's `style`
takes, and `className` on a Devup UI element passes it on:

```tsx
const button = css({ borderRadius: '8px', px: 5 })

function Save() {
  const label = css({ color: '$text', fontSize: [14, null, 18] })
  return (
    <Button className={button} bg="$primary">
      <Text className={css(label, saving && { opacity: 0.5 })}>Save</Text>
    </Button>
  )
}
```

`css()` takes what it does on the web: objects of style props (with
responsive values, theme tokens and the theme, platform and orientation
blocks), conditions choosing between them, what another `css()` gives, and
CSS text (`` css`padding: 8px; background-color: $primary;` ``, each
interpolation a whole value). The element's style props apply after its
`className`, and its `style` prop last.

A `css()` whose styles differ by theme, breakpoint or orientation reads the
runtime, so it must be called in a function component or a hook. One that does
not can be called anywhere, such as at the top of the file. These are build
errors:

- a class name (`className="p-4"`, `css('p-4')`)
- a press style in `css()`: give `_active` to the element itself
- selectors, nesting and at-rules in CSS text

`css()` gives a style rather than a class on React Native, so pass it to
`className`, not to the `style` of a React Native component, which TypeScript
types as a string.

## `styled()`

`styled()` compiles to a function component that renders its target with the
styles it describes, before the `style` the component is given:

```tsx
const Card = styled(Box)({ bg: '$surface', p: [2, null, 4] })
const Title = styled(Text)`
  color: ${(props) => props.color};
  font-size: 18px;
`
// compiles to
const Card = (props) => {
  const t = useThemeIndex(), b = useBreakpoint()
  return <View {...props} style={[[s.a, s.b][t], b < 2 ? s.c : s.d, props.style]} />
}
```

It takes the parts `css()` takes, as `styled(Target)(...parts)`,
`` styled(Target)`css text` `` or `styled(Target, styles)`. A function in
them is called with the component's props: a value (`w: (p) => p.width`, or
an interpolation in CSS text), or a part giving what `css()` gives
(`(p) => p.active && css({ opacity: 0.5 })`). The target is a Devup UI
component, which keeps the styles it declares first (`Flex` its row), or any
React Native component. A press style makes a styled `Box` a `Pressable`.
These are build errors:

- a tag (`styled('div')`, `styled.div`)
- a function of the props giving style props (`(p) => ({ bg: p.color })`):
  return what `css()` gives instead
- a press style on a component other than a `Box` or a `Button`

The component reads style props only through `styled()`: on React Native,
`<Card p={1} />` passes `p` on to the target, so give it `style` or
`className` instead.

## Limits of this version

Only styles the build can decide are supported: literals, values read at
runtime, theme tokens, responsive values, conditions between them and the
platform, theme, orientation and interaction blocks, `css()`, `styled()`
and animations. These are build errors, reported by Metro at the file, line and column:

- selectors other than the states above (`_focusVisible`, `_checked`, `_before`, ...)
- `globalCss()` and `Grid`
- animations and transitions in an app without react-native-reanimated
- any prop React Native has no style for
- with `runtime: false`, a value that differs by theme or breakpoint,
  `_theme*` with more than one theme, `_portrait`, `_landscape`, and
  `useTheme`, `getTheme` and `setTheme`

Metro reads devup.json when it starts, so restart Metro after changing it
(`--reset-cache` is not needed).
