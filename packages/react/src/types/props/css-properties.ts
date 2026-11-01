import type { Properties } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'

type Responsive<T> = { [K in keyof T]?: ResponsiveValue<T[K]> }

type UnsupportedProperties =
  | 'boxAlign'
  | 'boxPack'
  | 'boxFlex'
  | 'boxFlexGroup'
  | 'boxOrient'
  | 'boxOrdinalGroup'
  | 'boxDirection'
  | 'boxLines'
  | 'flexOrder'
  | 'flexPositive'
  | 'flexNegative'
  | 'flexPreferredSize'
  | 'scrollSnapCoordinate'
  | 'scrollSnapDestination'
  | 'scrollSnapPointsX'
  | 'scrollSnapPointsY'
  | 'scrollSnapTypeX'
  | 'scrollSnapTypeY'

export type SupportedCssProperties = Omit<Properties, UnsupportedProperties>

// Keep an interface boundary so consumers can reuse cached type relationships.
// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface DevupCssProperties extends Responsive<SupportedCssProperties> {}
