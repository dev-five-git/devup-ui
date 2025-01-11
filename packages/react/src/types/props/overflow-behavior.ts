import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiOverflowBehaviorProps {
  overscrollBehavior?: ResponsiveValue<Property.OverscrollBehavior>
  overscrollBehaviorBlock?: ResponsiveValue<Property.OverscrollBehaviorBlock>
  overscrollBehaviorInline?: ResponsiveValue<Property.OverscrollBehaviorInline>
  overscrollBehaviorX?: ResponsiveValue<Property.OverscrollBehaviorX>
  overscrollBehaviorY?: ResponsiveValue<Property.OverscrollBehaviorY>
}
