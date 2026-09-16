import type { GlobalCssProps } from './global-css'

/**
 * styled-components compatible global style declaration.
 *
 * The CSS is extracted at build time exactly like `globalCss`, and the call is
 * replaced with a component that renders nothing — so an existing
 * `<GlobalStyle />` render site keeps working without a runtime.
 */
export function createGlobalStyle(
  strings: TemplateStringsArray,
  ...values: (string | number | boolean | null | undefined)[]
): () => null

export function createGlobalStyle(props: GlobalCssProps): () => null

export function createGlobalStyle(
  _strings?: TemplateStringsArray | GlobalCssProps,
  ..._values: (string | number | boolean | null | undefined)[]
): () => null {
  throw new Error('Cannot run on the runtime')
}
