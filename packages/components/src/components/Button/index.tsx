import { Button as DevupButton } from '@devup-ui/react'

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'default'
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
  ...props
}: ButtonProps): React.ReactElement {
  const isPrimary = variant === 'primary'
  return (
    <DevupButton
      _active={{
        bg: isPrimary
          ? 'color-mix(in srgb, $primary 100%, #000 30%);'
          : 'color-mix(in srgb, $primary 20%, #FFF 80%); ',
        border: isPrimary ? undefined : '1px solid $primary',
      }}
      _disabled={{
        color: '#D6D7DE',
        bgColor: '#F0F0F3',
        cursor: 'not-allowed',
        borderColor: '$border',
      }}
      _focusVisible={{
        outline: '2px solid',
        outlineColor: isPrimary
          ? 'color-mix(in srgb, $primary 80%, #000 20%)'
          : '$primary',
      }}
      _hover={{
        borderColor: '$primary',
        bg: isPrimary
          ? 'color-mix(in srgb, $primary 100%, #000 15%);'
          : 'color-mix(in srgb, $primary 10%, #FFF 90%);',
      }}
      _themeDark={{
        _disabled: {
          color: '#555',
          bgColor: '#414244',
          cursor: 'not-allowed',
          borderColor: '$border',
        },
        _active: {
          bg: isPrimary
            ? 'color-mix(in srgb, $primary 100%, #FFF 30%);'
            : 'color-mix(in srgb, $primary 30%, #000 70%);',
          border: isPrimary ? undefined : '1px solid $primary',
        },
        _hover: {
          bg: isPrimary
            ? 'color-mix(in srgb, $primary 100%, #FFF 15%);'
            : 'color-mix(in srgb, $primary 20%, #000 80%);',
        },
        _focusVisible: {
          outlineColor: isPrimary
            ? 'color-mix(in srgb, color-mix(in srgb, $primary 50%, #fff 50%) 90%, #fff 90%)'
            : '$primary',
        },
      }}
      bg={variant === 'primary' ? '$primary' : '$inputBg'}
      border="1px solid var(--border, #EEE)"
      borderRadius="8px"
      className={className}
      color={variant === 'primary' ? '#FFF' : '$text'}
      cursor="pointer"
      outlineOffset="2px"
      px="40px"
      py="12px"
      styleOrder={1}
      transition=".25s"
      type={type}
      {...props}
    />
  )
}
