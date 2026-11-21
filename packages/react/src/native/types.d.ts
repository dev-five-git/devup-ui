/**
 * Types `@devup-ui/react` for React Native: each component takes the props of
 * the React Native component the build renders it as, instead of those of the
 * HTML element it renders on the web. `@devup-ui/metro-plugin` imports this
 * file from `<distDir>/native.d.ts`; to opt in by hand, import it from a
 * declaration file of the project:
 *
 * ```ts
 * import '@devup-ui/react/native/types'
 * ```
 */
import type {
  ImageProps,
  PressableProps,
  TextInputProps,
  TextProps,
  ViewProps,
} from 'react-native'

/** What every element takes besides its component's props */
interface DevupNativeElementProps {
  /** The class of `css()`, whose styles the build gives the element */
  className?: string
}

declare module '@devup-ui/react' {
  interface DevupElementProps {
    /** `Box`, `Flex`, `VStack`, `Center` and `Grid`: a `View` */
    div: ViewProps & DevupNativeElementProps
    /** `Text`: a `Text` */
    span: TextProps & DevupNativeElementProps
    /** `Image`: an `Image` */
    img: ImageProps & DevupNativeElementProps
    /** `Input`: a `TextInput` */
    input: TextInputProps & DevupNativeElementProps
    /** `Button`: a `Pressable` */
    button: PressableProps & DevupNativeElementProps
  }
}
