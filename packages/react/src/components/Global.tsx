import type { GlobalCssProps } from '../utils/global-css'

interface GlobalProps {
  styles?: GlobalCssProps
}

/**
 * Emotion compatible global stylesheet component.
 *
 * The extractor lifts `styles` into the stylesheet at build time and strips the
 * prop, so the rendered component is inert — it exists only as a render site.
 */
export function Global(_props: GlobalProps) {
  return null
}
