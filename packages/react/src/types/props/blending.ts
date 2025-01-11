import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBlendingProps {
  backgroundBlendMode?: ResponsiveValue<Property.BackgroundBlendMode>
  isolation?: ResponsiveValue<Property.Isolation>
  mixBlendMode?: ResponsiveValue<Property.MixBlendMode>
}
