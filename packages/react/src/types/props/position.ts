import type { Property } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiPositionProps {
  pos?: ResponsiveValue<Property.Position>
  positioning?: ResponsiveValue<
    | 'top'
    | 'right'
    | 'bottom'
    | 'left'
    | 'top-right'
    | 'top-left'
    | 'bottom-left'
    | 'bottom-right'
  >
}
