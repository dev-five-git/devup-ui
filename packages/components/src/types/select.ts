export type SelectType = 'default' | 'radio' | 'checkbox'
export type SelectValue<T extends SelectType> = T extends 'radio'
  ? string
  : string[]
