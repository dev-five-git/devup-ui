'use client'

import { Box, css, Flex, VStack } from '@devup-ui/react'
import clsx from 'clsx'
import {
  ComponentProps,
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'

import { Button } from '../Button'
import { IconCheck } from './IconCheck'

type SelectType = 'default' | 'radio' | 'checkbox'
type SelectValue<T extends SelectType> = T extends 'radio' ? string : string[]

interface SelectProps {
  open?: boolean
  onOpenChange?: (open: boolean) => void
  children: React.ReactNode
  type?: SelectType
}

const SelectContext = createContext<{
  open: boolean
  setOpen: (open: boolean) => void
  value: SelectValue<SelectType>
  setValue: (value: string) => void
  type: SelectType
} | null>(null)

export const useSelect = () => {
  const context = useContext(SelectContext)
  if (!context) {
    throw new Error('useSelect must be used within a Select')
  }
  return context
}

export function Select({
  type = 'default',
  children,
  open: openProp,
  onOpenChange,
}: SelectProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(openProp ?? false)
  const [value, setValue] = useState<SelectValue<typeof type>>(
    type === 'checkbox' ? [] : '',
  )

  useEffect(() => {
    if (!ref.current) return
    const handleOutsideClick = (e: MouseEvent) => {
      if (ref.current?.contains(e.target as Node)) return
      setOpen(false)
    }
    document.addEventListener('click', handleOutsideClick)
    return () => document.removeEventListener('click', handleOutsideClick)
  }, [open, setOpen])

  const handleOpenChange = (open: boolean) => {
    setOpen(open)
    onOpenChange?.(open)
  }

  const handleValueChange = (nextValue: string) => {
    if (type === 'default') return
    if (type === 'radio') {
      setValue(nextValue)
      return
    }
    if (Array.isArray(value) && value.includes(nextValue)) {
      setValue(value.filter((v) => v !== nextValue))
    } else {
      setValue([...value, nextValue])
    }
  }

  return (
    <SelectContext.Provider
      value={{
        open,
        setOpen: handleOpenChange,
        value,
        setValue: handleValueChange,
        type,
      }}
    >
      <Box ref={ref} display="inline-block" pos="relative">
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
  const { setOpen, setValue, value, type } = useSelect()

  const handleClose = () => {
    if (type === 'checkbox') return
    setOpen(false)
  }

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (onClick) {
      onClick(e)
      return
    }
    setValue(children as string)
    handleClose()
  }

  const isChecked = Array.isArray(value)
    ? value.includes(children as string)
    : value === children

  return (
    <Flex
      _hover={
        !disabled && {
          bg: '$primaryBg',
        }
      }
      alignItems="center"
      borderRadius="8px"
      color={disabled ? '$selectDisabled' : isChecked ? '$primary' : '$title'}
      cursor={disabled ? 'default' : 'pointer'}
      gap={
        {
          checkbox: '10px',
          radio: '6px',
          default: '0',
        }[type]
      }
      h="40px"
      onClick={disabled ? undefined : handleClick}
      px="10px"
      styleOrder={1}
      transition="background-color 0.1s ease-in-out"
      typography={isChecked ? 'inputBold' : 'inputText'}
      {...props}
    >
      {
        {
          checkbox: (
            <Box
              bg={isChecked ? '$primary' : '$border'}
              borderRadius="4px"
              boxSize="18px"
              pos="relative"
              transition="background-color 0.1s ease-in-out"
            >
              {isChecked && (
                <IconCheck
                  className={css({
                    position: 'absolute',
                    top: '55%',
                    left: '50%',
                    transform: 'translate(-50%, -50%)',
                  })}
                />
              )}
            </Box>
          ),
          radio: (
            <>
              {isChecked && (
                <Box
                  borderRadius="4px"
                  boxSize="18px"
                  pos="relative"
                  transition="background-color 0.1s ease-in-out"
                >
                  <IconCheck
                    className={css({
                      position: 'absolute',
                      top: '55%',
                      left: '50%',
                      transform: 'translate(-50%, -50%)',
                      color: '$primary',
                    })}
                  />
                </Box>
              )}
            </>
          ),
          default: null,
        }[type]
      }
      {children}
    </Flex>
  )
}

export function SelectDivider({ ...props }: ComponentProps<'div'>) {
  return <Box bg="$border" h="1px" styleOrder={1} w="100%" {...props} />
}
