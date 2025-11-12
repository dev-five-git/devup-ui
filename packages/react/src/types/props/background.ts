import type { Property } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBackgroundProps {
  bg?: ResponsiveValue<Property.Background>
  bgAttachment?: ResponsiveValue<Property.BackgroundAttachment>
  bgClip?: ResponsiveValue<Property.BackgroundClip>
  bgColor?: ResponsiveValue<Property.BackgroundColor>
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
  backgroundImg?: ResponsiveValue<Property.BackgroundImage>
  bgImg?: ResponsiveValue<Property.BackgroundImage>
}
