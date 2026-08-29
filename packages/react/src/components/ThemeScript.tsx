import type { DevupTheme } from '../types/theme'
import type { Conditional } from '../types/utils'

interface ThemeScriptProps {
  auto?: boolean
  theme?: Conditional<DevupTheme>
}

function escapeScriptClosingTag(script: string) {
  return script.replace(/<\/script/giu, '\\u003c/script')
}

export function ThemeScript({ auto = true, theme }: ThemeScriptProps) {
  const script = theme
    ? `(function (){document.documentElement.setAttribute('data-theme',${JSON.stringify(theme)});}())`
    : `(function (){document.documentElement.setAttribute('data-theme',localStorage.getItem('__DF_THEME_SELECTED__')||(${String(auto)}&&window.matchMedia('(prefers-color-scheme:dark)').matches?'dark':${JSON.stringify(process.env.DEVUP_UI_DEFAULT_THEME ?? 'default')}));})()`

  return <script>{escapeScriptClosingTag(script)}</script>
}
