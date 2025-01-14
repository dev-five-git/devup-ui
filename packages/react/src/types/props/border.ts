import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { DevupThemeColors } from '../theme'

export interface DevupUiBorderProps {
  border?: ResponsiveValue<Property.Border>
  borderBottom?: ResponsiveValue<Property.BorderBottom>
  borderBottomColor?: ResponsiveValue<
    Property.BorderBottomColor | keyof DevupThemeColors
  >
  borderBottomLeftRadius?: ResponsiveValue<Property.BorderBottomLeftRadius>
  borderBottomRightRadius?: ResponsiveValue<Property.BorderBottomRightRadius>
  borderBottomStyle?: ResponsiveValue<Property.BorderBottomStyle>
  borderBottomWidth?: ResponsiveValue<Property.BorderBottomWidth>
  borderCollapse?: ResponsiveValue<Property.BorderCollapse>
  borderColor?: ResponsiveValue<Property.BorderColor | keyof DevupThemeColors>
  borderImage?: ResponsiveValue<Property.BorderImage>
  borderImageOutset?: ResponsiveValue<Property.BorderImageOutset>
  borderImageRepeat?: ResponsiveValue<Property.BorderImageRepeat>
  borderImageSlice?: ResponsiveValue<Property.BorderImageSlice>
  borderImageSource?: ResponsiveValue<Property.BorderImageSource>
  borderImageWidth?: ResponsiveValue<Property.BorderImageWidth>
  borderLeft?: ResponsiveValue<Property.BorderLeft>
  borderLeftColor?: ResponsiveValue<
    Property.BorderLeftColor | keyof DevupThemeColors
  >
  borderLeftStyle?: ResponsiveValue<Property.BorderLeftStyle>
  borderLeftWidth?: ResponsiveValue<Property.BorderLeftWidth>
  borderRadius?: ResponsiveValue<Property.BorderRadius>
  borderRight?: ResponsiveValue<Property.BorderRight>
  borderRightColor?: ResponsiveValue<
    Property.BorderRightColor | keyof DevupThemeColors
  >
  borderRightStyle?: ResponsiveValue<Property.BorderRightStyle>
  borderRightWidth?: ResponsiveValue<Property.BorderRightWidth>
  borderStyle?: ResponsiveValue<Property.BorderStyle>
  borderTop?: ResponsiveValue<Property.BorderTop>
  borderTopColor?: ResponsiveValue<
    Property.BorderTopColor | keyof DevupThemeColors
  >
  borderTopLeftRadius?: ResponsiveValue<Property.BorderTopLeftRadius>
  borderTopRightRadius?: ResponsiveValue<Property.BorderTopRightRadius>
  borderTopStyle?: ResponsiveValue<Property.BorderTopStyle>
  borderTopWidth?: ResponsiveValue<Property.BorderTopWidth>
  borderWidth?: ResponsiveValue<Property.BorderWidth>
  boxShadow?: ResponsiveValue<Property.BoxShadow>
}
