import type { Property } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'

export interface DevupUiAnimationProps {
  animation?: ResponsiveValue<Property.Animation>
  animationComposition?: ResponsiveValue<Property.AnimationComposition>
  animationDelay?: ResponsiveValue<Property.AnimationDelay>
  animationDirection?: ResponsiveValue<Property.AnimationDirection>
  animationDir?: ResponsiveValue<Property.AnimationDirection>
  animationDuration?: ResponsiveValue<Property.AnimationDuration>
  animationFillMode?: ResponsiveValue<Property.AnimationFillMode>
  animationIterationCount?: ResponsiveValue<Property.AnimationIterationCount>
  animationName?: ResponsiveValue<Property.AnimationName>
  animationPlayState?: ResponsiveValue<Property.AnimationPlayState>
  animationTimingFunction?: ResponsiveValue<Property.AnimationTimingFunction>
  animationTimeline?: ResponsiveValue<Property.AnimationTimeline>
}
