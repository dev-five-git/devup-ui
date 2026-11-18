import { Search } from '@devup-ui/components'

/**
 * **Disabled**
 * Use the `disabled` prop to prevent searching. The clear button is hidden and `Escape` keeps the query.
 */
export default function Disabled() {
  return (
    <Search
      aria-label="Search"
      defaultValue="devup"
      disabled
      placeholder="Search"
    />
  )
}
