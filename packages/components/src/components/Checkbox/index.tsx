'use client'

import { Box, css, Flex, Input, Text } from '@devup-ui/react'
import { ComponentProps, useId, useState } from 'react'

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
  defaultChecked = false,
  colors,
  onChange,
  ...props
}: CheckboxProps) {
  const generateId = useId()
  const [innerChecked, setInnerChecked] = useState(defaultChecked)
  const isChecked = checked ?? innerChecked

  const handleChange = (value: boolean) => {
    setInnerChecked(value)
    onChange?.(value)
  }

  return (
    <Flex alignItems="center" gap="8px">
      <label
        className={css({
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '16px',
          height: '16px',
          cursor: disabled ? 'not-allowed' : 'pointer',
        })}
        htmlFor={generateId}
      >
        <Input
          _active={
            !disabled && {
              bg: 'light-dark(color-mix(in srgb, var(--primary, #6159D4) 20%, #FFF 80%), color-mix(in srgb, var(--primary, #6670F9) 30%, #000 70%))',
            }
          }
          _checked={{
            bg: 'var(--primary, light-dark(#6159D4, #6670F9))',
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
              border: '1px solid var(--primary, light-dark(#6159D4, #6670F9))',
            }
          }
          accentColor="var(--primary, light-dark(#6159D4, #6670F9))"
          appearance="none"
          bg="var(--inputBg, light-dark(#FFF, #2E2E2E))"
          border="1px solid var(--border, light-dark(#E0E0E0, #333333))"
          borderRadius="2px"
          checked={isChecked}
          cursor={disabled ? 'not-allowed' : 'pointer'}
          disabled={disabled}
          display="block"
          height="100%"
          id={generateId}
          left={0}
          m="0"
          onChange={
            disabled ? undefined : (e) => handleChange(e.target.checked)
          }
          opacity={1}
          pointerEvents="none"
          pos="absolute"
          styleOrder={1}
          styleVars={{
            primary: colors?.primary,
            border: colors?.border,
            inputBg: colors?.inputBg,
          }}
          top={0}
          type="checkbox"
          width="100%"
          zIndex={0}
          {...props}
        />
        {isChecked && (
          <Box
            as={CheckIcon}
            opacity="1"
            pointerEvents="none"
            props={{
              color: disabled
                ? 'light-dark(#D6D7DE, #47474A)'
                : 'var(--checkIcon, #FFF)',
            }}
            styleVars={{
              checkIcon: colors?.checkIcon,
            }}
            zIndex="1"
          />
        )}
      </label>

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
                ? 'light-dark(#D6D7DE, #6F6E6E)'
                : 'var(--text, light-dark(#2F2F2F, #EDEDED))'
            }
            fontSize="14px"
            styleVars={{
              text: colors?.text,
            }}
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
