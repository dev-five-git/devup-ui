import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiColorProps {
  colorScheme?: ResponsiveValue<Property.ColorScheme>
  forcedColorAdjust?: ResponsiveValue<Property.ForcedColorAdjust>
  printColorAdjust?: ResponsiveValue<Property.PrintColorAdjust>

  color?: ResponsiveValue<Property.Color>
  opacity?: ResponsiveValue<Property.Opacity>
}
