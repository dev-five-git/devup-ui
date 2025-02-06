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

export interface DevupSelectorProps {
  _active?: DevupCommonProps
  _checked?: DevupCommonProps
  _default?: DevupCommonProps
  _disabled?: DevupCommonProps
  _empty?: DevupCommonProps
  _enabled?: DevupCommonProps
  _first?: DevupCommonProps
  _firstChild?: DevupCommonProps
  _firstOfType?: DevupCommonProps
  _focus?: DevupCommonProps
  _focusVisible?: DevupCommonProps
  _focusWithin?: DevupCommonProps
  _hover?: DevupCommonProps
  _invalid?: DevupCommonProps
  _lastChild?: DevupCommonProps
  _lastOfType?: DevupCommonProps
  _link?: DevupCommonProps
  _onlyChild?: DevupCommonProps
  _optional?: DevupCommonProps
  _readOnly?: DevupCommonProps
  _print?: DevupCommonProps

  _groupActive?: DevupCommonProps
  _groupChecked?: DevupCommonProps
  _groupDefault?: DevupCommonProps
  _groupDisabled?: DevupCommonProps
  _groupEmpty?: DevupCommonProps
  _groupEnabled?: DevupCommonProps
  _groupFirst?: DevupCommonProps
  _groupFirstChild?: DevupCommonProps
  _groupFirstOfType?: DevupCommonProps
  _groupFocus?: DevupCommonProps
  _groupFocusVisible?: DevupCommonProps
  _groupFocusWithin?: DevupCommonProps
  _groupHover?: DevupCommonProps
  _groupInvalid?: DevupCommonProps
  _groupLastChild?: DevupCommonProps
  _groupLastOfType?: DevupCommonProps
  _groupLink?: DevupCommonProps
  _groupOnlyChild?: DevupCommonProps
  _groupOptional?: DevupCommonProps
  _groupReadOnly?: DevupCommonProps

  // double separator
  _placeholder?: DevupCommonProps
  _before?: DevupCommonProps
  _after?: DevupCommonProps
  _highlight?: DevupCommonProps
  _viewTransition?: DevupCommonProps
  _viewTransitionGroup?: DevupCommonProps
  _viewTransitionImagePair?: DevupCommonProps
  _viewTransitionNew?: DevupCommonProps
  _viewTransitionOld?: DevupCommonProps
}
