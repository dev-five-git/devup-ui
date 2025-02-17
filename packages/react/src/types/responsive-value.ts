type Value<T> = T | null | undefined | false
export type ResponsiveValue<T> = 0 extends T
  ? Value<number | T> | Value<number | T>[]
  : Value<T> | Value<T>[]
