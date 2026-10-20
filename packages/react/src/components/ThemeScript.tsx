import type { ComponentProps } from 'react'

import type { DevupTheme } from '../types/theme'
import type { Conditional } from '../types/utils'

interface ThemeScriptProps extends Omit<
  ComponentProps<'script'>,
  'children' | 'dangerouslySetInnerHTML'
> {
  auto?: boolean
  theme?: Conditional<DevupTheme>
}

function escapeScriptClosingTag(script: string) {
  return script.replace(/<\/script/giu, '\\u003c/script')
}

/**
 * Sets `data-theme` before the page paints. Reading the saved theme or the
 * system preference can throw (storage blocked, no `matchMedia`), so each read
 * falls back to the default theme. The script follows the system while no theme
 * was chosen, and the choice another tab saves. Pass `nonce` under a
 * nonce-based CSP.
 */
export function ThemeScript({
  auto = true,
  theme,
  ...props
}: ThemeScriptProps) {
  const fallback = JSON.stringify(
    process.env.DEVUP_UI_DEFAULT_THEME ?? 'default',
  )
  const script = theme
    ? `(function (){document.documentElement.setAttribute('data-theme',${JSON.stringify(theme)});}())`
    : `(function (){var d=document.documentElement,k='__DF_THEME_SELECTED__',q='(prefers-color-scheme:dark)';function r(){var t=null;try{t=localStorage.getItem(k)}catch(e){}if(!t){try{if(${String(auto)}&&window.matchMedia(q).matches)t='dark'}catch(e){}}d.setAttribute('data-theme',t||${fallback})}r();try{window.addEventListener('storage',function(e){if(e.key===k||e.key===null)r()})}catch(e){}if(${String(auto)})try{window.matchMedia(q).addEventListener('change',r)}catch(e){}})()`

  return <script {...props}>{escapeScriptClosingTag(script)}</script>
}
