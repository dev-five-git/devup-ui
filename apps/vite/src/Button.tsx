import { Button as DevupButton } from '@devup-ui/react'

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'default'
  colors?: {
    primary?: string
    error?: string
  }
  isError?: boolean
  size?: 's' | 'm'
}

/**
 * Button
 * ## Design Token
 * ### Color
 * - inputDisabledBackground
 * - inputDisabled
 * - inputBackground
 * - primaryHover
 * - text
 * - border
 *
 * @constructor
 */
export function Button({
  variant = 'default',
  className = '',
  type = 'button',
  colors,
  isError = false,
  children,
  size = 'm',
  ...props
}: ButtonProps): React.ReactElement {
  const isPrimary = variant === 'primary'

  return (
    <DevupButton
      _hover={{
        borderColor: true ? 'blue' : ``,
      }}
      bg="red"
      className={className}
    >
      {children}
    </DevupButton>
  )
}
