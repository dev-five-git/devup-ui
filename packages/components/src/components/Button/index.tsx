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
  colors = {
    primary: 'var(--primary)',
    error: 'var(--error)',
  },
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
          ? `color-mix(in srgb, var(--p, var(--primary, --fallback-white)) 100%, #000 30%);`
          : isError
            ? 'var(--e, var(--error, --fallback-black))'
            : `color-mix(in srgb, var(--p, var(--primary, --fallback-black)) 20%, #FFF 80%);`,
        border:
          !isPrimary &&
          (isError
            ? '1px solid var(--e, var(--error, --fallback-black))'
            : `1px solid var(--p, var(--primary, --fallback-black))`),
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
          !isPrimary && isError
            ? 'var(--e, var(--error, --fallback-black))'
            : '$primaryFocus',
      }}
      _hover={{
        borderColor: isError
          ? 'var(--e, var(--error, --fallback-black))'
          : `var(--p, var(--primary, --fallback-black))`,
        bg: isPrimary
          ? `color-mix(in srgb, var(--p, var(--primary, --fallback-white)) 100%, #000 15%)`
          : isError
            ? '1px solid var(--e, var(--error, --fallback-black))'
            : `color-mix(in srgb, var(--p, var(--primary, --fallback-black)) 10%, #FFF 90%)`,
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
            ? `color-mix(in srgb, var(--p, var(--primary, --fallback-black)) 100%, #FFF 30%);`
            : isError
              ? 'var(--e, var(--error, --fallback-black))'
              : 'var(--p, var(--primary, --fallback-white))',
          border:
            !isPrimary &&
            (isError
              ? '1px solid var(--e, var(--error, --fallback-black))'
              : `1px solid var(--p, var(--primary, --fallback-white))`),
          color: !isPrimary && isError && '#FFF',
        },
        _hover: {
          bg: isPrimary
            ? `color-mix(in srgb, var(--p, var(--primary, --fallback-black)) 100%, #FFF 15%);`
            : isError
              ? '$inputBg'
              : `color-mix(in srgb, var(--p, var(--primary, --fallback-white)) 20%, #000 80%);`,
        },
        _focusVisible: {
          outlineColor: isPrimary
            ? `var(--p, var(--primary, --fallback-white))`
            : isError
              ? 'var(--e, var(--error, --fallback-black))'
              : '$primaryFocus',
        },
      }}
      bg={isPrimary ? 'var(--p, var(--primary, --fallback-black))' : '$inputBg'}
      border={isPrimary ? 'none' : '1px solid $border'}
      borderRadius={isPrimary ? '8px' : '10px'}
      className={className}
      color={
        isPrimary
          ? '#FFF'
          : isError
            ? 'var(--e, var(--error, --fallback-black))'
            : '$text'
      }
      cursor="pointer"
      outlineOffset="2px"
      px="40px"
      py="12px"
      styleOrder={1}
      styleVars={{
        '--p': colors.primary ?? 'var(--primary)',
        '--e': colors.error ?? 'var(--error)',
        '--fallback-white': '#FFF',
        '--fallback-black': '#000',
      }}
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
