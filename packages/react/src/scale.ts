/**
 * Unit conversion for style values only known at runtime.
 *
 * The build converts a literal number by its property (`p={2}` becomes
 * `padding: 8px`); a value read at runtime is passed through these functions
 * where the element sets it, so it reads as the same literal would. The
 * compiled output imports them from `@devup-ui/react/scale`. They depend on
 * nothing and run no code of the value's own.
 */

/**
 * `v` as the web build reads a literal: a number, or a string holding only
 * one, times `s` in the unit `u`; any other value, `null`, `undefined` and
 * `false` included, as it is
 */
export function __devupScale(v: unknown, s = 4, u = 'px'): unknown {
  return typeof v === 'number' ||
    (typeof v === 'string' && /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(v))
    ? Number(v) * s + u
    : v
}

/**
 * `v` as the React Native build reads a literal: a number, or a string
 * holding only one, times `s`; a `px` length its number, a `rem` or `em`
 * length in pixels of a 16px root; any other value as it is
 */
export function __devupNativeScale(v: unknown, s: number): unknown {
  if (typeof v === 'number') return v * s
  const m =
    typeof v === 'string' && /^\s*(-?(?:\d+\.?\d*|\.\d+))(px|r?em)?\s*$/.exec(v)
  return m ? Number(m[1]) * (m[2] ? (m[2] === 'px' ? 1 : 16) : s) : v
}
