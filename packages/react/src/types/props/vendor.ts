import type {
  VendorLonghandProperties,
  VendorShorthandProperties,
} from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export type DevupUiVendorProps = {
  [key in keyof (VendorLonghandProperties &
    VendorShorthandProperties)]: ResponsiveValue<
    (VendorLonghandProperties & VendorShorthandProperties)[key]
  >
}
