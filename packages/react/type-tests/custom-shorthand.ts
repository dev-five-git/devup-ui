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

export { boxProps, customShorthandProps }
