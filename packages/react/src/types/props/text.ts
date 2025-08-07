import type { DevupThemeTypography } from '../typography'
import type { Conditional } from '../utils'

export interface DevupUiTextProps {
  typography?: Conditional<DevupThemeTypography>
}
