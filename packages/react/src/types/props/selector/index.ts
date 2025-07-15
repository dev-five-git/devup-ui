import type { ResponsiveValue } from '../../responsive-value'
import type { DevupTheme } from '../../theme'
import type { DevupCommonProps } from '../index'

type toPascalCase<S extends string> = S extends `${infer T}${infer U}`
  ? `${Uppercase<T>}${U}`
  : S

export type DevupThemeSelectorProps = keyof DevupTheme extends undefined
  ? Record<`_theme${string}`, DevupCommonProps & DevupSelectorProps>
  : {
      [K in keyof DevupTheme as `_theme${toPascalCase<K>}`]?: DevupCommonProps &
        DevupSelectorProps
    }

type SelectorProps = ResponsiveValue<
  | (DevupCommonProps & DevupSelectorProps & DevupThemeSelectorProps)
  | string
  | false
>

export interface DevupSelectorProps {
  _active?: SelectorProps
  _checked?: SelectorProps
  _default?: SelectorProps
  _disabled?: SelectorProps
  _empty?: SelectorProps
  _enabled?: SelectorProps
  _first?: SelectorProps
  _firstChild?: SelectorProps
  _firstOfType?: SelectorProps
  _focus?: SelectorProps
  _focusVisible?: SelectorProps
  _focusWithin?: SelectorProps
  _hover?: SelectorProps
  _invalid?: SelectorProps
  _lastChild?: SelectorProps
  _lastOfType?: SelectorProps
  _link?: SelectorProps
  _onlyChild?: SelectorProps
  _optional?: SelectorProps
  _readOnly?: SelectorProps
  _print?: SelectorProps

  _groupActive?: SelectorProps
  _groupChecked?: SelectorProps
  _groupDefault?: SelectorProps
  _groupDisabled?: SelectorProps
  _groupEmpty?: SelectorProps
  _groupEnabled?: SelectorProps
  _groupFirst?: SelectorProps
  _groupFirstChild?: SelectorProps
  _groupFirstOfType?: SelectorProps
  _groupFocus?: SelectorProps
  _groupFocusVisible?: SelectorProps
  _groupFocusWithin?: SelectorProps
  _groupHover?: SelectorProps
  _groupInvalid?: SelectorProps
  _groupLastChild?: SelectorProps
  _groupLastOfType?: SelectorProps
  _groupLink?: SelectorProps
  _groupOnlyChild?: SelectorProps
  _groupOptional?: SelectorProps
  _groupReadOnly?: SelectorProps

  // double separator
  _placeholder?: SelectorProps
  _before?: SelectorProps
  _after?: SelectorProps
  _highlight?: SelectorProps
  _viewTransition?: SelectorProps
  _viewTransitionGroup?: SelectorProps
  _viewTransitionImagePair?: SelectorProps
  _viewTransitionNew?: SelectorProps
  _viewTransitionOld?: SelectorProps

  selectors?: Record<string, SelectorProps>

  styleOrder?: number
}
