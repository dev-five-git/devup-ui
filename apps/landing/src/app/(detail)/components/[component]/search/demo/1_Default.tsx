import { Search } from '@devup-ui/components'

/**
 * **Default**
 * A search input with a magnifier icon. Name it with `aria-label` (or a `<label>`) since it usually has no visible label. A clear button appears once there is a query, and `Escape` clears it too.
 */
export default function Default() {
  return <Search aria-label="Search" placeholder="Search" />
}
