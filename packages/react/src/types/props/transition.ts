import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiTransitionProps {
  transition?: ResponsiveValue<Property.Transition>
  transitionDelay?: ResponsiveValue<Property.TransitionDelay>
  transitionDuration?: ResponsiveValue<Property.TransitionDuration>
  transitionProperty?: ResponsiveValue<Property.TransitionProperty>
  transitionTimingFunction?: ResponsiveValue<Property.TransitionTimingFunction>
}
