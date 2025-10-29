'use client'

import { css, Flex } from '@devup-ui/react'
import clsx from 'clsx'
import { ComponentProps, createContext, useContext, useState } from 'react'

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
    if (onValueChange) {
      onValueChange(sanitized)
      return
    }
    setValue(sanitized)
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

function StepperContainer(props: ComponentProps<'div'>) {
  return <Flex alignItems="center" gap="8px" styleOrder={1} {...props} />
}

function StepperDecreaseButton({ ...props }: ComponentProps<typeof Button>) {
  const { value, setValue, min } = useStepper()
  const disabled = value <= min
  return (
    <Button
      aria-label="Decrease button"
      className={css({
        p: '0',
        boxSize: '28px',
        borderRadius: '4px',
      })}
      disabled={disabled}
      onClick={() => setValue(value - 1)}
      {...props}
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
}

function StepperIncreaseButton({ ...props }: ComponentProps<typeof Button>) {
  const { value, setValue, max } = useStepper()
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
      disabled={disabled}
      onClick={() => setValue(value + 1)}
      {...props}
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
}

type StepperInputProps = ComponentProps<typeof Input>

function StepperInput({ className, ...props }: StepperInputProps) {
  const { value, setValue, type } = useStepper()
  const notEditableClass = css({
    p: '0',
    border: 'none',
    w: 'fit-content',
    h: 'fit-content',
    styleOrder: 3,
  })
  const isInput = type === 'input'
  const Comp = isInput ? Input : 'div'
  if (isInput) {
    // div tag doesn't support allowClear prop
    Object.assign(props, { allowClear: false })
  }

  return (
    <Comp
      aria-label="Stepper value"
      className={clsx(
        css({
          styleOrder: 2,
          w: '60px',
          textAlign: 'center',
          borderRadius: '6px',
          selectors: {
            '&::-webkit-outer-spin-button, &::-webkit-inner-spin-button': {
              display: 'none',
            },
          },
        }),
        !isInput && notEditableClass,
        className,
      )}
      dangerouslySetInnerHTML={
        isInput ? undefined : { __html: Number(value).toString() }
      }
      data-value={value}
      onChange={(e) => {
        setValue(Number(e.target.value))
      }}
      readOnly={!isInput}
      type="number"
      // Fix prefix 0 issue
      value={value.toString()}
      {...props}
    />
  )
}

export {
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
}
