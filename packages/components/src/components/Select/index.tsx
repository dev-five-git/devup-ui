'use client'

import {
  Box,
  css,
  type DevupThemeTypographyKeys,
  Flex,
  VStack,
} from '@devup-ui/react'
import clsx from 'clsx'
import {
  ComponentProps,
  isValidElement,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react'

import { SelectContext, useSelect } from '../../contexts/useSelect'
import { SelectType, SelectValue } from '../../types/select'
import { Button } from '../Button'
import { IconCheck } from './IconCheck'

interface SelectProps extends Omit<ComponentProps<'div'>, 'onChange'> {
  defaultValue?: SelectValue<SelectType>
  value?: SelectValue<SelectType>
  onChange?: (value: string) => void
  defaultOpen?: boolean
  open?: boolean
  onOpenChange?: (open: boolean) => void
  children: React.ReactNode
  type?: SelectType
  colors?: {
    primary?: string
    border?: string
    inputBackground?: string
    inputDisabledBackground?: string
    inputDisabledText?: string
    base10?: string
    title?: string
    selectDisabled?: string
    primaryBg?: string
  }
  typography?: DevupThemeTypographyKeys
  options?: {
    label?: string
    disabled?: boolean
    onClick?: (
      value: string | undefined,
      e?: React.MouseEvent<HTMLDivElement>,
    ) => void
    showCheck?: boolean
    value: string
  }[]
  triggerProps?: ComponentProps<typeof SelectTrigger>
  containerProps?: ComponentProps<typeof SelectContainer>
  optionProps?: ComponentProps<typeof SelectOption>
}

export function Select({
  type = 'default',
  children,
  defaultValue,
  value: valueProp,
  onChange,
  defaultOpen,
  open: openProp,
  onOpenChange,
  colors,
  typography,
  options,
  triggerProps,
  containerProps,
  optionProps,
  ...props
}: SelectProps) {
  const ref = useRef<HTMLDivElement>(null)
  const generatedId = useId()
  const listboxId = `${props.id ?? generatedId}-listbox`
  const [open, setOpen] = useState(defaultOpen ?? false)
  const isOpen = openProp ?? open
  const [value, setValue] = useState<SelectValue<typeof type>>(
    defaultValue ?? (type === 'checkbox' ? [] : ''),
  )

  // A controlled select only tells its owner, which decides whether it closes
  useEffect(() => {
    if (!isOpen) return
    const handleOutsideClick = (e: MouseEvent) => {
      if (ref.current && ref.current.contains(e.target as Node)) return
      onOpenChange?.(false)
      if (openProp === undefined) setOpen(false)
    }
    document.addEventListener('click', handleOutsideClick)
    return () => document.removeEventListener('click', handleOutsideClick)
  }, [isOpen, openProp, onOpenChange])

  const handleOpenChange = (next: boolean) => {
    onOpenChange?.(next)
    if (openProp === undefined) setOpen(next)
  }

  const handleValueChange = (nextValue: string) => {
    onChange?.(nextValue)

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
    <SelectContext
      value={{
        open: isOpen,
        setOpen: handleOpenChange,
        listboxId,
        value: valueProp ?? value,
        setValue: handleValueChange,
        type,
        ref,
      }}
    >
      <Box
        ref={ref}
        display="inline-block"
        h="fit-content"
        selectors={{
          '&, & *': {
            boxSizing: 'border-box',
          },
        }}
        styleOrder={1}
        styleVars={{
          primary: colors?.primary,
          border: colors?.border,
          inputBackground: colors?.inputBackground,
          base10: colors?.base10,
          title: colors?.title,
          selectDisabled: colors?.selectDisabled,
          primaryBg: colors?.primaryBg,
          inputDisabledBackground: colors?.inputDisabledBackground,
          inputDisabledText: colors?.inputDisabledText,
        }}
        typography={typography}
        {...props}
      >
        {options ? (
          <>
            <SelectTrigger {...triggerProps}>{children}</SelectTrigger>
            <SelectContainer {...containerProps}>
              {options?.map((option) => (
                <SelectOption
                  key={'option-' + option.value}
                  {...optionProps}
                  {...option}
                >
                  {option.label ?? option.value}
                </SelectOption>
              ))}
            </SelectContainer>
          </>
        ) : (
          children
        )}
      </Box>
    </SelectContext>
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
  const { open, setOpen, listboxId } = useSelect()
  const handleClick = () => {
    setOpen(!open)
  }
  const handleKeyDown = (e: React.KeyboardEvent<HTMLElement>) => {
    if (open || (e.key !== 'ArrowDown' && e.key !== 'ArrowUp')) return
    e.preventDefault()
    setOpen(true)
  }

  if (asChild) {
    if (!isValidElement<Record<string, unknown>>(children)) {
      throw new Error('SelectTrigger with asChild requires a single element')
    }

    const Comp = children.type
    const childProps = {
      'aria-controls': listboxId,
      'aria-expanded': open,
      'aria-haspopup': 'listbox',
      'aria-label': 'Select toggle',
      onKeyDown: handleKeyDown,
      onClick: children.props.onClick ?? handleClick,
      ...children.props,
    }
    return <Comp {...childProps} />
  }

  return (
    <Button
      aria-controls={listboxId}
      aria-expanded={open}
      aria-haspopup="listbox"
      aria-label="Select toggle"
      className={clsx(
        css({
          borderRadius: '8px',
          styleOrder: 2,
        }),
        className,
      )}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      {...props}
    >
      {children}
    </Button>
  )
}

interface SelectContainerProps extends ComponentProps<'div'> {
  showConfirmButton?: boolean
  confirmButtonText?: string
  x?: number
  y?: number
}
export function SelectContainer({
  children,
  showConfirmButton,
  confirmButtonText = '완료',
  x = 0,
  y = 0,
  ...props
}: SelectContainerProps) {
  const { open, setOpen, type, ref, listboxId } = useSelect()

  // Opening moves focus to the selected option, or the first one
  useEffect(() => {
    if (!open) return
    const listbox = document.getElementById(listboxId)
    const option =
      listbox?.querySelector<HTMLElement>(
        '[role=option][aria-selected=true]:not([aria-disabled=true])',
      ) ??
      listbox?.querySelector<HTMLElement>(
        '[role=option]:not([aria-disabled=true])',
      )
    option?.focus()
  }, [open, listboxId])

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const options = Array.from(
      e.currentTarget.querySelectorAll<HTMLElement>(
        '[role=option]:not([aria-disabled=true])',
      ),
    )
    const index = options.findIndex(
      (option) => option === document.activeElement,
    )
    const moves: Partial<Record<string, number>> = {
      ArrowDown: index + 1,
      ArrowUp: index - 1,
      Home: 0,
      End: options.length - 1,
    }
    const next = moves[e.key]
    if (next !== undefined) {
      e.preventDefault()
      options[(next + options.length) % options.length]?.focus()
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      setOpen(false)
      ref.current
        ?.querySelector<HTMLElement>('[aria-haspopup=listbox]')
        ?.focus()
    }
  }

  if (!open) return null
  return (
    <VStack
      ref={(el) => {
        if (!ref.current || !el) return
        const combobox = ref.current

        // 요소가 움직일 때마다(스크롤, 리사이즈 등) 위치를 갱신하도록 이벤트를 등록합니다.
        const updatePosition = () => {
          const {
            height,
            x: comboboxX,
            y: comboboxY,
            top,
            left,
          } = combobox.getBoundingClientRect()

          const isOverflowBottom =
            el.offsetHeight + top + window.scrollY + height + y >
            document.documentElement.scrollHeight

          const isOverflowRight =
            el.offsetWidth + left + window.scrollX + x >
            document.documentElement.scrollWidth

          if (isOverflowBottom)
            el.style.bottom = `${window.innerHeight - comboboxY + 10}px`
          else el.style.top = `${comboboxY + height + 10 + y}px`

          if (isOverflowRight)
            el.style.left = `${Math.max(comboboxX - el.offsetWidth + combobox.offsetWidth, 0) + x}px`
          else el.style.left = `${comboboxX + x}px`
        }

        // 최초 위치 설정
        updatePosition()

        // 스크롤, 리사이즈, DOM 변경 등 요소 위치가 변할 수 있는 이벤트에 리스너 등록
        window.addEventListener('scroll', updatePosition, true)
        window.addEventListener('resize', updatePosition)

        // 컴포넌트 언마운트 시 이벤트 해제
        return () => {
          window.removeEventListener('scroll', updatePosition, true)
          window.removeEventListener('resize', updatePosition)
        }
      }}
      aria-label="Select container"
      aria-multiselectable={type === 'checkbox' || undefined}
      bg="var(--inputBg, light-dark(#FFF,#2E2E2E))"
      border="1px solid var(--border, light-dark(#E4E4E4,#434343))"
      borderRadius="8px"
      bottom="-4px"
      boxShadow="0 2px 2px 0 var(--base10, light-dark(#0000001A,#FFFFFF1A))"
      boxSize="fit-content"
      gap="6px"
      id={listboxId}
      minW="232px"
      onKeyDown={handleKeyDown}
      p="10px"
      pos="fixed"
      role="listbox"
      styleOrder={1}
      userSelect="none"
      zIndex={1}
      {...props}
    >
      {children}
      {showConfirmButton && type === 'checkbox' && (
        <Flex justifyContent="end" w="100%">
          <Button
            aria-label="Select confirm button"
            className={css({
              textAlign: 'end',
              bg: 'var(--primary, light-dark(#674DC7, #8163E1))',
              borderRadius: '8px',
              w: 'fit-content',
              px: '30px',
              py: '10px',
              color: '#FFF',
              typography: 'buttonS',
            })}
            onClick={() => setOpen(false)}
            variant="primary"
          >
            {confirmButtonText}
          </Button>
        </Flex>
      )}
    </VStack>
  )
}

interface SelectOptionProps extends Omit<ComponentProps<'div'>, 'onClick'> {
  value?: string
  disabled?: boolean
  onClick?: (
    value: string | undefined,
    e?: React.MouseEvent<HTMLDivElement>,
  ) => void
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
  const { setOpen, setValue, value: selectedValue, type, ref } = useSelect()

  const handleClose = () => {
    if (type === 'checkbox') return
    setOpen(false)
    ref.current?.querySelector<HTMLElement>('[aria-haspopup=listbox]')?.focus()
  }

  const handleClick = (e?: React.MouseEvent<HTMLDivElement>) => {
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
      aria-disabled={disabled}
      aria-label="Select option"
      aria-selected={!!isSelected}
      borderRadius="6px"
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
      onClick={disabled ? undefined : (e) => handleClick(e)}
      onKeyDown={(e) => {
        if (disabled || (e.key !== 'Enter' && e.key !== ' ')) return
        e.preventDefault()
        handleClick()
      }}
      px="10px"
      role="option"
      styleOrder={1}
      tabIndex={disabled ? undefined : -1}
      transition="background-color 0.1s ease-in-out"
      {...props}
    >
      {showCheck &&
        {
          checkbox: (
            <Box
              bg={
                disabled
                  ? 'var(--inputDisabledBackground, light-dark(#F0F0F3, #414244))'
                  : isSelected
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
                    color: disabled
                      ? 'var(--inputDisabledText, light-dark(#E5E5E5, #373737))'
                      : '#FFF',
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
