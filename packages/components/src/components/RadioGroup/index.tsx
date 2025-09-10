'use client'
import { Flex } from '@devup-ui/react'
import { useState } from 'react'

import { Radio } from '../Radio'

interface RadioGroupProps {
  options: {
    value: string | number | boolean
    label: React.ReactNode
  }[]
  disabled?: boolean
  direction?: 'row' | 'column'
  variant?: 'default' | 'button'
  style?: React.CSSProperties
  value?: string | number | boolean
  onChange?: (value: string | number | boolean) => void
  defaultValue?: string | number | boolean
  className?: string
  colors?: {
    primary?: string
    border?: string
    text?: string
    bg?: string
    hoverBg?: string
    hoverBorder?: string
    hoverColor?: string
    checkedBg?: string
    checkedBorder?: string
    checkedColor?: string
    disabledBg?: string
    disabledColor?: string
  }
  classNames?: {
    label?: string
    container?: string
  }
  styles?: {
    label?: React.CSSProperties
    container?: React.CSSProperties
  }
}
export function RadioGroup({
  disabled,
  options,
  direction = 'row',
  variant = 'default',
  style,
  value,
  onChange,
  defaultValue,
  colors,
  className,
  classNames,
  styles,
}: RadioGroupProps) {
  const [innerValue, setInnerValue] = useState(
    value ? String(value) : defaultValue ? String(defaultValue) : undefined,
  )
  const resultValue = value ? String(value) : (innerValue ?? '')

  function handleChange(_value: string) {
    onChange?.(_value)
    setInnerValue(_value)
  }

  return (
    <Flex
      className={classNames?.container}
      flexDir={variant === 'button' ? 'row' : direction}
      gap={variant === 'button' ? 0 : direction === 'row' ? '30px' : '16px'}
      style={styles?.container}
    >
      {options.map(({ value: optionValue, label }, idx) => {
        const stringValue = String(optionValue)
        const props = {
          checked: resultValue === stringValue,
          disabled,
          onChange: () => !disabled && handleChange(stringValue),
          className,
          classNames,
          styles,
          style,
        } as const
        return variant === 'button' ? (
          <Radio
            key={stringValue}
            colors={colors}
            firstButton={idx === 0}
            lastButton={idx === options.length - 1}
            variant={variant}
            {...props}
          >
            {label}
          </Radio>
        ) : (
          <Radio key={stringValue} colors={colors} variant={variant} {...props}>
            {label}
          </Radio>
        )
      })}
    </Flex>
  )
}
