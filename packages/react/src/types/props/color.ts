import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { DevupThemeColors } from '../theme'

export interface DevupUiColorProps {
  colorScheme?: ResponsiveValue<Property.ColorScheme>
  forcedColorAdjust?: ResponsiveValue<Property.ForcedColorAdjust>
  printColorAdjust?: ResponsiveValue<Property.PrintColorAdjust>

  color?: ResponsiveValue<Property.Color | keyof DevupThemeColors>
  opacity?: ResponsiveValue<Property.Opacity>
  visibility?: ResponsiveValue<Property.Visibility>
}
