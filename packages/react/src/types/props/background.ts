import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBackgroundProps {
  bg?: ResponsiveValue<Property.Background>
  background?: ResponsiveValue<Property.Background>
  bgAttachment?: ResponsiveValue<Property.BackgroundAttachment>
  backgroundAttachment?: ResponsiveValue<Property.BackgroundAttachment>
  bgClip?: ResponsiveValue<Property.BackgroundClip>
  backgroundClip?: ResponsiveValue<Property.BackgroundClip>
  bgColor?: ResponsiveValue<Property.BackgroundColor>
  backgroundColor?: ResponsiveValue<Property.BackgroundColor>
  bgImage?: ResponsiveValue<Property.BackgroundImage>
  backgroundImage?: ResponsiveValue<Property.BackgroundImage>
  bgOrigin?: ResponsiveValue<Property.BackgroundOrigin>
  backgroundOrigin?: ResponsiveValue<Property.BackgroundOrigin>
  bgPosition?: ResponsiveValue<Property.BackgroundPosition>
  backgroundPosition?: ResponsiveValue<Property.BackgroundPosition>
  bgPositionX?: ResponsiveValue<Property.BackgroundPositionX>
  backgroundPositionX?: ResponsiveValue<Property.BackgroundPositionX>
  bgPositionY?: ResponsiveValue<Property.BackgroundPositionY>
  backgroundPositionY?: ResponsiveValue<Property.BackgroundPositionY>
  bgRepeat?: ResponsiveValue<Property.BackgroundRepeat>
  backgroundRepeat?: ResponsiveValue<Property.BackgroundRepeat>
  bgSize?: ResponsiveValue<Property.BackgroundSize>
  backgroundSize?: ResponsiveValue<Property.BackgroundSize>
}
