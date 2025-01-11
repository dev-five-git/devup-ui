import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiMotionPathProps {
  offset?: ResponsiveValue<Property.Offset>
  offsetAnchor?: ResponsiveValue<Property.OffsetAnchor>
  offsetDistance?: ResponsiveValue<Property.OffsetDistance>
  offsetPath?: ResponsiveValue<Property.OffsetPath>
  offsetPosition?: ResponsiveValue<Property.OffsetPosition>
  offsetRotate?: ResponsiveValue<Property.OffsetRotate>
}
