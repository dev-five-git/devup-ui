import type { Properties } from 'csstype-extra'

import { Box, type DevupProps } from '../src'
import type {
  DevupComponentAdditionalProps,
  DevupComponentBaseProps,
  DevupComponentMergedProps,
  DevupDefaultComponentMergedProps,
  DevupElementTypeProps,
} from '../src/types/props'
import type { DevupCssProperties } from '../src/types/props/generated-css-properties'
import type { ResponsiveValue } from '../src/types/responsive-value'

// Mirrors the module augmentation emitted to <distDir>/theme.d.ts.
declare module '../src' {
  interface DevupCustomShorthands {
    insetX?: DevupProps['w']
  }
}

const customShorthandProps: DevupProps = {
  insetX: [0, null, 'auto'],
  _hover: {
    insetX: 4,
  },
  selectors: {
    '& > *': {
      insetX: '$contentWidth',
    },
  },
}

const boxProps: Parameters<typeof Box>[0] = {
  insetX: 'auto',
  _focus: {
    insetX: [0, 4],
  },
}

// Polymorphic inference must come from `as` while preserving the exact native
// element props and their contextual event types.
Box({
  as: 'a',
  href: '/docs',
  onClick(event) {
    return event.currentTarget.href
  },
})

Box({
  as: 'button',
  onClick(event) {
    return event.currentTarget.disabled
  },
})

Box({ props: { id: 'box' } })

// @ts-expect-error polymorphic element props require the matching `as`
Box({ href: '/docs' })

// @ts-expect-error href is not a button prop
Box({ as: 'button', href: '/docs' })

function CustomLink(_props: { to: string }) {
  return null
}

Box({ as: CustomLink, props: { to: '/docs' } })

// @ts-expect-error required custom-component props stay required
Box({ as: CustomLink })

type Assert<T extends true> = T
type LegacyBaseProps<T extends React.ElementType> = DevupElementTypeProps<T> &
  DevupComponentAdditionalProps<T>
type IsEquivalent<T extends React.ElementType> =
  DevupComponentBaseProps<T> extends LegacyBaseProps<T>
    ? LegacyBaseProps<T> extends DevupComponentBaseProps<T>
      ? true
      : false
    : false

type _DivPropsStayEquivalent = Assert<IsEquivalent<'div'>>
type _AnchorPropsStayEquivalent = Assert<IsEquivalent<'a'>>
type _ButtonPropsStayEquivalent = Assert<IsEquivalent<'button'>>
type _CustomPropsStayEquivalent = Assert<IsEquivalent<typeof CustomLink>>

type OriginalCssProperties = {
  [K in keyof Properties]?: ResponsiveValue<Properties[K]>
}
type _GeneratedCssPropsStayEquivalent = Assert<
  DevupCssProperties extends OriginalCssProperties
    ? OriginalCssProperties extends DevupCssProperties
      ? true
      : false
    : false
>

type IntrinsicPropsStayEquivalent<T extends keyof React.JSX.IntrinsicElements> =
  DevupDefaultComponentMergedProps<T> extends DevupComponentMergedProps<T>
    ? DevupComponentMergedProps<T> extends DevupDefaultComponentMergedProps<T>
      ? keyof DevupDefaultComponentMergedProps<T> extends keyof DevupComponentMergedProps<T>
        ? keyof DevupComponentMergedProps<T> extends keyof DevupDefaultComponentMergedProps<T>
          ? true
          : false
        : false
      : false
    : false

type _DivIntrinsicPropsStayEquivalent = Assert<
  IntrinsicPropsStayEquivalent<'div'>
>
type _ButtonIntrinsicPropsStayEquivalent = Assert<
  IntrinsicPropsStayEquivalent<'button'>
>
type _ImageIntrinsicPropsStayEquivalent = Assert<
  IntrinsicPropsStayEquivalent<'img'>
>
type _InputIntrinsicPropsStayEquivalent = Assert<
  IntrinsicPropsStayEquivalent<'input'>
>
type _SpanIntrinsicPropsStayEquivalent = Assert<
  IntrinsicPropsStayEquivalent<'span'>
>

export { boxProps, customShorthandProps }
