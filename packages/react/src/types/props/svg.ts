import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiSvgProps {
  fill?: ResponsiveValue<Property.Fill>
  stroke?: ResponsiveValue<Property.Stroke>
  strokeWidth?: ResponsiveValue<Property.StrokeWidth>
  strokeOpacity?: ResponsiveValue<Property.StrokeOpacity>
}
