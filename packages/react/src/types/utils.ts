export type Merge<T, U> = Omit<T, Extract<keyof T, keyof U>> & U

export type Conditional<T> = keyof T extends undefined ? string : keyof T
