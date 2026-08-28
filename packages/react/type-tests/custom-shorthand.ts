import { Box, type DevupProps } from '../src'
import type {
  DevupComponentAdditionalProps,
  DevupComponentBaseProps,
  DevupElementTypeProps,
} from '../src/types/props'

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

export { boxProps, customShorthandProps }
