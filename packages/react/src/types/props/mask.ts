import type { Property } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiMaskProps {
  maskPos?: ResponsiveValue<Property.MaskPosition>
  maskImg?: ResponsiveValue<Property.MaskImage>
}
