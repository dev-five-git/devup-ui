import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBoxModelProps {
  margin?: ResponsiveValue<Property.Margin>
  marginBottom?: ResponsiveValue<Property.MarginBottom>
  marginLeft?: ResponsiveValue<Property.MarginLeft>
  marginRight?: ResponsiveValue<Property.MarginRight>
  marginTop?: ResponsiveValue<Property.MarginTop>

  m?: ResponsiveValue<Property.Margin>
  mb?: ResponsiveValue<Property.MarginBottom>
  ml?: ResponsiveValue<Property.MarginLeft>
  mr?: ResponsiveValue<Property.MarginRight>
  mt?: ResponsiveValue<Property.MarginTop>

  padding?: ResponsiveValue<Property.Padding>
  paddingBottom?: ResponsiveValue<Property.PaddingBottom>
  paddingLeft?: ResponsiveValue<Property.PaddingLeft>
  paddingRight?: ResponsiveValue<Property.PaddingRight>
  paddingTop?: ResponsiveValue<Property.PaddingTop>

  p?: ResponsiveValue<Property.Padding>
  pb?: ResponsiveValue<Property.PaddingBottom>
  pl?: ResponsiveValue<Property.PaddingLeft>
  pr?: ResponsiveValue<Property.PaddingRight>
  pt?: ResponsiveValue<Property.PaddingTop>
}
