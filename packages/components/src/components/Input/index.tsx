'use client'

import {
  Box,
  Button,
  Center,
  DevupThemeTypography,
  Input as DevupInput,
  Text,
} from '@devup-ui/react'
import { ComponentProps, useState } from 'react'

interface InputProps extends Omit<ComponentProps<'input'>, 'type'> {
  type?: Exclude<ComponentProps<'input'>['type'], 'file'>
  typography?: keyof DevupThemeTypography
  error?: boolean
  errorMessage?: string
  allowClear?: boolean
  classNames?: {
    input?: string
    icon?: string
    errorMessage?: string
  }
  colors?: {
    primary?: string
    error?: string
    text?: string
    base?: string
    iconBold?: string
    border?: string
    inputBackground?: string
    primaryFocus?: string
    negative20?: string
  }
  icon?: React.ReactNode
}

export function Input({
  defaultValue,
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
  ref,
  ...props
}: InputProps) {
  const [value, setValue] = useState(defaultValue || '')
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value)
    onChangeProp?.(e)
  }
  const handleClear = () => {
    setValue('')
  }
  const clearButtonVisible = value && !disabled && allowClear

  return (
    <Box
      display="inline-block"
      pos="relative"
      selectors={{ '&, & *': { boxSizing: 'border-box' } }}
    >
      {icon && (
        <Center
          aria-label="icon"
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
        ref={ref}
        _disabled={{
          _placeholder: {
            color: 'var(--inputDisabledText, light-dark(#D6D7DE, #373737))',
          },
          bg: 'var(--inputDisabledBg, light-dark(#F0F0F3, #414244))',
          border: '1px solid var(--border, light-dark(#E4E4E4, #434343))',
          color: 'var(--inputDisabledText, light-dark(#D6D7DE, #373737))',
        }}
        _focus={{
          bg: 'var(--primaryBg, light-dark(#F4F3FA, #F4F3FA0D))',
          border: '1px solid var(--primary, light-dark(#674DC7, #8163E1))',
          outline: 'none',
        }}
        _hover={{
          border: '1px solid var(--primary, light-dark(red, blue))',
        }}
        _placeholder={{
          color: 'var(--inputPlaceholder, light-dark(#A9A8AB, #CBCBCB))',
        }}
        aria-label="input"
        bg="var(--inputBg, light-dark(#FFFFFF, #2E2E2E))"
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
        onChange={handleChange}
        pl={icon ? '36px' : '12px'}
        pr={allowClear ? '36px' : '12px'}
        py="12px"
        styleOrder={1}
        styleVars={{
          primary: colors?.primary,
          error: colors?.error,
          text: colors?.text,
          base: colors?.base,
          iconBold: colors?.iconBold,
          border: colors?.border,
          inputBackground: colors?.inputBackground,
          primaryFocus: colors?.primaryFocus,
          negative20: colors?.negative20,
        }}
        transition="all 0.1s ease-in-out"
        typography={typography}
        value={valueProp ?? value}
        {...props}
      />
      {clearButtonVisible && <ClearButton onClick={handleClear} />}
      {errorMessage && (
        <Text
          aria-label="error-message"
          bottom="-8px"
          className={classNames?.errorMessage}
          color="var(--error, light-dark(#D52B2E, #FF5B5E))"
          left="0"
          pos="absolute"
          styleOrder={1}
          transform="translateY(100%)"
          typography="inputPlaceholder"
        >
          {errorMessage}
        </Text>
      )}
    </Box>
  )
}

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
