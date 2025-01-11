import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBorderProps {
  border?: ResponsiveValue<Property.Border>
  borderBottom?: ResponsiveValue<Property.BorderBottom>
  borderBottomColor?: ResponsiveValue<Property.BorderBottomColor>
  borderBottomLeftRadius?: ResponsiveValue<Property.BorderBottomLeftRadius>
  borderBottomRightRadius?: ResponsiveValue<Property.BorderBottomRightRadius>
  borderBottomStyle?: ResponsiveValue<Property.BorderBottomStyle>
  borderBottomWidth?: ResponsiveValue<Property.BorderBottomWidth>
  borderCollapse?: ResponsiveValue<Property.BorderCollapse>
  borderColor?: ResponsiveValue<Property.BorderColor>
  borderImage?: ResponsiveValue<Property.BorderImage>
  borderImageOutset?: ResponsiveValue<Property.BorderImageOutset>
  borderImageRepeat?: ResponsiveValue<Property.BorderImageRepeat>
  borderImageSlice?: ResponsiveValue<Property.BorderImageSlice>
  borderImageSource?: ResponsiveValue<Property.BorderImageSource>
  borderImageWidth?: ResponsiveValue<Property.BorderImageWidth>
  borderLeft?: ResponsiveValue<Property.BorderLeft>
  borderLeftColor?: ResponsiveValue<Property.BorderLeftColor>
  borderLeftStyle?: ResponsiveValue<Property.BorderLeftStyle>
  borderLeftWidth?: ResponsiveValue<Property.BorderLeftWidth>
  borderRadius?: ResponsiveValue<Property.BorderRadius>
  borderRight?: ResponsiveValue<Property.BorderRight>
  borderRightColor?: ResponsiveValue<Property.BorderRightColor>
  borderRightStyle?: ResponsiveValue<Property.BorderRightStyle>
  borderRightWidth?: ResponsiveValue<Property.BorderRightWidth>
  borderStyle?: ResponsiveValue<Property.BorderStyle>
  borderTop?: ResponsiveValue<Property.BorderTop>
  borderTopColor?: ResponsiveValue<Property.BorderTopColor>
  borderTopLeftRadius?: ResponsiveValue<Property.BorderTopLeftRadius>
  borderTopRightRadius?: ResponsiveValue<Property.BorderTopRightRadius>
  borderTopStyle?: ResponsiveValue<Property.BorderTopStyle>
  borderTopWidth?: ResponsiveValue<Property.BorderTopWidth>
  borderWidth?: ResponsiveValue<Property.BorderWidth>
  boxShadow?: ResponsiveValue<Property.BoxShadow>
}
