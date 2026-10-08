import type { CssProp } from '@devup-ui/react/compat'

declare module 'react' {
  interface Attributes {
    css?: CssProp
  }
}
