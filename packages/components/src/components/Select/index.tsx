'use client'

import { Box, css, DevupThemeTypography, Flex, VStack } from '@devup-ui/react'
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

interface SelectProps extends ComponentProps<'div'> {
  defaultValue?: SelectValue<SelectType>
  value?: SelectValue<SelectType>
  onValueChange?: (value: string) => void
  defaultOpen?: boolean
  open?: boolean
  onOpenChange?: (open: boolean) => void
  children: React.ReactNode
  type?: SelectType
  colors?: {
    primary?: string
    border?: string
    inputBg?: string
    base10?: string
    title?: string
    selectDisabled?: string
    primaryBg?: string
  }
  typography?: keyof DevupThemeTypography
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
  colors,
  typography,
  ...props
}: SelectProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [open, setOpen] = useState(defaultOpen ?? false)
  const [value, setValue] = useState<SelectValue<typeof type>>(
    defaultValue ?? (type === 'checkbox' ? [] : ''),
  )

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (ref.current && ref.current.contains(e.target as Node)) return
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
      <Box
        ref={ref}
        display="inline-block"
        pos="relative"
        selectors={{
          '&, & *': {
            boxSizing: 'border-box',
          },
        }}
        styleOrder={1}
        styleVars={{
          primary: colors?.primary,
          border: colors?.border,
          inputBg: colors?.inputBg,
          base10: colors?.base10,
          title: colors?.title,
          selectDisabled: colors?.selectDisabled,
          primaryBg: colors?.primaryBg,
        }}
        typography={typography}
        {...props}
      >
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
    return (
      <Comp
        aria-expanded={open}
        aria-label="Select toggle"
        onClick={handleClick}
        {...element.props}
      />
    )
  }

  return (
    <Button
      aria-expanded={open}
      aria-label="Select toggle"
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

interface SelectContainerProps extends ComponentProps<'div'> {
  showConfirmButton?: boolean
}
export function SelectContainer({
  children,
  showConfirmButton,
  ...props
}: SelectContainerProps) {
  const { open, setOpen, type } = useSelect()

  if (!open) return null
  return (
    <VStack
      aria-label="Select container"
      bg="var(--inputBg, light-dark(#FFF,#2E2E2E))"
      border="1px solid var(--border, light-dark(#E4E4E4,#434343))"
      borderRadius="8px"
      bottom="-4px"
      boxShadow="0 2px 2px 0 var(--base10, light-dark(#0000001A,#FFFFFF1A))"
      gap="6px"
      h="fit-content"
      p="10px"
      pos="absolute"
      styleOrder={1}
      transform="translateY(100%)"
      userSelect="none"
      w="232px"
      {...props}
    >
      {children}
      {showConfirmButton && type === 'checkbox' && (
        <Flex justifyContent="end" w="100%">
          <Button
            className={css({
              textAlign: 'end',
              bg: 'var(--primary, light-dark(#674DC7, #8163E1))',
              borderRadius: '8px',
              w: 'fit-content',
              px: '30px',
              py: '10px',
              color: 'var(--white, light-dark(#FFF,#FFF))',
              typography: 'buttonS',
            })}
            onClick={() => setOpen(false)}
            variant="primary"
          >
            완료
          </Button>
        </Flex>
      )}
    </VStack>
  )
}

interface SelectOptionProps extends Omit<ComponentProps<'div'>, 'onClick'> {
  onClick?: (
    value: string | undefined,
    e?: React.MouseEvent<HTMLDivElement>,
  ) => void
  disabled?: boolean
  value?: string
  showCheck?: boolean
}

export function SelectOption({
  disabled,
  onClick,
  children,
  value,
  showCheck = true,
  ...props
}: SelectOptionProps) {
  const { setOpen, setValue, value: selectedValue, type } = useSelect()

  const handleClose = () => {
    if (type === 'checkbox') return
    setOpen(false)
  }

  const handleClick = (
    value: string | undefined,
    e: React.MouseEvent<HTMLDivElement>,
  ) => {
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
          bg: 'var(--primaryBg, light-dark(#F4F3FA, #F4F3FA0D))',
        }
      }
      alignItems="center"
      borderRadius="8px"
      color={
        disabled
          ? 'var(--selectDisabled, light-dark(#C4C5D1, #45464D))'
          : isSelected
            ? 'var(--primary, light-dark(#674DC7, #8163E1)'
            : 'var(--title, light-dark(#1A1A1A,#FAFAFA))'
      }
      cursor={changesOnHover ? 'pointer' : 'default'}
      data-value={value}
      fontWeight={isSelected ? '700' : '400'}
      gap={
        {
          checkbox: '10px',
          radio: '6px',
          default: '0',
        }[type]
      }
      h="40px"
      onClick={disabled ? undefined : (e) => handleClick(value, e)}
      px="10px"
      styleOrder={1}
      transition="background-color 0.1s ease-in-out"
      {...props}
    >
      {showCheck &&
        {
          checkbox: (
            <Box
              bg={
                isSelected
                  ? 'var(--primary, light-dark(#674DC7, #8163E1)'
                  : 'var(--border, light-dark(#E4E4E4, #434343))'
              }
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
                      color: 'inherit',
                    })}
                  />
                </Box>
              )}
            </>
          ),
          default: null,
        }[type]}
      {children}
    </Flex>
  )
}

export function SelectDivider({ ...props }: ComponentProps<'div'>) {
  return (
    <Box
      bg="var(--border, light-dark(#E4E4E4,#434343)"
      h="1px"
      styleOrder={1}
      w="100%"
      {...props}
    />
  )
}
