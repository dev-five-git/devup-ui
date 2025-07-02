import { Button as DevupButton, css } from '@devup-ui/react'
import clsx from 'clsx'

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'default'
  colors?: {
    primary?: string
    error?: string
  }
  isError?: boolean
  size?: 'sm' | 'md'
}

const variants = {
  primary: css({
    styleOrder: 2,
    _active: {
      bg: `color-mix(in srgb, var(--primary, #FFF) 100%, #000 30%)`,
    },
    _disabled: {
      color: '#D6D7DE',
      bgColor: '#F0F0F3',
      cursor: 'not-allowed',
    },
    _hover: {
      bg: `color-mix(in srgb, var(--primary, #FFF) 100%, #000 15%)`,
    },
    _themeDark: {
      _active: {
        bg: `color-mix(in srgb, var(--primary, #000) 100%, #FFF 30%);`,
      },
      _disabled: {
        color: '#373737',
        bgColor: '#47474A',
        cursor: 'not-allowed',
        borderColor: 'transparent',
      },
      _hover: {
        bg: `color-mix(in srgb, var(--primary, #000) 100%, #FFF 15%);`,
        outlineColor: `var(--primary, #FFF)`,
      },
    },
    bg: 'var(--primary, #000)',
    border: 'none',
    borderRadius: '8px',
    color: '#FFF',
  }),
  default: css({
    styleOrder: 2,
    _active: {
      bg: `color-mix(in srgb, var(--primary, #000) 20%, #FFF 80%)`,
      border: `1px solid var(--primary, #000)`,
      color: '#000',
    },
    _hover: {
      borderColor: `var(--primary, #000)`,
      bg: `color-mix(in srgb, var(--primary, #000) 10%, #FFF 90%)`,
    },
    bg: '$inputBg',
    border: '1px solid $border',
    borderRadius: '10px',
    color: '$text',
  }),
}

const errorClassNames = css({
  styleOrder: 3,
  _active: {
    bg: 'var(--error, #000)',
    border: '1px solid var(--error, #000)',
    color: '#000',
  },
  _focusVisible: {
    outlineColor: 'var(--error, #000)',
  },
  _hover: {
    border: '1px solid var(--error, #000)',
  },
  _themeDark: {
    _active: {
      bg: 'var(--error, #000)',
      border: '1px solid var(--error, #000)',
      color: '#000',
    },
    _hover: {
      bg: '$inputBg',
    },
  },
})

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
  size = 'md',
  ...props
}: ButtonProps): React.ReactElement {
  const isPrimary = variant === 'primary'

  return (
    <DevupButton
      _disabled={{
        color: '#D6D7DE',
        bgColor: '#F0F0F3',
        cursor: 'not-allowed',
      }}
      _focusVisible={{
        outline: '2px solid',
        outlineColor: '$primaryFocus',
      }}
      _themeDark={{
        _disabled: {
          color: '#373737',
          bgColor: '#47474A',
          cursor: 'not-allowed',
          borderColor: 'transparent',
        },
        _focusVisible: {
          outlineColor: '$primaryFocus',
        },
      }}
      className={clsx(
        variants[variant],
        isError && variant === 'default' && errorClassNames,
        className,
      )}
      color={isError ? 'var(--error, #000)' : '$text'}
      cursor="pointer"
      outlineOffset="2px"
      px="40px"
      py="12px"
      styleOrder={1}
      styleVars={{ primary: colors?.primary, error: colors?.error }}
      transition=".25s"
      type={type}
      typography={
        isPrimary
          ? {
              sm: 'buttonS',
              md: 'buttonM',
            }[size]
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
