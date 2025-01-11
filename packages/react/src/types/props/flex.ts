import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiFlexProps {
  flex?: ResponsiveValue<Property.Flex>
  flexBasis?: ResponsiveValue<Property.FlexBasis>
  flexDirection?: ResponsiveValue<Property.FlexDirection>
  flexFlow?: ResponsiveValue<Property.FlexFlow>
  flexGrow?: ResponsiveValue<Property.FlexGrow>
  flexShrink?: ResponsiveValue<Property.FlexShrink>
  flexWrap?: ResponsiveValue<Property.FlexWrap>
  order?: ResponsiveValue<Property.Order>

  flexDir?: ResponsiveValue<Property.FlexDirection>
}
