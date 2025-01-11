import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiScrollbarProps {
  scrollbarWidth?: ResponsiveValue<Property.ScrollbarWidth>
  scrollbarColor?: ResponsiveValue<Property.ScrollbarColor>
}
