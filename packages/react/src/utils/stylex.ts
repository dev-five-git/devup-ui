type StyleValue = string | number | null | undefined

type StyleProperties = Record<string, StyleValue | Record<string, StyleValue>>

interface StyleXTypes {
  angle<T extends string | number>(value: T): T
  color<T extends string>(value: T): T
  image<T extends string>(value: T): T
  integer<T extends number>(value: T): T
  length<T extends string | number>(value: T): T
  lengthPercentage<T extends string | number>(value: T): T
  number<T extends number>(value: T): T
  percentage<T extends string | number>(value: T): T
  resolution<T extends string>(value: T): T
  time<T extends string>(value: T): T
  transformFunction<T extends string>(value: T): T
  transformList<T extends string>(value: T): T
  url<T extends string>(value: T): T
}

export function create<S extends Record<string, StyleProperties>>(
  _styles: S,
): { readonly [K in keyof S]: S[K] } {
  throw new Error('Cannot run on the runtime')
}

export function props(
  ..._styles: ReadonlyArray<StyleProperties | false | null | undefined>
): { className?: string; style?: Record<string, string> } {
  throw new Error('Cannot run on the runtime')
}

export function keyframes(_frames: Record<string, StyleProperties>): string {
  throw new Error('Cannot run on the runtime')
}

export function firstThatWorks<T extends StyleValue>(..._values: T[]): T {
  throw new Error('Cannot run on the runtime')
}

export function include<S extends StyleProperties>(_style: S): S {
  throw new Error('Cannot run on the runtime')
}

export function defineVars<V extends Record<string, StyleValue>>(
  _vars: V,
): { readonly [K in keyof V]: string } {
  throw new Error('Cannot run on the runtime')
}

export function createTheme<V extends Record<string, string>>(
  _vars: V,

  _overrides: { readonly [K in keyof V]: StyleValue },
): Record<string, StyleValue> {
  throw new Error('Cannot run on the runtime')
}

export const types: StyleXTypes = new Proxy({} as StyleXTypes, {
  get() {
    return () => {
      throw new Error('Cannot run on the runtime')
    }
  },
})
