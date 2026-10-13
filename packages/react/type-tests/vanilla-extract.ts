import '../src/compat/vanilla-extract'

import {
  assignVars,
  createContainer,
  createGlobalTheme,
  createGlobalThemeContract,
  createTheme,
  createThemeContract,
  createVar,
  type CSSVarFunction,
  fallbackVar,
  fontFace,
  type FontFaceRule,
  globalFontFace,
  globalLayer,
  globalStyle,
  keyframes,
  layer,
  type MapLeafNodes,
  type NullableTokens,
  style,
  styleVariants,
  type Tokens,
  type VarDeclaration,
} from '@vanilla-extract/css'

type Assert<T extends true> = T
type Equal<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
    ? true
    : false
type Rejects<Input, Shape> = Input extends Shape ? false : true

// Given nested placeholders, consumer values must become CSS variable strings.
const contract = createThemeContract({
  colors: { bg: null, fg: '' },
  space: null,
})
export const mapped = createGlobalThemeContract(contract, (value, path) => {
  type _ValueContext = Assert<Equal<typeof value, string | null>>
  type _PathContext = Assert<Equal<typeof path, string[]>>
  const leaf: string | null = value
  const segments: string[] = path
  return `${segments.join('-')}-${leaf ?? 'token'}`
})
export const named = createGlobalThemeContract({
  colors: { bg: '--background' },
})
type _Leaf = Assert<Equal<typeof contract.colors.bg, CSSVarFunction>>
type _MappedLeaf = Assert<Equal<typeof mapped.colors.fg, CSSVarFunction>>
type _NamedLeaf = Assert<Equal<typeof named.colors.bg, CSSVarFunction>>
type _Boolean = Assert<Rejects<{ leaf: boolean }, NullableTokens>>
type _Array = Assert<Rejects<{ leaf: string[] }, NullableTokens>>
type _Function = Assert<Rejects<{ leaf: () => string }, NullableTokens>>
type _NullTheme = Assert<Rejects<{ leaf: null }, Tokens>>
type _NumberTheme = Assert<Rejects<{ leaf: number }, Tokens>>

// When creating inferred and existing-contract themes, their return shapes differ.
const parent = layer('themes')
const child = layer({ parent }, 'light')
const global = globalLayer({ parent: globalLayer('app') }, 'themes')
export const inferred = createTheme(
  { colors: { bg: 'white' }, '@layer': child },
  'light',
)
export const existing = createTheme(
  contract,
  {
    colors: { bg: 'white', fg: 'black' },
    space: '8px',
    '@layer': child,
  },
  'existing',
)
export const globalVars = createGlobalTheme(':root', {
  colors: { bg: 'white' },
  '@layer': global,
})
export const globalResult = createGlobalTheme(':root', contract, {
  colors: { bg: 'white', fg: 'black' },
  space: '8px',
  '@layer': global,
})
type _Tuple = Assert<
  Equal<typeof inferred, [string, { colors: { bg: CSSVarFunction } }]>
>
type _Class = Assert<Equal<typeof existing, string>>
type _GlobalVars = Assert<
  Equal<typeof globalVars, { colors: { bg: CSSVarFunction } }>
>
type _Void = Assert<Equal<typeof globalResult, void>>
type _LayerOmitted = Assert<
  Equal<Extract<keyof (typeof inferred)[1], '@layer'>, never>
>

// Then assignments require the full selected subcontract, not a partial tree.
const assigned = assignVars(contract, {
  colors: { bg: 'black', fg: 'white' },
  space: '4px',
})
const subAssigned = assignVars(contract.colors, { bg: 'black', fg: 'white' })
type SubValues = Parameters<typeof assignVars<typeof contract.colors>>[1]
type _SubShape = Assert<Equal<SubValues, { bg: string; fg: string }>>
type _Missing = Assert<Rejects<{ bg: string }, SubValues>>
type _NestedMismatch = Assert<
  Rejects<{ bg: { value: string }; fg: string }, SubValues>
>
type _NumericValue = Assert<Rejects<{ bg: number; fg: string }, SubValues>>
type _FullShape = Assert<
  Equal<
    Parameters<typeof assignVars<typeof contract>>[1],
    MapLeafNodes<typeof contract, string>
  >
>
type _Assignments = Assert<
  Equal<typeof assigned, Record<CSSVarFunction, string>>
>

const variable = createVar(
  { syntax: '<length>', inherits: false, initialValue: '0px' },
  'gap',
)
createVar({ syntax: '*', inherits: true })
createVar({
  syntax: ['<length>', '<percentage>'],
  inherits: true,
  initialValue: '0px',
})
createVar('color')
type _RequiredInitial = Assert<
  Rejects<{ syntax: '<length>'; inherits: false }, VarDeclaration>
>
type _RequiredInherits = Assert<
  Rejects<{ syntax: '*'; initialValue: string }, VarDeclaration>
>
type _InitialString = Assert<
  Rejects<{ syntax: '*'; inherits: true; initialValue: number }, VarDeclaration>
>
type _FallbackArity = Assert<Rejects<[], Parameters<typeof fallbackVar>>>
const base = style({ vars: assigned, background: contract.colors.bg }, 'base')
const composed = style([base, [base], { padding: 4 }], 'composed')
export const variants = styleVariants(
  { small: [base, { padding: 4 }], large: { padding: 8 } },
  'size',
)
const callback = styleVariants(
  { small: 4, large: 8 },
  (value, key) => {
    type _ValueContext = Assert<Equal<typeof value, number>>
    type _KeyContext = Assert<Equal<typeof key, 'small' | 'large'>>
    const size: number = value
    const name: 'small' | 'large' = key
    return [base, [composed], { padding: size, content: name }]
  },
  'callback',
)
type _Variants = Assert<
  Equal<typeof variants, Record<'small' | 'large', string>>
>
type _Callback = Assert<
  Equal<typeof callback, Record<'small' | 'large', string>>
>
type _StyleBoolean = Assert<
  Rejects<{ color: boolean }, Parameters<typeof style>[0]>
>
type _StyleShorthand = Assert<
  Rejects<{ p: number }, Parameters<typeof style>[0]>
>
type _StyleVars = Assert<
  Rejects<{ vars: { color: number } }, Parameters<typeof style>[0]>
>
type _SelectorGlobal = Assert<
  Rejects<
    { selectors: { '&': { color: string } } },
    Parameters<typeof globalStyle>[1]
  >
>
type _FontSrc = Assert<
  Rejects<{ fontWeight: number }, Parameters<typeof fontFace>[0]>
>
type _FontFamily = Assert<
  Equal<Extract<keyof FontFaceRule, 'fontFamily'>, never>
>
globalStyle('body', {
  vars: subAssigned,
  '@media': { print: { color: 'black' } },
})
globalFontFace('App Font', [{ src: 'url(font.woff2)' }])
const font = fontFace([{ src: 'url(font.woff2)', fontWeight: 400 }], 'font')
const animation = keyframes(
  { from: { vars: { [variable]: '0px' } }, to: { opacity: 1 } },
  'fade',
)
const container = createContainer('sidebar')
export const names = [
  font,
  animation,
  container,
  fallbackVar(variable, '1px'),
  callback.small,
]
