import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiListProps {
  listStyleImage?: ResponsiveValue<Property.ListStyleImage>
  listStyleType?: ResponsiveValue<Property.ListStyleType>
  listStylePosition?: ResponsiveValue<Property.ListStylePosition>
  listStyle?: ResponsiveValue<Property.ListStyle>
}
