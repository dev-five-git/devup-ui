'use client'

import { Box, css, Flex, VStack } from '@devup-ui/react'
import clsx from 'clsx'
import {
  Children,
  ComponentProps,
  createContext,
  JSX,
  JSXElementConstructor,
  ReactElement,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'

import { Button } from '../Button'
import { IconCheck } from './IconCheck'

type SelectType = 'default' | 'radio' | 'checkbox'
type SelectValue<T extends SelectType> = T extends 'radio' ? string : string[]

interface SelectProps extends ComponentProps<'div'> {
  defaultValue?: SelectValue<SelectType>
  value?: SelectValue<SelectType>
  onValueChange?: (value: SelectValue<SelectType>) => void
  defaultOpen?: boolean
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
  defaultValue,
  value: valueProp,
  onValueChange,
  defaultOpen,
  open: openProp,
  onOpenChange,
  ...props
}: SelectProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(defaultOpen ?? false)
  const [value, setValue] = useState<SelectValue<typeof type>>(
    defaultValue ?? (type === 'checkbox' ? [] : ''),
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
    if (onOpenChange) {
      onOpenChange(open)
      return
    }
    setOpen(open)
  }

  const handleValueChange = (nextValue: string) => {
    if (onValueChange) {
      onValueChange(nextValue)
      return
    }

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
        open: openProp ?? open,
        setOpen: handleOpenChange,
        value: valueProp ?? value,
        setValue: handleValueChange,
        type,
      }}
    >
      <Box ref={ref} display="inline-block" pos="relative" {...props}>
        {children}
      </Box>
    </SelectContext.Provider>
  )
}

interface SelectTriggerProps extends ComponentProps<typeof Button> {
  asChild?: boolean
}
export function SelectTrigger({
  className,
  children,
  asChild,
  ...props
}: SelectTriggerProps) {
  const { open, setOpen } = useSelect()
  const handleClick = () => {
    setOpen(!open)
  }

  if (asChild) {
    const element = Children.only(children) as ReactElement<
      ComponentProps<keyof JSX.IntrinsicElements | JSXElementConstructor<any>>
    >
    const Comp = element.type
    return <Comp onClick={handleClick} {...element.props} />
  }

  return (
    <Button
      className={clsx(
        css({
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
      h="fit-content"
      p="10px"
      pos="absolute"
      transform="translateY(100%)"
      userSelect="none"
      w="232px"
      {...props}
    >
      {children}
    </VStack>
  )
}

interface SelectOptionProps extends Omit<ComponentProps<'div'>, 'onClick'> {
  onClick?: (value?: string, e?: React.MouseEvent<HTMLDivElement>) => void
  disabled?: boolean
  value?: string
}

export function SelectOption({
  disabled,
  onClick,
  children,
  value,
  ...props
}: SelectOptionProps) {
  const { setOpen, setValue, value: selectedValue, type } = useSelect()

  const handleClose = () => {
    if (type === 'checkbox') return
    setOpen(false)
  }

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (onClick) {
      onClick(value, e)
      return
    }
    if (typeof value === 'string') setValue(value)
    handleClose()
  }

  const isSelected = {
    default: false,
    radio: selectedValue === value,
    checkbox:
      Array.isArray(selectedValue) && value && selectedValue.includes(value),
  }[type]

  const changesOnHover = !disabled && !(type === 'radio' && isSelected)

  return (
    <Flex
      _hover={
        changesOnHover && {
          bg: '$primaryBg',
        }
      }
      alignItems="center"
      borderRadius="8px"
      color={disabled ? '$selectDisabled' : isSelected ? '$primary' : '$title'}
      cursor={changesOnHover ? 'pointer' : 'default'}
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
      typography={isSelected ? 'inputBold' : 'inputText'}
      {...props}
    >
      {
        {
          checkbox: (
            <Box
              bg={isSelected ? '$primary' : '$border'}
              borderRadius="4px"
              boxSize="18px"
              pos="relative"
              transition="background-color 0.1s ease-in-out"
            >
              {isSelected && (
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
              {isSelected && (
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
