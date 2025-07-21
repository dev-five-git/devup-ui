import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { DevupThemeColors } from '../theme'

export interface DevupUiSvgProps {
  fill?: ResponsiveValue<Property.Fill | keyof DevupThemeColors>
  stroke?: ResponsiveValue<Property.Stroke | keyof DevupThemeColors>
  strokeWidth?: ResponsiveValue<Property.StrokeWidth>
  strokeOpacity?: ResponsiveValue<Property.StrokeOpacity>
}
