import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiImageProps {
  imageOrientation?: ResponsiveValue<Property.ImageOrientation>
  imageRendering?: ResponsiveValue<Property.ImageRendering>
  imageResolution?: ResponsiveValue<Property.ImageResolution>
  objectFit?: ResponsiveValue<Property.ObjectFit>
  objectPosition?: ResponsiveValue<Property.ObjectPosition>
}
