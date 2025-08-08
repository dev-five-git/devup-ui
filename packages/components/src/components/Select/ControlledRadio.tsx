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

export function ControlledRadio() {
  const [value, setValue] = useState('')
  const handleChange = (value: string) => {
    setValue(value)
  }
  const [subValue, setSubValue] = useState('')
  const handleSubChange = (value: string) => {
    setSubValue(value)
  }
  return (
    <Select onValueChange={handleChange} type="radio" value={value}>
      <SelectTrigger>Select {value}</SelectTrigger>
      <SelectContainer>
        <SelectOption value="Option 1">Option 1</SelectOption>
        <SelectOption value="Option 2">Option 2</SelectOption>
        <SelectDivider />
        <SelectOption value="Option 3">Option 3</SelectOption>
        <SelectOption value="Option 4">Option 4</SelectOption>
        <Select onValueChange={handleSubChange} type="radio" value={subValue}>
          <SelectTrigger asChild>
            <SelectOption showCheck={false}>
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
            <SelectOption
              onClick={(value) => {
                if (value) {
                  setSubValue(value)
                }
              }}
              value="Option 6"
            >
              Option 6
            </SelectOption>
            <SelectOption
              onClick={(value) => {
                if (value) {
                  setSubValue(value)
                }
              }}
              value="Option 7"
            >
              Option 7
            </SelectOption>
          </SelectContainer>
        </Select>
      </SelectContainer>
    </Select>
  )
}
