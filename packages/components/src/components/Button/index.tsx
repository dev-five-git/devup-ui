import { Box, Button as DevupButton, Center, css } from '@devup-ui/react'
import clsx from 'clsx'

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'default'
  colors?: {
    primary?: string
    error?: string
    text?: string
    border?: string
    inputBg?: string
    primaryFocus?: string
    white?: string
  }
  isError?: boolean
  size?: 'sm' | 'md'
  icon?: React.ReactNode
  ellipsis?: boolean
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
    border: 'none',
    borderRadius: '8px',
    bg: 'var(--primary, #000)',
    color: 'var(--white, #FFF)',
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
    _disabled: {
      color: '#D6D7DE',
      bgColor: '#F0F0F3',
      cursor: 'not-allowed',
      borderColor: 'var(--border, #000)',
    },
    bg: 'var(--inputBg, #FFF)',
    border: '1px solid var(--border, #000)',
    typography: 'buttonxs',
    borderRadius: '10px',
    _themeDark: {
      _disabled: {
        color: '#373737',
        bgColor: '#47474A',
        cursor: 'not-allowed',
        borderColor: 'transparent',
      },
      _hover: {
        borderColor: `var(--primary, #000)`,
        bg: `color-mix(in srgb, var(--primary, #FFF) 10%, var(--inputBg, #000) 90%)`,
      },
      _active: {
        bg: 'var(--primary, #000)',
        color: 'var(--text, #FFF)',
      },
    },
  }),
}

const errorClassNames = css({
  styleOrder: 3,
  color: 'var(--error, #000)',
  _active: {
    bg: 'var(--error, #000)',
    border: '1px solid var(--error, #000)',
    color: '#000',
  },
  _focusVisible: {
    outlineColor: 'var(--error, #000)',
  },
  _hover: {
    bg: 'inherit',
    border: '1px solid var(--error, #000)',
  },
  _disabled: {
    color: '#D6D7DE',
    bgColor: '#F0F0F3',
    cursor: 'not-allowed',
    borderColor: 'var(--border, #000)',
  },
  _themeDark: {
    _disabled: {
      color: '#373737',
      bgColor: '#47474A',
      cursor: 'not-allowed',
      borderColor: 'transparent',
    },
    _active: {
      bg: 'var(--error, #000)',
      border: '1px solid var(--error, #000)',
      color: '#FFF',
    },
    _hover: {
      bg: 'var(--inputBg, #000)',
      borderColor: 'var(--error, #000)',
    },
    _focusVisible: {
      outlineColor: 'var(--error, #000)',
    },
  },
  typography: 'inputBold',
})

const buttonTextEllipsis = css({
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
})

export function Button({
  variant = 'default',
  type = 'button',
  colors,
  isError = false,
  children,
  size = 'md',
  className,
  icon,
  ellipsis = false,
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
        outlineColor: 'var(--primaryFocus, #000)',
      }}
      _themeDark={{
        _disabled: {
          color: '#373737',
          bgColor: '#47474A',
          cursor: 'not-allowed',
          borderColor: 'transparent',
        },
        _focusVisible: {
          outlineColor: 'var(--primaryFocus, #FFF)',
        },
      }}
      aria-disabled={props.disabled}
      aria-label="button"
      boxSizing="border-box"
      className={clsx(
        variants[variant],
        isError && variant === 'default' && errorClassNames,
        className,
      )}
      color="var(--text, #000)"
      cursor="pointer"
      outlineOffset="2px"
      pos="relative"
      px="40px"
      py="12px"
      styleOrder={1}
      styleVars={{
        primary: colors?.primary,
        error: colors?.error,
        text: colors?.text,
        border: colors?.border,
        inputBg: colors?.inputBg,
        primaryFocus: colors?.primaryFocus,
        white: colors?.white,
      }}
      transition=".25s"
      type={type}
      typography={
        isPrimary
          ? {
              sm: 'buttonS',
              md: 'buttonM',
            }[size]
          : undefined
      }
      {...props}
    >
      <Box maxW="100%" mx="auto" pos="relative" w="fit-content">
        {icon && (
          <Center
            boxSize="24px"
            left="4px"
            pos="absolute"
            role="presentation"
            selectors={{
              '&>svg': {
                color: 'inherit',
              },
            }}
            top="50%"
            transform="translate(-100%, -50%)"
          >
            {icon}
          </Center>
        )}
        <Box
          className={clsx(ellipsis && buttonTextEllipsis)}
          lineHeight="1em"
          minH="1em"
          transform={!!icon && 'translateX(8px)'}
        >
          {children}
        </Box>
      </Box>
    </DevupButton>
  )
}
