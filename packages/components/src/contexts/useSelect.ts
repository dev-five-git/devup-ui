'use client'
import { createContext, useContext } from 'react'

type SelectType = 'default' | 'radio' | 'checkbox'
type SelectValue<T extends SelectType> = T extends 'radio' ? string : string[]

export const SelectContext = createContext<{
  open: boolean
  setOpen: (open: boolean) => void
  value: SelectValue<SelectType>
  setValue: (value: string) => void
  type: SelectType
  ref: React.RefObject<HTMLDivElement | null>
} | null>(null)

export const useSelect = () => {
  const context = useContext(SelectContext)
  if (!context) {
    throw new Error('useSelect must be used within a Select')
  }
  return context
}
