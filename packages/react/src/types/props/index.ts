import type { Properties } from 'csstype'

import type { ResponsiveValue } from '../responsive-value'
import type { Merge } from '../utils'
import type { DevupUiBackgroundProps } from './background'
import type { DevupUiBorderProps } from './border'
import type { DevupUiBoxModelProps } from './box-model'
import type { DevupUiBoxSizingProps } from './box-sizing'
import type { DevupUiFlexProps } from './flex'
import type { DevupUiImageProps } from './image'
import type { DevupUiMotionPathProps } from './motion-path'
import type { DevupUiPositionProps } from './position'
import type { DevupSelectorProps, DevupThemeSelectorProps } from './selector'
import type { DevupUiTextProps } from './text'

export interface DevupShortcutsProps
  extends DevupUiBackgroundProps,
    DevupUiBorderProps,
    DevupUiBoxModelProps,
    DevupUiBoxSizingProps,
    DevupUiFlexProps,
    DevupUiImageProps,
    DevupUiMotionPathProps,
    DevupUiPositionProps,
    DevupUiTextProps {}

export type DevupCommonProps = Merge<
  {
    [K in keyof Properties]?: ResponsiveValue<Properties[K]>
  },
  DevupShortcutsProps
>

export interface DevupProps
  extends DevupCommonProps,
    DevupSelectorProps,
    DevupThemeSelectorProps {}

export interface DevupComponentProps<T extends React.ElementType>
  extends DevupProps {
  as?: T
  styleVars?: Record<string, string | undefined>
}
