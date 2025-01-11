import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiGridProps {
  gridTemplateColumns?: ResponsiveValue<Property.GridTemplateColumns>
  gridTemplateRows?: ResponsiveValue<Property.GridTemplateRows>
  gridTemplateAreas?: ResponsiveValue<Property.GridTemplateAreas>
  gridTemplate?: ResponsiveValue<Property.GridTemplate>
  gridAutoColumns?: ResponsiveValue<Property.GridAutoColumns>
  gridAutoRows?: ResponsiveValue<Property.GridAutoRows>
  gridAutoFlow?: ResponsiveValue<Property.GridAutoFlow>
  grid?: ResponsiveValue<Property.Grid>
  gridRowStart?: ResponsiveValue<Property.GridRowStart>
  gridColumnStart?: ResponsiveValue<Property.GridColumnStart>
  gridRowEnd?: ResponsiveValue<Property.GridRowEnd>
  gridColumnEnd?: ResponsiveValue<Property.GridColumnEnd>
  gridRow?: ResponsiveValue<Property.GridRow>
  gridColumn?: ResponsiveValue<Property.GridColumn>
  gridArea?: ResponsiveValue<Property.GridArea>
  gridRowGap?: ResponsiveValue<Property.GridRowGap>
  gridColumnGap?: ResponsiveValue<Property.GridColumnGap>
  gridGap?: ResponsiveValue<Property.GridGap>
}
