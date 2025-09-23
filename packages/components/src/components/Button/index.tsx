import {
  Box,
  Button as DevupButton,
  Center,
  css,
  type DevupThemeTypography,
} from '@devup-ui/react'
import { clsx } from 'clsx'

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'default'
  colors?: {
    primary?: string
    error?: string
    text?: string
    border?: string
    inputBackground?: string
    primaryFocus?: string
  }
  typography?: keyof DevupThemeTypography
  danger?: boolean
  size?: 'sm' | 'md' | 'lg'
  icon?: React.ReactNode
  ellipsis?: boolean
}

export function Button({
  variant = 'default',
  type = 'button',
  colors,
  danger = false,
  children,
  size = 'md',
  className,
  icon,
  ellipsis = false,
  typography,
  disabled,
  ...props
}: ButtonProps): React.ReactElement {
  return (
    <DevupButton
      _active={
        {
          primary: {
            bg: `color-mix(in srgb, var(--primary, #674DC7) 100%, #000 30%)`,
          },
          default: {
            bg: {
              true: 'var(--error, #D52B2E)',
              false: `color-mix(in srgb, var(--primary, #8163E1) 20%, #FFF 80%)`,
            }[danger.toString()],
            border: {
              true: '1px solid var(--error, #D52B2E)',
              false: '1px solid var(--primary, #8163E1)',
            }[danger.toString()],
            color: 'var(--text, #272727)',
          },
        }[variant]
      }
      _disabled={
        {
          primary: {
            color: '#D6D7DE',
            bgColor: '#F0F0F3',
            cursor: 'not-allowed',
          },
          default: {
            color: '#D6D7DE',
            bgColor: '#F0F0F3',
            cursor: 'not-allowed',
            borderColor: 'var(--border, #E4E4E4)',
          },
        }[variant]
      }
      _focusVisible={{
        outline: '2px solid',
        outlineColor: {
          primary: 'var(--primaryFocus, #9385D3)',
          default: {
            true: 'var(--error, #FF5B5E)',
            false: 'var(--primaryFocus, #9385D3)',
          }[danger.toString()],
        }[variant],
      }}
      _hover={
        {
          primary: {
            bg: `color-mix(in srgb, var(--primary, #674DC7) 100%, #000 15%)`,
          },
          default: {
            borderColor: {
              true: 'var(--error, #D52B2E)',
              false: 'var(--primary, #8163E1)',
            }[danger.toString()],
            bg:
              !danger &&
              `color-mix(in srgb, var(--primary, #8163E1) 10%, #FFF 90%)`,
          },
        }[variant]
      }
      _themeDark={{
        _active: {
          primary: {
            bg: `color-mix(in srgb, var(--primary, #8163E1) 100%, #FFF 30%);`,
          },
          default: {
            bg: {
              true: 'var(--error, #FF5B5E)',
              false: 'var(--primary, #8163E1)',
            }[danger.toString()],
            color: 'var(--text, #F6F6F6)',
          },
        }[variant],
        _disabled: {
          primary: {
            color: '#373737',
            bgColor: '#47474A',
            borderColor: 'transparent',
          },
          default: {
            color: '#373737',
            bgColor: '#47474A',
            borderColor: 'transparent',
          },
        }[variant],
        _hover: {
          primary: {
            bg: `color-mix(in srgb, var(--primary, #8163E1) 100%, #FFF 15%);`,
            outlineColor: `var(--primary, #674DC7)`,
          },
          default: {
            borderColor: {
              true: 'var(--error, #FF5B5E)',
              false: 'var(--primary, #8163E1)',
            }[danger.toString()],
            bg:
              !danger &&
              `color-mix(in srgb, var(--primary, #674DC7) 10%, var(--inputBackground, #2E2E2E) 90%)`,
          },
        }[variant],
        _focusVisible: {
          outlineColor: {
            primary: 'var(--primaryFocus, #927CE4)',
            default: {
              true: 'var(--error, #D52B2E)',
              false: 'var(--primaryFocus, #927CE4)',
            }[danger.toString()],
          }[variant],
        },
        bg: {
          primary: 'var(--primary, #8163E1)',
          default: 'var(--inputBackground, #2E2E2E)',
        }[variant],
      }}
      aria-disabled={disabled}
      aria-label="button"
      bg={
        {
          primary: 'var(--primary, #8163E1)',
          default: 'var(--inputBackground, #FFF)',
        }[variant]
      }
      border={
        {
          primary: 'none',
          default: '1px solid var(--border, #E4E4E4)',
        }[variant]
      }
      borderRadius={
        {
          primary: '8px',
          default: '10px',
        }[variant]
      }
      boxSizing="border-box"
      className={className}
      color={
        {
          primary: '#FFF',
          default: {
            true: 'var(--error, #D52B2E)',
            false: 'var(--text, #272727)',
          }[danger.toString()],
        }[variant]
      }
      cursor="pointer"
      disabled={disabled}
      fontSize={
        {
          default: ['14px', null, null, null, '15px'],
          primary: ['15px', null, null, null, '16px'],
        }[variant]
      }
      fontWeight={700}
      letterSpacing={
        {
          default: ['-0.02em', null, null, null, '-0.03em'],
          primary: ['0px', null, null, null, '-0.01em'],
        }[variant]
      }
      outlineOffset="2px"
      pos="relative"
      px={
        {
          false: { sm: '12px', md: '16px', lg: '20px' }[size],
          true: { sm: '24px', md: '28px', lg: '32px' }[size],
        }[(!!icon).toString()]
      }
      py={{ sm: '8px', md: '10px', lg: '12px' }[size]}
      styleOrder={1}
      styleVars={{
        primary: colors?.primary,
        error: colors?.error,
        text: colors?.text,
        border: colors?.border,
        inputBackground: colors?.inputBackground,
        primaryFocus: colors?.primaryFocus,
      }}
      transition=".25s"
      type={type}
      typography={typography}
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
          className={clsx(
            ellipsis &&
              css({
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }),
          )}
          lineHeight="1.2"
          minH="1.2em"
          transform={!!icon && 'translateX(8px)'}
        >
          {children}
        </Box>
      </Box>
    </DevupButton>
  )
}
