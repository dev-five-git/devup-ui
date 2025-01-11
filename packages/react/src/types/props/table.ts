import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiTableProps {
  borderSpacing?: ResponsiveValue<Property.BorderSpacing>
  captionSide?: ResponsiveValue<Property.CaptionSide>
  emptyCells?: ResponsiveValue<Property.EmptyCells>
  tableLayout?: ResponsiveValue<Property.TableLayout>
  verticalAlign?: ResponsiveValue<Property.VerticalAlign>
}
