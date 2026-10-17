import type { DevupProps } from '../src'
import type { StyleXRawStyles } from '../src/utils/stylex'

type Assert<T extends true> = T
type DeadCssProperty =
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

export type DeadPropertiesAreRejected = Assert<
  Extract<DeadCssProperty, keyof DevupProps> extends never ? true : false
>

export type DeadStyleXPropertiesAreRejected = Assert<
  Extract<DeadCssProperty, keyof StyleXRawStyles> extends never ? true : false
>

export const compatibilityProperties = {
  WebkitBoxAlign: 'center',
  WebkitBoxFlex: 2,
  WebkitBoxOrient: 'vertical',
  strokeColor: 'red',
  imeMode: 'active',
  scrollSnapType: 'x mandatory',
  _hover: { WebkitBoxFlex: 3 },
} satisfies DevupProps
