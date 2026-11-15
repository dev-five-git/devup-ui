'use client'

import { css, Flex } from '@devup-ui/react'
import clsx from 'clsx'
import {
  ComponentProps,
  ComponentPropsWithoutRef,
  createContext,
  forwardRef,
  useContext,
  useState,
} from 'react'

import { Button } from '../Button'
import { Input } from '../Input'
import { IconMinus } from './IconMinus'
import { IconPlus } from './IconPlus'

type StepperContextType = {
  value: number
  setValue: (value: number) => void
  min: number
  max: number
  type: 'input' | 'text'
}

const StepperContext = createContext<StepperContextType | null>(null)

export const useStepper = () => {
  const context = useContext(StepperContext)
  if (!context) {
    throw new Error('useStepper must be used within a StepperProvider')
  }
  return context
}

type StepperProps = {
  children?: React.ReactNode
  defaultValue?: number
  value?: number
  onValueChange?: (value: number) => void
  min?: number
  max?: number
  type?: 'input' | 'text'
}

function Stepper({
  children,
  defaultValue,
  value: valueProp,
  onValueChange,
  min = 0,
  max = 100,
  type = 'input',
}: StepperProps) {
  const [value, setValue] = useState(defaultValue ?? 0)

  const handleChange = (nextValue: number) => {
    const sanitized = Math.min(Math.max(nextValue, min), max)
    if (valueProp === undefined) setValue(sanitized)
    onValueChange?.(sanitized)
  }

  return (
    <StepperContext.Provider
      value={{
        value: valueProp ?? value,
        setValue: handleChange,
        min,
        max,
        type,
      }}
    >
      {children}
    </StepperContext.Provider>
  )
}

// forwardRef: React 18 does not pass `ref` to a function component as a prop
const StepperContainer = forwardRef<
  HTMLDivElement,
  ComponentPropsWithoutRef<'div'>
>(function StepperContainer(props, ref) {
  return (
    <Flex ref={ref} alignItems="center" gap="8px" styleOrder={1} {...props} />
  )
})

const StepperDecreaseButton = forwardRef<
  HTMLButtonElement,
  ComponentPropsWithoutRef<typeof Button>
>(function StepperDecreaseButton(
  { onClick, disabled: disabledProp, ...props },
  ref,
) {
  const stepper = useStepper()
  const { value, min } = stepper
  const disabled = value <= min
  return (
    <Button
      aria-label="Decrease button"
      className={css({
        p: '0',
        boxSize: '28px',
        borderRadius: '4px',
      })}
      {...props}
      ref={ref}
      disabled={disabled || disabledProp}
      onClick={(e) => {
        onClick?.(e)
        if (!e.defaultPrevented) stepper.setValue(value - 1)
      }}
    >
      <IconMinus
        className={css({
          color: disabled
            ? 'var(--base10, light-dark(#0000001A,#FFFFFF1A))'
            : 'var(--text, light-dark(#272727, #F6F6F6))',
        })}
      />
    </Button>
  )
})

const StepperIncreaseButton = forwardRef<
  HTMLButtonElement,
  ComponentPropsWithoutRef<typeof Button>
>(function StepperIncreaseButton(
  { onClick, disabled: disabledProp, ...props },
  ref,
) {
  const stepper = useStepper()
  const { value, max } = stepper
  const disabled = value >= max
  return (
    <Button
      aria-label="Increase button"
      className={css({
        p: '0',
        boxSize: '28px',
        borderRadius: '4px',
        selectors: {
          '&>div>div': {},
        },
      })}
      {...props}
      ref={ref}
      disabled={disabled || disabledProp}
      onClick={(e) => {
        onClick?.(e)
        if (!e.defaultPrevented) stepper.setValue(value + 1)
      }}
    >
      <IconPlus
        className={css({
          color: disabled
            ? 'var(--base10, light-dark(#0000001A,#FFFFFF1A))'
            : 'var(--text, light-dark(#272727, #F6F6F6))',
        })}
      />
    </Button>
  )
})

type StepperInputProps = ComponentPropsWithoutRef<typeof Input>

const notEditableClass = css({
  p: '0',
  border: 'none',
  w: 'fit-content',
  h: 'fit-content',
  styleOrder: 3,
})

const stepperValueClass = css({
  styleOrder: 2,
  w: '60px',
  textAlign: 'center',
  borderRadius: '6px',
  selectors: {
    '&::-webkit-outer-spin-button, &::-webkit-inner-spin-button': {
      display: 'none',
    },
  },
})

/** The props a read-only value can take: the ones every element has, not those only an input has */
function outputProps(props: StepperInputProps) {
  return Object.fromEntries(
    Object.entries(props).filter(
      ([name]) =>
        name.startsWith('data-') ||
        name.startsWith('aria-') ||
        ['id', 'style', 'title', 'lang', 'dir', 'hidden', 'role'].includes(
          name,
        ),
    ),
  )
}

/** The ref is the `<input>`, or the `<output>` of a `type="text"` stepper */
const StepperInput = forwardRef<HTMLElement, StepperInputProps>(
  function StepperInput({ className, onChange, ...props }, ref) {
    const stepper = useStepper()
    const { value, type } = stepper

    if (type === 'text')
      return (
        <output
          ref={ref as React.Ref<HTMLOutputElement>}
          aria-label="Stepper value"
          className={clsx(stepperValueClass, notEditableClass, className)}
          data-value={value}
          {...outputProps(props)}
        >
          {Number(value).toString()}
        </output>
      )

    return (
      <Input
        aria-label="Stepper value"
        className={clsx(stepperValueClass, className)}
        data-value={value}
        {...props}
        ref={ref as React.Ref<HTMLInputElement>}
        allowClear={false}
        onChange={(e) => {
          onChange?.(e)
          if (!e.defaultPrevented) stepper.setValue(Number(e.target.value))
        }}
        type="number"
        // Fix prefix 0 issue
        value={value.toString()}
      />
    )
  },
)
export {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
}
