import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBoxAlignProps {
  justifyContent?: ResponsiveValue<Property.JustifyContent>
  alignContent?: ResponsiveValue<Property.AlignContent>
  placeContent?: ResponsiveValue<Property.PlaceContent>
  justifyItems?: ResponsiveValue<Property.JustifyItems>
  alignItems?: ResponsiveValue<Property.AlignItems>
  placeItems?: ResponsiveValue<Property.PlaceItems>
  justifySelf?: ResponsiveValue<Property.JustifySelf>
  alignSelf?: ResponsiveValue<Property.AlignSelf>
  placeSelf?: ResponsiveValue<Property.PlaceSelf>
  rowGap?: ResponsiveValue<Property.RowGap>
  columnGap?: ResponsiveValue<Property.ColumnGap>
  gap?: ResponsiveValue<Property.Gap>
}
