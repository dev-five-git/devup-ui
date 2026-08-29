import type { DevupCustomShorthands } from '../shorthand'
import type { Merge } from '../utils'
import type { DevupUiBackgroundProps } from './background'
import type { DevupUiBorderProps } from './border'
import type { DevupUiBoxModelProps } from './box-model'
import type { DevupUiBoxSizingProps } from './box-sizing'
import type { DevupUiFlexProps } from './flex'
import type { DevupCssProperties } from './generated-css-properties'
import type { DevupUiImageProps } from './image'
import type { DevupUiMaskProps } from './mask'
import type { DevupUiMotionPathProps } from './motion-path'
import type { DevupUiPositionProps } from './position'
import type { DevupSelectorProps, DevupThemeSelectorProps } from './selector'
import type { DevupUiTextProps } from './text'

export interface DevupShortcutsProps
  extends
    DevupUiBackgroundProps,
    DevupUiBorderProps,
    DevupUiBoxModelProps,
    DevupUiBoxSizingProps,
    DevupUiFlexProps,
    DevupUiImageProps,
    DevupUiMotionPathProps,
    DevupUiPositionProps,
    DevupUiMaskProps,
    DevupUiTextProps,
    DevupCustomShorthands {}

export type DevupCommonProps = Merge<DevupCssProperties, DevupShortcutsProps>

export interface DevupProps extends DevupCommonProps, DevupSelectorProps {}

export interface DevupPropsWithTheme
  extends DevupProps, DevupThemeSelectorProps {}

export interface DevupComponentProps<
  T extends React.ElementType,
> extends DevupPropsWithTheme {
  as?: T
  styleVars?: Record<string, string | undefined>
}

export type DevupComponentMergedProps<T extends React.ElementType> = Merge<
  DevupComponentBaseProps<T>,
  DevupComponentProps<T>
>

export type DevupPolymorphicComponentMergedProps<T extends React.ElementType> =
  DevupComponentMergedProps<T> & { as: T }

type DevupDefaultIntrinsicPropConflicts =
  | 'color'
  | 'content'
  | 'height'
  | 'translate'
  | 'width'
  | keyof DevupCustomShorthands

// Component declarations put this overload first for fast resolution and last
// so Parameters<typeof Component> keeps describing the default element.
export type DevupDefaultComponentMergedProps<
  T extends keyof React.JSX.IntrinsicElements,
> = Omit<React.ComponentProps<T>, DevupDefaultIntrinsicPropConflicts> & {
  props?: FilterChildren<React.ComponentProps<T>>
} & DevupComponentProps<T>

export type DevupComponentBaseProps<T extends React.ElementType> =
  NoInfer<T> extends string
    ? React.ComponentProps<NoInfer<T>> & {
        props?: FilterChildren<React.ComponentProps<NoInfer<T>>>
      }
    : DevupComponentAdditionalProps<NoInfer<T>>

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
