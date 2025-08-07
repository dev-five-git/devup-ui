import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBoxSizingProps {
  h?: ResponsiveValue<Property.Height>
  maxH?: ResponsiveValue<Property.MaxHeight>
  maxW?: ResponsiveValue<Property.MaxWidth>
  minH?: ResponsiveValue<Property.MinHeight>
  minW?: ResponsiveValue<Property.MinWidth>
  w?: ResponsiveValue<Property.Width>

  // width and height
  boxSize?: ResponsiveValue<Property.Width | Property.Height>
}
