import { ProgressBar } from '@devup-ui/components'

/**
 * **Default**
 * A bar filled to `value` out of `max` (100 by default). Name it with `aria-label` or `aria-labelledby`; it is exposed as a `progressbar` with its range.
 */
export default function Default() {
  return <ProgressBar aria-label="Upload progress" value={40} />
}
