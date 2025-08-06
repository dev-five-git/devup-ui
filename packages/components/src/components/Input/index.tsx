'use client'

import {
  Box,
  Button,
  Center,
  css,
  DevupThemeTypography,
  Input as DevupInput,
} from '@devup-ui/react'
import { ComponentProps, useState } from 'react'

interface InputProps extends ComponentProps<'input'> {
  typography?: keyof DevupThemeTypography
  error?: boolean
  errorMessage?: string
  allowClear?: boolean
  icon?: React.ReactNode
}

export function Input({
  defaultValue,
  value: valueProp,
  onChange: onChangeProp,
  typography,
  error = false,
  errorMessage,
  allowClear = false,
  icon,
  ...props
}: InputProps) {
  const [value, setValue] = useState(defaultValue ?? '')
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value)
  }
  const handleClear = () => {
    setValue('')
  }
  const clearButtonVisible = value && !props.disabled

  return (
    <Box
      pos="relative"
      selectors={{ '&,&>*': { boxSizing: 'border-box' } }}
      w="fit-content"
    >
      {icon && (
        <Center
          boxSize="24px"
          color="$base"
          left="12px"
          pos="absolute"
          top="50%"
          transform="translateY(-50%)"
        >
          {icon}
        </Center>
      )}
      <DevupInput
        _disabled={{
          selectors: {
            '&::placeholder': {
              color: '$inputDisabledText',
            },
          },
          bg: '$inputDisabledBg',
          border: '1px solid $border',
          color: '$inputDisabledText',
        }}
        _focus={{
          bg: '$primaryBg',
          border: '1px solid $primary',
          outline: 'none',
        }}
        _hover={{
          border: '1px solid $primary',
        }}
        bg="$inputBg"
        border={error ? '1px solid $error' : '1px solid $border'}
        borderRadius="8px"
        onChange={onChangeProp ?? handleChange}
        pl={icon ? '36px' : '12px'}
        pr={['36px', null, allowClear ? '36px' : '12px']}
        py="12px"
        selectors={{
          '&::placeholder': {
            color: '$inputPlaceholder',
          },
        }}
        styleOrder={1}
        transition="all 0.1s ease-in-out"
        typography={typography}
        value={valueProp ?? value}
        w="200px"
        {...props}
      />
      {clearButtonVisible && (
        <ClearButton
          className={css({
            display: ['flex', null, allowClear ? 'flex' : 'none'],
          })}
          onClick={handleClear}
        />
      )}
    </Box>
  )
}

export function ClearButton(props: ComponentProps<'button'>) {
  return (
    <Button
      alignItems="center"
      bg="$negative20"
      border="none"
      borderRadius="50%"
      boxSize="20px"
      color="$base"
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
