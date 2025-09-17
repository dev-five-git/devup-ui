import { Box, css, Flex, getTheme, Input, Text } from '@devup-ui/react'
import { ComponentProps } from 'react'

import { CheckIcon } from './CheckIcon'

interface CheckboxProps
  extends Omit<ComponentProps<'input'>, 'type' | 'onChange'> {
  children: React.ReactNode
  onChange?: (checked: boolean) => void
  label: string
}

export function Checkbox({
  children,
  disabled,
  checked,
  onChange,
  label,
  ...props
}: CheckboxProps) {
  const theme = getTheme()

  return (
    <Flex alignItems="center" gap="8px" h="fit-content">
      <Box h="18px" pos="relative" w="fit-content">
        <Input
          _active={
            disabled
              ? undefined
              : {
                  bg:
                    theme === 'dark'
                      ? 'color-mix(in srgb, var(--primary) 30%, black 70%)'
                      : 'color-mix(in srgb, var(--primary) 20%, #FFF 80%)',
                }
          }
          _checked={{
            bg: '$primary',
            border: 'none',
            _hover: disabled
              ? undefined
              : {
                  bg:
                    theme === 'dark'
                      ? 'color-mix(in srgb, var(--primary) 100%, #FFF 15%)'
                      : 'color-mix(in srgb, var(--primary) 100%, #000 15%)',
                },
            _disabled: {
              bg: theme === 'dark' ? '#47474A' : '#F0F0F3',
            },
          }}
          _disabled={{
            bg: theme === 'dark' ? '#47474A' : '#F0F0F3',
          }}
          _hover={
            disabled
              ? undefined
              : {
                  bg:
                    theme === 'dark'
                      ? 'color-mix(in srgb, var(--primary) 20%, black 80%);'
                      : 'color-mix(in srgb, var(--primary) 10%, #FFF 90%)',
                  border: '1px solid var(--primary)',
                }
          }
          accentColor="$primary"
          appearance="none"
          bg={theme === 'dark' ? '$inputBg' : '$contentBackground'}
          border="1px solid var(--border)"
          borderRadius="2px"
          boxSize="16px"
          checked={checked}
          cursor={disabled ? 'not-allowed' : 'pointer'}
          disabled={disabled}
          id={label}
          m="0"
          onChange={(e) => !disabled && onChange?.(e.target.checked)}
          styleOrder={1}
          type="checkbox"
          {...props}
        />
        {checked && (
          <CheckIcon
            className={css({
              position: 'absolute',
              top: '8px',
              left: '50%',
              transform: 'translate(-50%, -50%)',
              pointerEvents: 'none',
            })}
            color={
              disabled ? (theme === 'dark' ? '#373737' : '#D6D7DE') : '#FFF'
            }
          />
        )}
      </Box>

      <label
        className={css({
          cursor: disabled ? 'not-allowed' : 'pointer',
        })}
        htmlFor={label}
      >
        {typeof children === 'string' ? (
          <Text
            as="span"
            color={disabled ? '#D6D7DE' : '$text'}
            fontSize="14px"
            style={{ userSelect: 'none', verticalAlign: 'middle' }}
          >
            {children}
          </Text>
        ) : (
          children
        )}
      </label>
    </Flex>
  )
}
