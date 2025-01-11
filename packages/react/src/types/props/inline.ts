import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiInlineProps {
  alignmentBaseline?: ResponsiveValue<Property.AlignmentBaseline>
  dominantBaseline?: ResponsiveValue<Property.DominantBaseline>
  initialLetter?: ResponsiveValue<Property.InitialLetter>
  lineHeight?: ResponsiveValue<Property.LineHeight>
}
