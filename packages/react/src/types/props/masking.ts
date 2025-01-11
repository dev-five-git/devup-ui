import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiMaskingProps {
  clipPath?: ResponsiveValue<Property.ClipPath>
  clipRule?: ResponsiveValue<Property.ClipRule>
  mask?: ResponsiveValue<Property.Mask>
  maskBorder?: ResponsiveValue<Property.MaskBorder>
  maskBorderMode?: ResponsiveValue<Property.MaskBorderMode>
  maskBorderOutset?: ResponsiveValue<Property.MaskBorderOutset>
  maskBorderRepeat?: ResponsiveValue<Property.MaskBorderRepeat>
  maskBorderSlice?: ResponsiveValue<Property.MaskBorderSlice>
  maskBorderSource?: ResponsiveValue<Property.MaskBorderSource>
  maskBorderWidth?: ResponsiveValue<Property.MaskBorderWidth>
  maskClip?: ResponsiveValue<Property.MaskClip>
  maskComposite?: ResponsiveValue<Property.MaskComposite>
  maskImage?: ResponsiveValue<Property.MaskImage>
  maskMode?: ResponsiveValue<Property.MaskMode>
  maskOrigin?: ResponsiveValue<Property.MaskOrigin>
  maskPosition?: ResponsiveValue<Property.MaskPosition>
  maskRepeat?: ResponsiveValue<Property.MaskRepeat>
  maskSize?: ResponsiveValue<Property.MaskSize>
  maskType?: ResponsiveValue<Property.MaskType>
}
