'use client'

import {
  Box,
  Button,
  Center,
  DevupThemeTypography,
  Input as DevupInput,
  Text,
} from '@devup-ui/react'
import { ComponentProps, forwardRef, useId, useRef, useState } from 'react'

import { joinIds, mergeRefs } from '../../utils/dom'

interface InputProps extends Omit<ComponentProps<'input'>, 'type'> {
  type?: Exclude<ComponentProps<'input'>['type'], 'file'>
  typography?: keyof DevupThemeTypography
  error?: boolean
  errorMessage?: string
  allowClear?: boolean
  classNames?: {
    container?: string
    input?: string
    icon?: string
    errorMessage?: string
  }
  onClear?: () => void
  colors?: {
    primary?: string
    error?: string
    text?: string
    base?: string
    iconBold?: string
    border?: string
    inputBackground?: string
    inputDisabledBackground?: string
    inputDisabledText?: string
    inputPlaceholder?: string
    primaryBackground?: string
    primaryFocus?: string
    negative20?: string
  }
  icon?: React.ReactNode
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  {
    defaultValue = '',
    value: valueProp,
    onChange: onChangeProp,
    typography,
    error = false,
    errorMessage,
    allowClear = true,
    icon,
    colors,
    disabled,
    className,
    classNames,
    readOnly,
    onClear,
    id,
    'aria-describedby': describedBy,
    ...props
  },
  ref,
) {
  const [value, setValue] = useState(defaultValue)
  const inputRef = useRef<HTMLInputElement>(null)
  const generatedId = useId()
  const errorMessageId = `${id ?? generatedId}-error`
  const showsError = error && !!errorMessage

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value)
    onChangeProp?.(e)
  }

  const handleClear = () => {
    const input = inputRef.current
    if (input) {
      // A real input event, so the change reaches React and the owner as it does when the user types
      Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        'value',
      )?.set?.call(input, '')
      input.dispatchEvent(new Event('input', { bubbles: true }))
      input.focus()
    }
    onClear?.()
  }

  const innerValue = valueProp ?? value

  const clearButtonVisible =
    !!innerValue && !disabled && allowClear && !readOnly

  return (
    <Box
      className={classNames?.container}
      display="inline-block"
      pos="relative"
      selectors={{ '&, & *': { boxSizing: 'border-box' } }}
      styleVars={{
        primary: colors?.primary,
        error: colors?.error,
        text: colors?.text,
        base: colors?.base,
        iconBold: colors?.iconBold,
        border: colors?.border,
        inputBackground: colors?.inputBackground,
        inputDisabledBackground: colors?.inputDisabledBackground,
        inputDisabledText: colors?.inputDisabledText,
        inputPlaceholder: colors?.inputPlaceholder,
        primaryBackground: colors?.primaryBackground,
        primaryFocus: colors?.primaryFocus,
        negative20: colors?.negative20,
      }}
    >
      {icon && (
        <Center
          aria-hidden

          boxSize="24px"
          className={classNames?.icon}
          color={
            disabled
              ? 'var(--inputDisabledText, light-dark(#D6D7DE, #373737))'
              : 'var(--iconBold, light-dark(#8D8C9A, #666577))'
          }
          left="12px"
          pos="absolute"
          styleOrder={1}
          top="50%"
          transform="translateY(-50%)"
        >
          {icon}
        </Center>
      )}
      <DevupInput
        ref={mergeRefs(ref, inputRef)}
        _disabled={{
          _placeholder: {
            color: 'var(--inputDisabledText, light-dark(#D6D7DE, #373737))',
          },
          bg: 'var(--inputDisabledBackground, light-dark(#F0F0F3, #414244))',
          border: '1px solid var(--border, light-dark(#E4E4E4, #434343))',
          color: 'var(--inputDisabledText, light-dark(#D6D7DE, #373737))',
        }}
        _focus={{
          bg: 'var(--primaryBackground, light-dark(#F4F3FA, #F4F3FA0D))',
          border: '1px solid var(--primary, light-dark(#674DC7, #8163E1))',
          outline: 'none',
        }}
        _hover={{
          border: '1px solid var(--primary, light-dark(#674DC7, #8163E1))',
        }}
        _placeholder={{
          color: 'var(--inputPlaceholder, light-dark(#A9A8AB, #CBCBCB))',
        }}
        aria-describedby={joinIds(describedBy, showsError && errorMessageId)}
        aria-invalid={error || undefined}
        bg="var(--inputBackground, light-dark(#FFFFFF, #2E2E2E))"
        borderColor={
          error
            ? 'var(--error, light-dark(#D52B2E, #FF5B5E))'
            : 'var(--border, light-dark(#E4E4E4, #434343))'
        }
        borderRadius="8px"
        borderStyle="solid"
        borderWidth="1px"
        className={`${className || ''} ${classNames?.input || ''}`.trim()}
        disabled={disabled}
        id={id}
        onChange={handleChange}
        pl={icon ? '36px' : '12px'}
        pr={allowClear ? '36px' : '12px'}
        py="12px"
        styleOrder={1}
        transition="all 0.1s ease-in-out"
        typography={typography}
        value={innerValue}
        {...props}
      />
      {clearButtonVisible && <ClearButton onClick={handleClear} />}
      {showsError && (
        <Text
          bottom="-8px"
          className={classNames?.errorMessage}
          color="var(--error, light-dark(#D52B2E, #FF5B5E))"
          id={errorMessageId}
          left="0"
          pos="absolute"
          role="alert"
          styleOrder={1}
          transform="translateY(100%)"
          typography="inputPlaceholder"
        >
          {errorMessage}
        </Text>
      )}
    </Box>
  )
})

export function ClearButton(props: ComponentProps<'button'>) {
  return (
    <Button
      alignItems="center"
      aria-label="clear-button"
      bg="var(--negative20, light-dark(#0003, #FFF6))"
      border="none"
      borderRadius="50%"
      boxSize="20px"
      color="var(--base, light-dark(#FFF, #000))"
      cursor="pointer"
      display="flex"
      justifyContent="center"
      p="2px"
      pos="absolute"
      right="12px"
      styleOrder={1}
      top="50%"
      transform="translateY(-50%)"
      type="button"
      {...props}
    >
      <svg
        fill="none"
        height="24"
        viewBox="0 0 24 24"
        width="24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M18 6L6 18"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
        />
        <path
          d="M6 6L18 18"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
        />
      </svg>
    </Button>
  )
}
