import { ProgressBar } from '@devup-ui/components'

/**
 * **Colors**
 * Use the `colors` prop to change the filled part (`primary`), the empty part (`track`) and the value text (`text`).
 */
export default function Colors() {
  return (
    <ProgressBar
      aria-label="Storage used"
      colors={{ primary: '#2CA353', track: '#D9F2E1', text: '#2CA353' }}
      showValue
      value={72}
    />
  )
}
