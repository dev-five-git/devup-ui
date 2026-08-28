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

// The concrete default overload is a fast path only. The generic fallback
// must continue to infer intrinsic-element props from `as`.
const polymorphicBox = Box({
  as: 'a',
  bg: 'red',
  href: '/docs',
  onClick: (event) => event.currentTarget.href,
  target: '_blank',
})

function RequiredLink({ to }: { to: string }) {
  return to
}

const customComponentBox = Box({
  as: RequiredLink,
  props: { to: '/docs' },
})

export { boxProps, customComponentBox, customShorthandProps, polymorphicBox }
