import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiBoxSizingProps {
  aspectRatio?: ResponsiveValue<Property.AspectRatio>
  boxSizing?: ResponsiveValue<Property.BoxSizing>
  containIntrinsicBlockSize?: ResponsiveValue<Property.ContainIntrinsicBlockSize>
  containIntrinsicHeight?: ResponsiveValue<Property.ContainIntrinsicHeight>
  containIntrinsicInlineSize?: ResponsiveValue<Property.ContainIntrinsicInlineSize>
  containIntrinsicSize?: ResponsiveValue<Property.ContainIntrinsicSize>
  containIntrinsicWidth?: ResponsiveValue<Property.ContainIntrinsicWidth>

  height?: ResponsiveValue<Property.Height>
  maxHeight?: ResponsiveValue<Property.MaxHeight>
  maxWidth?: ResponsiveValue<Property.MaxWidth>
  minHeight?: ResponsiveValue<Property.MinHeight>
  minWidth?: ResponsiveValue<Property.MinWidth>
  width?: ResponsiveValue<Property.Width>

  h?: ResponsiveValue<Property.Height>
  maxH?: ResponsiveValue<Property.MaxHeight>
  maxW?: ResponsiveValue<Property.MaxWidth>
  minH?: ResponsiveValue<Property.MinHeight>
  minW?: ResponsiveValue<Property.MinWidth>
  w?: ResponsiveValue<Property.Width>

  // width and height
  boxSize?: ResponsiveValue<Property.Width | Property.Height>
}
