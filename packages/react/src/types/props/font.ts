import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiFontProps {
  font?: ResponsiveValue<Property.Font>
  fontFamily?: ResponsiveValue<Property.FontFamily>
  fontFeatureSettings?: ResponsiveValue<Property.FontFeatureSettings>
  fontKerning?: ResponsiveValue<Property.FontKerning>
  fontLanguageOverride?: ResponsiveValue<Property.FontLanguageOverride>
  fontOpticalSizing?: ResponsiveValue<Property.FontOpticalSizing>
  fontSize?: ResponsiveValue<Property.FontSize | number>
  fontSizeAdjust?: ResponsiveValue<Property.FontSizeAdjust>
  fontStretch?: ResponsiveValue<Property.FontStretch>
  fontStyle?: ResponsiveValue<Property.FontStyle>
  fontSynthesis?: ResponsiveValue<Property.FontSynthesis>
  fontVariant?: ResponsiveValue<Property.FontVariant>
  fontVariantAlternates?: ResponsiveValue<Property.FontVariantAlternates>
  fontVariantCaps?: ResponsiveValue<Property.FontVariantCaps>
  fontVariantEastAsian?: ResponsiveValue<Property.FontVariantEastAsian>
  fontVariantLigatures?: ResponsiveValue<Property.FontVariantLigatures>
  fontVariantNumeric?: ResponsiveValue<Property.FontVariantNumeric>
  fontVariantPosition?: ResponsiveValue<Property.FontVariantPosition>
  fontVariationSettings?: ResponsiveValue<Property.FontVariationSettings>
  fontWeight?: ResponsiveValue<Property.FontWeight>
  lineHeight?: ResponsiveValue<Property.LineHeight>
}
