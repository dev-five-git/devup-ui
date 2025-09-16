import { Flex } from '@devup-ui/react'
import { useState } from 'react'

import { Checkbox } from '../Checkbox'

export interface CheckboxItem {
  id: string
  value: React.ReactNode
  label: string
  disabled?: boolean
  checked?: boolean
}

export interface CheckboxChangeEvent {
  id: string
  value: React.ReactNode
  checked: boolean
  checkedValues: React.ReactNode[]
}

export interface CheckBoxLayerProps {
  checkboxes: CheckboxItem[]
  flexDir: 'row' | 'column'
  gap?: number
  onCheckboxChange?: (event: CheckboxChangeEvent) => void
  defaultCheckedIds?: string[]
  variant?: 'primary' | 'default'
}

export function CheckboxLayer({
  checkboxes,
  flexDir,
  gap,
  onCheckboxChange,
  defaultCheckedIds = [],
  variant = 'primary',
}: CheckBoxLayerProps) {
  const [checkedIds, setCheckedIds] = useState<string[]>(defaultCheckedIds)

  const handleCheckboxChange = (
    id: string,
    value: React.ReactNode,
    checked: boolean,
  ) => {
    const updatedIds = checked
      ? [...checkedIds, id]
      : checkedIds.filter((checkedId) => checkedId !== id)

    setCheckedIds(updatedIds)

    const checkedValues = updatedIds
      .map((checkedId) => checkboxes.find((cb) => cb.id === checkedId)?.value)
      .filter((val): val is React.ReactNode => val !== undefined)

    onCheckboxChange?.({
      id,
      value,
      checked,
      checkedValues,
    })
  }

  return (
    <Flex flexDir={flexDir} gap={gap || (flexDir === 'row' ? '30px' : '16px')}>
      {checkboxes.map((checkbox) => (
        <Checkbox
          key={checkbox.id}
          checked={checkedIds.includes(checkbox.id)}
          disabled={checkbox.disabled}
          label={`${checkbox.id}-${checkbox.label}`} // 고유한 label 생성
          onChange={(checked) =>
            handleCheckboxChange(checkbox.id, checkbox.value, checked)
          }
          variant={variant}
        >
          {checkbox.value}
        </Checkbox>
      ))}
    </Flex>
  )
}
