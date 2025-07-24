import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { DevupThemeColors } from '../theme'

export interface DevupUiBackgroundProps {
  bg?: ResponsiveValue<Property.Background | keyof DevupThemeColors>
  bgAttachment?: ResponsiveValue<Property.BackgroundAttachment>
  bgClip?: ResponsiveValue<Property.BackgroundClip>
  bgColor?: ResponsiveValue<Property.BackgroundColor | keyof DevupThemeColors>
  bgImage?: ResponsiveValue<Property.BackgroundImage>
  bgOrigin?: ResponsiveValue<Property.BackgroundOrigin>
  bgPosition?: ResponsiveValue<Property.BackgroundPosition>
  bgPositionX?: ResponsiveValue<Property.BackgroundPositionX>
  bgPositionY?: ResponsiveValue<Property.BackgroundPositionY>
  bgPos?: ResponsiveValue<Property.BackgroundPosition>
  bgPosX?: ResponsiveValue<Property.BackgroundPositionX>
  bgPosY?: ResponsiveValue<Property.BackgroundPositionY>
  bgRepeat?: ResponsiveValue<Property.BackgroundRepeat>
  bgSize?: ResponsiveValue<Property.BackgroundSize>
  bgBlendMode?: ResponsiveValue<Property.BackgroundBlendMode>
}
