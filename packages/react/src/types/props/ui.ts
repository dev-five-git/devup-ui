import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { DevupThemeColors } from '../theme'

export interface DevupUiUiProps {
  accentColor?: ResponsiveValue<Property.AccentColor>
  appearance?: ResponsiveValue<Property.Appearance>
  caret?: ResponsiveValue<Property.Caret>
  caretColor?: ResponsiveValue<Property.CaretColor>
  caretShape?: ResponsiveValue<Property.CaretShape>
  cursor?: ResponsiveValue<Property.Cursor>
  outline?: ResponsiveValue<Property.Outline>
  outlineColor?: ResponsiveValue<Property.OutlineColor | keyof DevupThemeColors>
  outlineStyle?: ResponsiveValue<Property.OutlineStyle>
  outlineWidth?: ResponsiveValue<Property.OutlineWidth>
  outlineOffset?: ResponsiveValue<Property.OutlineOffset>
  pointerEvents?: ResponsiveValue<Property.PointerEvents>
  resize?: ResponsiveValue<Property.Resize>
  userSelect?: ResponsiveValue<Property.UserSelect>
}
