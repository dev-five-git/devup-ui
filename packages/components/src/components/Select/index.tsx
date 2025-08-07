'use client'

import { Box, css, Flex, VStack } from '@devup-ui/react'
import clsx from 'clsx'
import { ComponentProps, createContext, useContext, useState } from 'react'

import { Button } from '../Button'

interface SelectProps {
  open?: boolean
  onOpenChange?: (open: boolean) => void
  children: React.ReactNode
}

const SelectContext = createContext<{
  open: boolean
  setOpen: (open: boolean) => void
} | null>(null)

export const useSelect = () => {
  const context = useContext(SelectContext)
  if (!context) {
    throw new Error('useSelect must be used within a Select')
  }
  return context
}

export function Select({
  children,
  open: openProp,
  onOpenChange,
}: SelectProps) {
  const [open, setOpen] = useState(openProp ?? false)
  const handleOpenChange = (open: boolean) => {
    setOpen(open)
    onOpenChange?.(open)
  }
  return (
    <SelectContext.Provider value={{ open, setOpen: handleOpenChange }}>
      <Box display="inline-block" pos="relative">
        {children}
      </Box>
    </SelectContext.Provider>
  )
}

export function SelectTrigger({
  className,
  children,
  ...props
}: ComponentProps<typeof Button>) {
  const { open, setOpen } = useSelect()
  const handleClick = () => {
    setOpen(!open)
  }

  return (
    <Button
      className={clsx(
        css({
          pos: 'relative',
          borderRadius: '8px',
        }),
        className,
      )}
      onClick={handleClick}
      {...props}
    >
      {children}
    </Button>
  )
}

export function SelectContainer({ children, ...props }: ComponentProps<'div'>) {
  const { open } = useSelect()
  if (!open) return null
  return (
    <VStack
      bg="$inputBg"
      border="1px solid $border"
      borderRadius="8px"
      bottom="-4px"
      boxShadow="0 2px 2px 0 $base10"
      gap="6px"
      p="10px"
      pos="absolute"
      styleOrder={1}
      transform="translateY(100%)"
      userSelect="none"
      w="232px"
      {...props}
    >
      {children}
    </VStack>
  )
}

interface SelectOptionProps extends ComponentProps<'div'> {
  disabled?: boolean
}

export function SelectOption({
  disabled,
  onClick,
  children,
  ...props
}: SelectOptionProps) {
  const { setOpen } = useSelect()
  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (onClick) {
      onClick(e)
      return
    }
    setOpen(false)
  }

  return (
    <Flex
      _hover={
        !disabled && {
          bg: '$primaryBg',
        }
      }
      alignItems="center"
      borderRadius="8px"
      color={disabled ? '$selectDisabled' : '$title'}
      cursor={disabled ? 'default' : 'pointer'}
      h="40px"
      onClick={disabled ? undefined : handleClick}
      px="10px"
      styleOrder={1}
      transition="background-color 0.1s ease-in-out"
      typography="inputText"
      {...props}
    >
      {children}
    </Flex>
  )
}

export function SelectDivider({ ...props }: ComponentProps<'div'>) {
  return <Box bg="$border" h="1px" styleOrder={1} w="100%" {...props} />
}
