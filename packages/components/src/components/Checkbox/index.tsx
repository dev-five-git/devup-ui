import { Box, css, Flex, Input, Text } from '@devup-ui/react'
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
  return (
    <Flex alignItems="center" gap="8px" h="fit-content">
      <Box h="18px" pos="relative" w="fit-content">
        <Input
          _active={
            !disabled && {
              bg: 'light-dark(color-mix(in srgb, var(--primary, #6159D4) 20%, #FFF 80%), color-mix(in srgb, var(--primary, #6670F9) 30%, #000 70%))',
            }
          }
          _checked={{
            bg: 'light-dark(var(--primary, #6159D4), var(--primary, #6670F9))',
            border: 'none',
            _hover: !disabled && {
              bg: 'light-dark(color-mix(in srgb, var(--primary, #6159D4) 100%, #000 15%), color-mix(in srgb, var(--primary, #6670F9) 100%, #FFF 15%))',
            },
            _disabled: {
              bg: 'light-dark(#F0F0F3, #47474A)',
            },
          }}
          _disabled={{
            bg: 'light-dark(#47474A, #F0F0F3)',
          }}
          _hover={
            !disabled && {
              bg: 'light-dark(color-mix(in srgb, var(--primary, #6159D4) 10%, #FFF 90%), color-mix(in srgb, var(--primary, #6670F9) 20%, #000 80%))',
              border:
                'light-dark(1px solid var(--primary, #6159D4), 1px solid var(--primary, #6670F9))',
            }
          }
          accentColor="light-dark(var(--primary, #6159D4), var(--primary, #6670F9))"
          appearance="none"
          bg="light-dark(#FFF, var(--inputBg, #2E2E2E))"
          border="light-dark(1px solid var(--border, #E0E0E0), 1px solid var(--border, #333333))"
          borderRadius="2px"
          boxSize="16px"
          checked={checked}
          cursor={disabled ? 'not-allowed' : 'pointer'}
          disabled={disabled}
          id={label}
          m="0"
          onChange={
            disabled || !onChange
              ? undefined
              : (e) => onChange(e.target.checked)
          }
          styleOrder={1}
          type="checkbox"
          {...props}
        />
        {checked && (
          <Box
            as={CheckIcon}
            props={{
              color: disabled ? 'light-dark(#D6D7DE, #373737)' : '#FFF',
              className: css({
                left: '50%',
                pointerEvents: 'none',
                pos: 'absolute',
                top: '8px',
                transform: 'translate(-50%, -50%)',
              }),
            }}
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
            color={
              disabled
                ? 'light-dark(#D6D7DE, #373737)'
                : 'light-dark(var(--text, #2F2F2F), var(--text, #EDEDED))'
            }
            fontSize="14px"
            userSelect="none"
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
