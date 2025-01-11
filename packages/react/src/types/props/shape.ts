import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiShapeProps {
  shapeOutside?: ResponsiveValue<Property.ShapeOutside>
  shapeMargin?: ResponsiveValue<Property.ShapeMargin>
  shapeImageThreshold?: ResponsiveValue<Property.ShapeImageThreshold>
}
