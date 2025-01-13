import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBoxModelProps {
  margin?: ResponsiveValue<Property.Margin>
  marginBottom?: ResponsiveValue<Property.MarginBottom>
  marginLeft?: ResponsiveValue<Property.MarginLeft>
  marginRight?: ResponsiveValue<Property.MarginRight>
  marginTop?: ResponsiveValue<Property.MarginTop>

  m?: ResponsiveValue<Property.Margin>
  mx?: ResponsiveValue<Property.MarginLeft | Property.MarginRight>
  my?: ResponsiveValue<Property.MarginTop | Property.MarginBottom>
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
  px?: ResponsiveValue<Property.PaddingLeft | Property.PaddingRight>
  py?: ResponsiveValue<Property.PaddingTop | Property.PaddingBottom>
  pb?: ResponsiveValue<Property.PaddingBottom>
  pl?: ResponsiveValue<Property.PaddingLeft>
  pr?: ResponsiveValue<Property.PaddingRight>
  pt?: ResponsiveValue<Property.PaddingTop>
}
