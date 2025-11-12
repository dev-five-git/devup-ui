import type { Property } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBorderProps {
  borderBottomRadius?: ResponsiveValue<
    Property.BorderBottomRightRadius | Property.BorderBottomLeftRadius
  >
  borderLeftRadius?: ResponsiveValue<
    Property.BorderBottomLeftRadius | Property.BorderTopLeftRadius
  >
  borderRightRadius?: ResponsiveValue<
    Property.BorderBottomRightRadius | Property.BorderTopRightRadius
  >
  borderTopRadius?: ResponsiveValue<
    Property.BorderTopRightRadius | Property.BorderTopLeftRadius
  >
}
