'use client'

import { css, Flex, Text } from '@devup-ui/react'
import clsx from 'clsx'
import { ChangeEvent, useState } from 'react'

import { Button } from '../Button'
import { Input } from '../Input'

type InputProps = Omit<
  React.InputHTMLAttributes<HTMLInputElement>,
  | 'value'
  | 'onChange'
  | 'defaultValue'
  | 'checked'
  | 'defaultChecked'
  | 'min'
  | 'max'
> & {
  type?: 'text' | 'input'
  value?: number
  onChange?: (value: number) => void
  defaultValue?: number
  min?: number | null
  max?: number | null
  classNames?: {
    button?: string
    container?: string
    input?: string
  }
  styles?: {
    button?: React.CSSProperties
    container?: React.CSSProperties
    input?: React.CSSProperties
  }
}

function valid(min: number | null, max: number | null, inp: number): boolean {
  return !((max !== null && inp > max) || (min !== null && inp < min))
}

export function Stepper({
  className,
  classNames,
  type = 'input',
  value,
  onChange,
  defaultValue,
  min = 0,
  max = 100,
  disabled = false,
  styles,
  ...props
}: InputProps) {
  const [internalValue, setInternalValue] = useState(value ?? defaultValue ?? 0)
  const resultValue = value ?? internalValue
  function handleChange(_value: number) {
    onChange?.(_value)
    setInternalValue(_value)
  }
  function handleInputChange(event: ChangeEvent<HTMLInputElement>) {
    if (!event.target.value.length) {
      const _value =
        min !== null && min > 0 ? min : max !== null && max < 0 ? max : 0
      handleChange(_value)
      return
    }
    const _value = parseInt(event.target.value)
    if (!valid(min, max, _value)) {
      return
    }
    handleChange(_value)
  }

  const handleClick = (type: 'add' | 'sub') => {
    const targetValue = resultValue + (type === 'sub' ? -1 : 1)
    handleChange(targetValue)
  }

  return (
    <Flex
      alignItems="center"
      aria-disabled={disabled}
      gap={5}
      selectors={{ '&, & *': { boxSizing: 'border-box' } }}
    >
      <Button
        className={clsx(
          css({
            boxSize: '28px',
            p: 0,
            borderRadius: 1,
            fontSize: '18px',
            fontWeight: 500,
            styleOrder: 2,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }),
          classNames?.button,
        )}
        disabled={disabled || resultValue === min}
        onClick={() => handleClick('sub')}
        style={styles?.button}
      >
        -
      </Button>
      {type === 'text' && (
        <Text className={className} fontSize="14px">
          {resultValue}
        </Text>
      )}
      <Input
        className={clsx(
          css({
            textAlign: 'center',
            color: '$text',
            p: 0,
            _placeholder: { fontSize: ['13px', null, '14px'] },
            _invalid: {
              borderColor: '$error',
            },
            selectors: {
              '&::-webkit-outer-spin-button, &::-webkit-inner-spin-button': {
                display: 'none',
              },
            },
            styleOrder: 2,
          }),
          classNames?.input,
        )}
        data-value={resultValue.toString()}
        disabled={disabled}
        onChange={handleInputChange}
        readOnly={type === 'text'}
        style={{
          display: type === 'text' ? 'none' : 'block',
        }}
        type="number"
        {...props}
        value={resultValue.toString()}
      />
      <Button
        className={clsx(
          css({
            boxSize: '28px',
            p: 0,
            borderRadius: 1,
            fontSize: '18px',
            fontWeight: 500,
            styleOrder: 2,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }),
          classNames?.button,
        )}
        disabled={disabled || resultValue === max}
        onClick={() => handleClick('add')}
        style={styles?.button}
      >
        +
      </Button>
    </Flex>
  )
}
