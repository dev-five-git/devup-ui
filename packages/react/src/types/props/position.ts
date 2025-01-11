import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiPositionProps {
  top?: ResponsiveValue<Property.Top>
  right?: ResponsiveValue<Property.Right>
  bottom?: ResponsiveValue<Property.Bottom>
  left?: ResponsiveValue<Property.Left>
  inset?: ResponsiveValue<Property.Inset>
  insetInline?: ResponsiveValue<Property.InsetInline>
  insetInlineStart?: ResponsiveValue<Property.InsetInlineStart>
  insetInlineEnd?: ResponsiveValue<Property.InsetInlineEnd>
  insetBlock?: ResponsiveValue<Property.InsetBlock>
  insetBlockStart?: ResponsiveValue<Property.InsetBlockStart>
  insetBlockEnd?: ResponsiveValue<Property.InsetBlockEnd>
  float?: ResponsiveValue<Property.Float>
  clear?: ResponsiveValue<Property.Clear>
  position?: ResponsiveValue<Property.Position>
  zIndex?: ResponsiveValue<Property.ZIndex>
}
