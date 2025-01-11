import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiTransformProps {
  backfaceVisibility?: ResponsiveValue<Property.BackfaceVisibility>
  perspective?: ResponsiveValue<Property.Perspective>
  perspectiveOrigin?: ResponsiveValue<Property.PerspectiveOrigin>
  rotate?: ResponsiveValue<Property.Rotate>
  scale?: ResponsiveValue<Property.Scale>
  transform?: ResponsiveValue<Property.Transform>
  transformBox?: ResponsiveValue<Property.TransformBox>
  transformOrigin?: ResponsiveValue<Property.TransformOrigin>
  transformStyle?: ResponsiveValue<Property.TransformStyle>
  translate?: ResponsiveValue<Property.Translate>
}
