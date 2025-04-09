import type { Conditional } from 'src/types/utils'

import type { DevupTheme } from '../types/theme'

interface ThemeScriptProps {
  auto?: boolean
  theme?: Conditional<DevupTheme>
}

export function ThemeScript({ auto = true, theme }: ThemeScriptProps) {
  return (
    <script
      dangerouslySetInnerHTML={{
        __html: theme
          ? `(function (){document.documentElement.setAttribute('data-theme','${theme}');}())`
          : `(function (){document.documentElement.setAttribute('data-theme',localStorage.getItem('__DF_THEME_SELECTED__')||(${String(auto)}&&window.matchMedia('(prefers-color-scheme:dark)').matches?'dark':'${process.env.DEVUP_UI_DEFAULT_THEME ?? 'default'}'));})()`,
      }}
    />
  )
}
