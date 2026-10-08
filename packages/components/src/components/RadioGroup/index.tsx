'use client'
import { Flex } from '@devup-ui/react'
import { useId, useState } from 'react'

import { Radio } from '../Radio'

type RadioValue = string | number | boolean

interface RadioGroupProps<T extends RadioValue> {
  options: {
    value: T
    label: React.ReactNode
  }[]
  disabled?: boolean
  direction?: 'row' | 'column'
  variant?: 'default' | 'button'
  style?: React.CSSProperties
  value?: T
  onChange?: (value: T) => void
  defaultValue?: T
  /** The name the radios share, generated when not given */
  name?: string
  /** The accessible name of the group */
  label?: string
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
export function RadioGroup<T extends RadioValue>({
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
  name,
  label,
}: RadioGroupProps<T>) {
  const generatedName = useId()
  const [innerValue, setInnerValue] = useState<T | undefined>(defaultValue)
  const resultValue = value !== undefined ? value : innerValue

  function handleChange(next: T) {
    onChange?.(next)
    setInnerValue(next)
  }

  return (
    <Flex
      aria-disabled={disabled}
      aria-label={label}
      className={classNames?.container}
      flexDir={variant === 'button' ? 'row' : direction}
      gap={variant === 'button' ? 0 : direction === 'row' ? '30px' : '16px'}
      role="radiogroup"
      style={styles?.container}
    >
      {options.map(({ value: optionValue, label }, idx) => {
        const stringValue = String(optionValue)
        const props = {
          checked: resultValue === optionValue,
          disabled,
          name: name ?? generatedName,
          value: stringValue,
          onChange: () => !disabled && handleChange(optionValue),
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
