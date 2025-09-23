import { Box, css, Flex, Input, Text } from '@devup-ui/react'
import { ComponentProps, useId } from 'react'

import { CheckIcon } from './CheckIcon'

interface CheckboxProps
  extends Omit<ComponentProps<'input'>, 'type' | 'onChange'> {
  children: React.ReactNode
  onChange?: (checked: boolean) => void
  colors?: {
    primary?: string
    border?: string
    text?: string
    inputBg?: string
    checkIcon?: string
  }
}

export function Checkbox({
  children,
  disabled,
  checked,
  colors,
  onChange,
  ...props
}: CheckboxProps) {
  const generateId = useId()
  return (
    <Flex alignItems="center" gap="8px">
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
            bg: 'light-dark( #F0F0F3, #47474A)',
          }}
          _hover={
            !disabled && {
              bg: 'light-dark(color-mix(in srgb, var(--primary, #6159D4) 10%, #FFF 90%), color-mix(in srgb, var(--primary, #6670F9) 20%, #000 80%))',
              border:
                '1px solid light-dark(var(--primary, #6159D4), var(--primary, #6670F9))',
            }
          }
          accentColor="light-dark(var(--primary, #6159D4), var(--primary, #6670F9))"
          appearance="none"
          bg="light-dark(#FFF, var(--inputBg, #2E2E2E))"
          border="1px solid light-dark(var(--border, #E0E0E0), var(--border, #333333))"
          borderRadius="2px"
          boxSize="16px"
          checked={checked}
          cursor={disabled ? 'not-allowed' : 'pointer'}
          disabled={disabled}
          id={generateId}
          m="0"
          onChange={
            disabled || !onChange
              ? undefined
              : (e) => onChange(e.target.checked)
          }
          styleOrder={1}
          styleVars={{
            primary: colors?.primary,
            border: colors?.border,
            text: colors?.text,
            inputBg: colors?.inputBg,
            checkIcon: colors?.checkIcon,
          }}
          type="checkbox"
          {...props}
        />
        {checked && (
          <Box
            as={CheckIcon}
            props={{
              color: disabled
                ? 'light-dark(#D6D7DE, #373737)'
                : 'var(--checkIcon, #FFF)',
              className: css({
                left: '50%',
                pointerEvents: 'none',
                pos: 'absolute',
                top: '60%',
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
        htmlFor={generateId}
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
