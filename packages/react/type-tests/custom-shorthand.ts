import { Box, type DevupProps } from '../src'

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

export { boxProps, customShorthandProps }
