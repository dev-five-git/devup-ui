'use client'

import { Box, type DevupThemeTypography, Text } from '@devup-ui/react'
import { type ComponentProps } from 'react'

interface TextareaProps extends ComponentProps<'textarea'> {
  typography?: keyof DevupThemeTypography
  error?: boolean
  errorMessage?: string
  classNames?: {
    container?: string
    textarea?: string
    errorMessage?: string
  }
  colors?: {
    primary?: string
    error?: string
    text?: string
    border?: string
    background?: string
    placeholder?: string
    focusRing?: string
  }
}

export function Textarea({
  typography,
  error = false,
  errorMessage,
  colors,
  disabled,
  className,
  classNames,
  rows = 3,
  ...props
}: TextareaProps) {
  return (
    <Box
      className={classNames?.container}
      display="inline-block"
      pos="relative"
      selectors={{ '&, & *': { boxSizing: 'border-box' } }}
      w="100%"
    >
      <Box
        _disabled={{
          _placeholder: {
            color: 'var(--disabledText, light-dark(#D6D7DE, #373737))',
          },
          bg: 'var(--disabledBg, light-dark(#F0F0F3, #414244))',
          borderColor: 'var(--border, light-dark(#E4E4E4, #434343))',
          color: 'var(--disabledText, light-dark(#D6D7DE, #373737))',
          cursor: 'not-allowed',
          opacity: 0.5,
        }}
        _focus={{
          borderColor: 'var(--primary, light-dark(#674DC7, #8163E1))',
          boxShadow:
            '0 0 0 3px var(--focusRing, light-dark(rgba(103, 77, 199, 0.15), rgba(129, 99, 225, 0.25)))',
          outline: 'none',
        }}
        _hover={
          !disabled && {
            borderColor: 'var(--primary, light-dark(#674DC7, #8163E1))',
          }
        }
        _placeholder={{
          color: 'var(--placeholder, light-dark(#A9A8AB, #CBCBCB))',
        }}
        aria-invalid={error || undefined}
        aria-label="textarea"
        as="textarea"
        bg="var(--background, light-dark(#FFFFFF, #2E2E2E))"
        borderColor="var(--border, light-dark(#E4E4E4, #434343))"
        borderRadius="8px"
        borderStyle="solid"
        borderWidth="1px"
        className={`${className || ''} ${classNames?.textarea || ''}`.trim()}
        color="var(--text, light-dark(#272727, #F6F6F6))"
        disabled={disabled}
        fontSize={['16px', null, null, null, '14px']}
        lineHeight="1.5"
        minH="80px"
        p="12px"
        rows={rows}
        selectors={{
          '&[aria-invalid="true"]': {
            borderColor: 'var(--error, light-dark(#D52B2E, #FF5B5E))',
          },
          '&[aria-invalid="true"]:focus': {
            borderColor: 'var(--error, light-dark(#D52B2E, #FF5B5E))',
            boxShadow:
              '0 0 0 3px var(--focusRing, light-dark(rgba(213, 43, 46, 0.2), rgba(255, 91, 94, 0.4)))',
          },
        }}
        styleOrder={1}
        styleVars={{
          primary: colors?.primary,
          error: colors?.error,
          text: colors?.text,
          border: colors?.border,
          background: colors?.background,
          placeholder: colors?.placeholder,
          focusRing: colors?.focusRing,
        }}
        transition="border-color 0.15s ease-in-out, box-shadow 0.15s ease-in-out"
        typography={typography}
        w="100%"
        {...props}
      />
      {error && errorMessage && (
        <Text
          aria-label="error-message"
          bottom="-8px"
          className={classNames?.errorMessage}
          color="var(--error, light-dark(#D52B2E, #FF5B5E))"
          fontSize="12px"
          left="0"
          pos="absolute"
          styleOrder={1}
          transform="translateY(100%)"
        >
          {errorMessage}
        </Text>
      )}
    </Box>
  )
}
