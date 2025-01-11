import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiOverflowProps {
  overflow?: ResponsiveValue<Property.Overflow>
  overflowBlock?: ResponsiveValue<Property.OverflowBlock>
  overflowClipMargin?: ResponsiveValue<Property.OverflowClipMargin>
  overflowInline?: ResponsiveValue<Property.OverflowInline>
  overflowX?: ResponsiveValue<Property.OverflowX>
  overflowY?: ResponsiveValue<Property.OverflowY>
  scrollBehavior?: ResponsiveValue<Property.ScrollBehavior>
  scrollbarGutter?: ResponsiveValue<Property.ScrollbarGutter>
  textOverflow?: ResponsiveValue<Property.TextOverflow>
}
