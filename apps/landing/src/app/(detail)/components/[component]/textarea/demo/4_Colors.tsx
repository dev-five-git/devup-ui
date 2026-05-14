import { Textarea } from '@devup-ui/components'

/**
 * **Colors**
 * Override the component color variables to match a custom theme.
 */
export default function Colors() {
  return (
    <Textarea
      colors={{
        primary: '#2563EB',
        border: '#BFDBFE',
        background: '#EFF6FF',
        placeholder: '#60A5FA',
        focusRing: 'rgba(37, 99, 235, 0.18)',
      }}
      placeholder="Custom themed textarea"
      rows={4}
    />
  )
}
