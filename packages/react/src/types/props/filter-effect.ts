import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiFilterEffectProps {
  backdropFilter?: ResponsiveValue<Property.BackdropFilter>
  filter?: ResponsiveValue<Property.Filter>
}
