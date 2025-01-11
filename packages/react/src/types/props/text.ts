import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiTextProps {
  hangingPunctuation?: ResponsiveValue<Property.HangingPunctuation>
  hyphenateLimitChars?: ResponsiveValue<Property.HyphenateLimitChars>
  hyphens?: ResponsiveValue<Property.Hyphens>
  lineBreak?: ResponsiveValue<Property.LineBreak>
  overflowWrap?: ResponsiveValue<Property.OverflowWrap>
  tabSize?: ResponsiveValue<Property.TabSize>
  textAlignLast?: ResponsiveValue<Property.TextAlignLast>
  textJustify?: ResponsiveValue<Property.TextJustify>
  textSizeAdjust?: ResponsiveValue<Property.TextSizeAdjust>
  textWrap?: ResponsiveValue<Property.TextWrap>
  whiteSpaceCollapse?: ResponsiveValue<Property.WhiteSpaceCollapse>
  wordBreak?: ResponsiveValue<Property.WordBreak>

  letterSpacing?: ResponsiveValue<Property.LetterSpacing>
  textAlign?: ResponsiveValue<Property.TextAlign>
  textDecoration?: ResponsiveValue<Property.TextDecoration>
  textDecorationColor?: ResponsiveValue<Property.TextDecorationColor>
  textDecorationLine?: ResponsiveValue<Property.TextDecorationLine>
  textDecorationStyle?: ResponsiveValue<Property.TextDecorationStyle>
  textEmphasis?: ResponsiveValue<Property.TextEmphasis>
  textEmphasisColor?: ResponsiveValue<Property.TextEmphasisColor>
  textEmphasisPosition?: ResponsiveValue<Property.TextEmphasisPosition>
  textEmphasisStyle?: ResponsiveValue<Property.TextEmphasisStyle>
  textIndent?: ResponsiveValue<Property.TextIndent>
  textRendering?: ResponsiveValue<Property.TextRendering>
  textShadow?: ResponsiveValue<Property.TextShadow>
  textTransform?: ResponsiveValue<Property.TextTransform>
  whiteSpace?: ResponsiveValue<Property.WhiteSpace>
  wordSpacing?: ResponsiveValue<Property.WordSpacing>
}
