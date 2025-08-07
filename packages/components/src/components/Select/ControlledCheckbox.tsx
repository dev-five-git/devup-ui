'use client'

import { css, Flex } from '@devup-ui/react'
import { useState } from 'react'

import {
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
} from '.'
import { IconArrow } from './IconArrow'

export function ControlledCheckbox() {
  const [value, setValue] = useState<string[]>([])
  const handleChange = (nextValue: string) => {
    if (value.includes(nextValue)) {
      setValue(value.filter((v) => v !== nextValue))
    } else {
      setValue([...value, nextValue])
    }
  }

  const [subValue, setSubValue] = useState<string[]>([])
  const handleSubChange = (nextValue: string) => {
    if (subValue.includes(nextValue)) {
      setSubValue(subValue.filter((v) => v !== nextValue))
    } else {
      setSubValue([...subValue, nextValue])
    }
  }

  return (
    <Select onValueChange={handleChange} type="checkbox" value={value}>
      <SelectTrigger>Select {value}</SelectTrigger>
      <SelectContainer>
        <SelectOption value="Option 1">Option 1</SelectOption>
        <SelectOption value="Option 2">Option 2</SelectOption>
        <SelectDivider />
        <SelectOption value="Option 3">Option 3</SelectOption>
        <SelectOption value="Option 4">Option 4</SelectOption>
        <Select
          onValueChange={handleSubChange}
          type="checkbox"
          value={subValue}
        >
          <SelectTrigger asChild>
            <SelectOption>
              <Flex alignItems="center" justifyContent="space-between" w="100%">
                Option 5<IconArrow />
              </Flex>
            </SelectOption>
          </SelectTrigger>
          <SelectContainer
            className={css({
              right: '0',
              top: '0',
              transform: 'translateX(100%)',
            })}
          >
            <SelectOption value="Option 6">Option 6</SelectOption>
            <SelectOption value="Option 7">Option 7</SelectOption>
          </SelectContainer>
        </Select>
      </SelectContainer>
    </Select>
  )
}
