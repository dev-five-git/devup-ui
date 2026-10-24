/* eslint-disable @typescript-eslint/no-unused-expressions */
import { css, globalCss, keyframes } from '../src'

declare const on: boolean
declare const name: string | undefined
const base = css({ p: 1 })

// FN-01: `css()` composes classes, style objects, arrays and falsy parts.
css('external-class', base, { p: 3 })
css(base, on && { m: 1 }, on ? 'a' : null, undefined, false)
css(name, on ? { p: 1 } : 'b')
css([base, { p: 1 }], [[{ m: 1 }]])
css(...[{ p: 1 }, 'a'])
css()

// @ts-expect-error a number is neither a style object nor a class
css(1)
// @ts-expect-error a function is read at runtime
css(() => ({ p: 1 }))
// @ts-expect-error a single object with an unknown key
css({ notAProperty: 1 })
// @ts-expect-error unknown keys stay rejected next to other parts
css(base, { notAProperty: 1 })

// FN-02: interpolations the build evaluates.
const unit = 4
const text = 'top: 0;'
css`
  padding: ${unit * 4}px;
  color: ${'red'};
  margin: ${base};
  ${on && text}
`
keyframes`from { opacity: ${0}; } to { opacity: ${1}; }`
globalCss`body { margin: ${0}px; }`

// The rejected forms go through aliases, which keep the formatter off their text.
const cssTag = css
const keyframesTag = keyframes
const globalCssTag = globalCss
const runtimeValue = () => 'red'
const rules = { a: 1 }

// @ts-expect-error a function is evaluated at runtime
cssTag`color: ${runtimeValue};`
// @ts-expect-error an object is not CSS text
keyframesTag`from { opacity: ${rules}; }`
// @ts-expect-error a function is evaluated at runtime
globalCssTag`body { color: ${runtimeValue}; }`
