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
      _active={{
        bg: isPrimary
          ? `color-mix(in srgb, var(--primary, #FFF) 100%, #000 30%);`
          : isError
            ? 'var(--error, #000)'
            : `color-mix(in srgb, var(--primary, #000) 20%, #FFF 80%);`,
        border:
          !isPrimary &&
          (isError
            ? '1px solid var(--error, #000)'
            : `1px solid var(--primary, #000)`),
        color: !isPrimary && isError && '#000',
      }}
      _disabled={{
        color: '#D6D7DE',
        bgColor: '#F0F0F3',
        cursor: 'not-allowed',
        borderColor: isPrimary ? 'transparent' : '$border',
      }}
      _focusVisible={{
        outline: '2px solid',
        outlineColor:
          !isPrimary && isError ? 'var(--error, #000)' : '$primaryFocus',
      }}
      _hover={{
        borderColor: isError ? 'var(--error, #000)' : `var(--primary, #000)`,
        bg: isPrimary
          ? `color-mix(in srgb, var(--primary, #FFF) 100%, #000 15%)`
          : isError
            ? '1px solid var(--error, #000)'
            : `color-mix(in srgb, var(--primary, #000) 10%, #FFF 90%)`,
      }}
      _themeDark={{
        _disabled: {
          color: '#373737',
          bgColor: '#47474A',
          cursor: 'not-allowed',
          borderColor: 'transparent',
        },
        _active: {
          bg: isPrimary
            ? `color-mix(in srgb, var(--primary, #000) 100%, #FFF 30%);`
            : isError
              ? 'var(--error, #000)'
              : 'var(--primary, #FFF)',
          border:
            !isPrimary &&
            (isError
              ? '1px solid var(--error, #000)'
              : `1px solid var(--primary, #FFF)`),
          color: !isPrimary && isError && '#FFF',
        },
        _hover: {
          bg: isPrimary
            ? `color-mix(in srgb, var(--primary, #000) 100%, #FFF 15%);`
            : isError
              ? '$inputBg'
              : `color-mix(in srgb, var(--primary, #FFF) 20%, #000 80%);`,
        },
        _focusVisible: {
          outlineColor: isPrimary
            ? `var(--primary, #FFF)`
            : isError
              ? 'var(--error, #000)'
              : '$primaryFocus',
        },
      }}
      bg={isPrimary ? 'var(--primary, #000)' : '$inputBg'}
      border={isPrimary ? 'none' : '1px solid $border'}
      borderRadius={isPrimary ? '8px' : '10px'}
      className={className}
      color={isPrimary ? '#FFF' : isError ? 'var(--error, #000)' : '$text'}
      cursor="pointer"
      outlineOffset="2px"
      px="40px"
      py="12px"
      styleOrder={1}
      styleVars={colors && { primary: colors.primary, error: colors.error }}
      transition=".25s"
      type={type}
      typography={
        isPrimary
          ? size === 's'
            ? 'buttonS'
            : 'buttonM'
          : isError
            ? 'inputBold'
            : 'buttonxs'
      }
      {...props}
    >
      {children}
    </DevupButton>
  )
}
