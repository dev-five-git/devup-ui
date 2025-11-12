import type { Properties } from 'csstype-extra'

import type { ResponsiveValue } from '../responsive-value'
import type { Merge } from '../utils'
import type { DevupUiBackgroundProps } from './background'
import type { DevupUiBorderProps } from './border'
import type { DevupUiBoxModelProps } from './box-model'
import type { DevupUiBoxSizingProps } from './box-sizing'
import type { DevupUiFlexProps } from './flex'
import type { DevupUiImageProps } from './image'
import type { DevupUiMaskProps } from './mask'
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
    DevupUiMaskProps,
    DevupUiTextProps {}

export type DevupCommonProps = Merge<
  {
    [K in keyof Properties]?: ResponsiveValue<Properties[K]>
  },
  DevupShortcutsProps
>

export interface DevupProps extends DevupCommonProps, DevupSelectorProps {}

export interface DevupPropsWithTheme
  extends DevupProps,
    DevupThemeSelectorProps {}

export interface DevupComponentProps<T extends React.ElementType>
  extends DevupPropsWithTheme {
  as?: T
  styleVars?: Record<string, string | undefined>
}
export type DevupComponentBaseProps<T extends React.ElementType> =
  DevupElementTypeProps<T> & DevupComponentAdditionalProps<T>

export type DevupElementTypeProps<T extends React.ElementType> =
  T extends string ? React.ComponentProps<T> : object

export type DevupComponentAdditionalProps<
  T extends React.ElementType,
  P extends React.ComponentProps<T> = React.ComponentProps<T>,
> = (Partial<P> extends P
  ? {
      props?: FilterChildren<P>
    }
  : {
      props: FilterChildren<P>
    }) &
  (P extends { children: infer U }
    ? {
        children: U
      }
    : P extends { children?: infer U }
      ? {
          children?: U
        }
      : object)

type FilterChildren<T> = Omit<T, 'children'>
