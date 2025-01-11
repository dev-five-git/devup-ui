export type Merge<T, U> = Omit<T, Extract<keyof T, keyof U>> & U
