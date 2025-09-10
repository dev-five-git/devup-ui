import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiMaskProps {
  maskPos?: ResponsiveValue<Property.MaskPosition>
  maskImg?: ResponsiveValue<Property.MaskImage>
}
