import { type DevupThemeTypography, Text as DevupText } from '@devup-ui/react'
import { clsx } from 'clsx'

interface LabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  // our custom properties.
  colors?: {
    text?: string
    error?: string
    disabled?: string
  }
  typography?: keyof DevupThemeTypography
  ellipsis?: boolean
  required?: boolean
  disabled?: boolean
  tooltip?: string
}

export function Label({
  form,
  htmlFor,
  colors,
  typography,
  ellipsis = false,
  required = false,
  disabled = false,
  tooltip,
  children,
  className,
  ...props
}: LabelProps): React.ReactElement {
  return (
    <DevupText
      as="label"
      className={clsx(ellipsis, className)}
      color={
        disabled
          ? colors?.disabled || '#999'
          : colors?.error || colors?.text || 'inherit'
      }
      cursor={disabled ? 'not-allowed' : 'default'}
      form={form}
      htmlFor={htmlFor}
      opacity={disabled ? 0.6 : 1}
      styleVars={{
        text: colors?.text,
        error: colors?.error,
        disabled: colors?.disabled,
      }}
      title={tooltip}
      typography={typography}
      {...props}
    >
      {children}
      {required && (
        <span style={{ color: colors?.error || 'red', marginLeft: '2px' }}>
          *
        </span>
      )}
    </DevupText>
  )
}
