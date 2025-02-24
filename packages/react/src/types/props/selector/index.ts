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
  _active?: DevupCommonProps | false
  _checked?: DevupCommonProps | false
  _default?: DevupCommonProps | false
  _disabled?: DevupCommonProps | false
  _empty?: DevupCommonProps | false
  _enabled?: DevupCommonProps | false
  _first?: DevupCommonProps | false
  _firstChild?: DevupCommonProps | false
  _firstOfType?: DevupCommonProps | false
  _focus?: DevupCommonProps | false
  _focusVisible?: DevupCommonProps | false
  _focusWithin?: DevupCommonProps | false
  _hover?: DevupCommonProps | false
  _invalid?: DevupCommonProps | false
  _lastChild?: DevupCommonProps | false
  _lastOfType?: DevupCommonProps | false
  _link?: DevupCommonProps | false
  _onlyChild?: DevupCommonProps | false
  _optional?: DevupCommonProps | false
  _readOnly?: DevupCommonProps | false
  _print?: DevupCommonProps | false

  _groupActive?: DevupCommonProps | false
  _groupChecked?: DevupCommonProps | false
  _groupDefault?: DevupCommonProps | false
  _groupDisabled?: DevupCommonProps | false
  _groupEmpty?: DevupCommonProps | false
  _groupEnabled?: DevupCommonProps | false
  _groupFirst?: DevupCommonProps | false
  _groupFirstChild?: DevupCommonProps | false
  _groupFirstOfType?: DevupCommonProps | false
  _groupFocus?: DevupCommonProps | false
  _groupFocusVisible?: DevupCommonProps | false
  _groupFocusWithin?: DevupCommonProps | false
  _groupHover?: DevupCommonProps | false
  _groupInvalid?: DevupCommonProps | false
  _groupLastChild?: DevupCommonProps | false
  _groupLastOfType?: DevupCommonProps | false
  _groupLink?: DevupCommonProps | false
  _groupOnlyChild?: DevupCommonProps | false
  _groupOptional?: DevupCommonProps | false
  _groupReadOnly?: DevupCommonProps | false

  // double separator
  _placeholder?: DevupCommonProps | false
  _before?: DevupCommonProps | false
  _after?: DevupCommonProps | false
  _highlight?: DevupCommonProps | false
  _viewTransition?: DevupCommonProps | false
  _viewTransitionGroup?: DevupCommonProps | false
  _viewTransitionImagePair?: DevupCommonProps | false
  _viewTransitionNew?: DevupCommonProps | false
  _viewTransitionOld?: DevupCommonProps | false

  selectors?: Record<string, DevupCommonProps>

  styleOrder?: number
}
