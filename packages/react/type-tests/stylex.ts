import stylex, {
  create as namedCreate,
  defineVars as namedDefineVars,
  props as namedProps,
  type StyleXCompiledStyle,
  type StyleXDynamicDeclarations,
  type StyleXFlatDeclarations,
  type StyleXRawStyles,
  type StyleXStyles,
  type StyleXVarGroup,
} from '@stylexjs/stylex'

import * as direct from '../src/utils/stylex'

type Assert<T extends true> = T
type Equal<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
    ? true
    : false
type Accepts<A, B> = [A] extends [B] ? true : false
type Rejects<A, B> = Accepts<A, B> extends false ? true : false
type InvalidVariableCondition = {
  readonly default: 'red'
  readonly ':hover': 'blue'
}
type InvalidVariableGroup = { readonly group: { readonly color: 'red' } }
type OfficialTransitionOptions = { readonly new: { readonly opacity: 0 } }
type ContractInput = Parameters<typeof direct.createThemeContract>[0]
type ConstantsInput = Parameters<typeof direct.defineConsts>[0]
type ExtraThemeInput = Parameters<
  typeof direct.createTheme<'text', { missing: 'red' }>
>[1]
type InvalidConditionInput = Parameters<
  typeof direct.defineVars<{ text: InvalidVariableCondition }>
>[0]
type InvalidGroupInput = Parameters<
  typeof direct.defineVars<InvalidVariableGroup>
>[0]
type OfficialTransitionInput = Parameters<
  typeof direct.viewTransitionClass<OfficialTransitionOptions>
>[0]

const colors = namedDefineVars({
  text: { default: 'black', '@media print': 'gray' },
  accent: stylex.types.color({
    default: 'blue',
    '@supports (color: oklch(0 0 0))': {
      default: 'oklch(0.6 0.2 250)',
      '@container sidebar (min-width: 400px)': 'red',
    },
  }),
  blank: null,
})
const contract = stylex.createThemeContract({
  primary: null,
  size: 'placeholder',
})
const partialTheme = stylex.createTheme(colors, { text: 'navy' })
const conditionTheme = stylex.createTheme(contract, {
  primary: { default: 'red', '@media print': null },
})
const emptyTheme = stylex.createTheme(colors, {})
const _consts = stylex.defineConsts({
  gap: 8,
  flag: true,
  color: 'red',
  zero: -0,
})
const animation = stylex.keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
const fallback = stylex.firstThatWorks(4, 'auto')
const _emptyFallback = stylex.firstThatWorks()
const base = namedCreate({
  root: {
    color: colors.text,
    width: { default: fallback, ':hover': { '@media print': '50%' } },
    ':focus': { outlineColor: 'red' },
    animationName: animation,
    '--local': 1,
  },
  empty: null,
  dynamic: (height: number, color: string | null) => ({ height, color }),
})
const included = stylex.include(base.root)
const styles = stylex.create({ root: { ...included, color: 'blue' } })
const _inherited = stylex.create({ root: { ...included, opacity: 1 } })
declare const enabled: boolean
declare const external: StyleXStyles
declare const externalClass: string
const composition = [
  base.root,
  [enabled && styles.root, null, undefined, false],
] as const
export const reactProps = namedProps(
  partialTheme,
  conditionTheme,
  emptyTheme,
  base.empty,
  base.dynamic(20, null),
  composition,
  external,
  'external',
  [externalClass, ['nested-external']],
)
export const domAttrs = stylex.attrs(composition, base.dynamic(10, 'red'))
export const tryName = stylex.positionTry({ top: 0, insetBlockEnd: 'auto' })
export const transitionName = stylex.viewTransitionClass({
  animationDuration: '1s',
})
export const scalarWrappers = [
  stylex.types.angle('1deg'),
  stylex.types.color('red'),
  stylex.types.image('url(image.png)'),
  stylex.types.integer(1),
  stylex.types.length(2),
  stylex.types.lengthPercentage('50%'),
  stylex.types.number(0.5),
  stylex.types.percentage('20%'),
  stylex.types.resolution('2dppx'),
  stylex.types.time('1s'),
  stylex.types.transformFunction('scale(2)'),
  stylex.types.transformList('scale(2) translateX(1px)'),
  stylex.types.url('url(image.png)'),
] as const
export const conditionalWrappers = [
  stylex.types.angle({ default: 1, '@media print': '0deg' }),
  stylex.types.color({ default: 'red', '@media print': null }),
  stylex.types.image({ default: 'none', '@media print': 'url(print.png)' }),
  stylex.types.integer({ default: 1, '@media print': 2 }),
  stylex.types.length({ default: 2, '@media print': '1px' }),
  stylex.types.lengthPercentage({ default: 0, '@media print': '50%' }),
  stylex.types.number({ default: 1, '@media print': 0 }),
  stylex.types.percentage({ default: 0, '@media print': '20%' }),
  stylex.types.resolution({ default: '1dppx', '@media print': '2dppx' }),
  stylex.types.time({ default: '1s', '@media print': '0s' }),
  stylex.types.transformFunction({ default: 'scale(2)', '@media print': null }),
  stylex.types.transformList({ default: 'none', '@media print': null }),
  stylex.types.url({ default: 'url(image.png)', '@media print': null }),
] as const

export type StyleXContractChecks = [
  Assert<Equal<typeof _consts.gap, '8'>>,
  Assert<Equal<typeof _consts.flag, 'true'>>,
  Assert<Equal<typeof _consts.color, 'red'>>,
  Assert<Equal<typeof _consts.zero, '0'>>,
  Assert<Equal<typeof fallback, 4 | 'auto'>>,
  Assert<Equal<typeof _emptyFallback, never>>,
  Assert<
    Equal<
      Parameters<typeof base.dynamic>,
      [height: number, color: string | null]
    >
  >,
  Assert<Accepts<ReturnType<typeof base.dynamic>, StyleXCompiledStyle>>,
  Assert<Rejects<[number], Parameters<typeof base.dynamic>>>,
  Assert<Rejects<[string, number], Parameters<typeof base.dynamic>>>,
  Assert<Rejects<[number, string, number], Parameters<typeof base.dynamic>>>,
  Assert<Accepts<typeof base.root, StyleXStyles>>,
  Assert<Accepts<typeof base.root, StyleXStyles<{ color?: string }>>>,
  Assert<Rejects<typeof base.root, StyleXStyles<{ color?: number }>>>,
  Assert<Rejects<{ color: 'red' }, Parameters<typeof direct.props>[number]>>,
  Assert<Rejects<'external', StyleXStyles>>,
  Assert<Rejects<typeof base, StyleXStyles>>,
  Assert<Rejects<typeof base.dynamic, StyleXStyles>>,
  Assert<
    Accepts<readonly [typeof base.root, readonly [false, null]], StyleXStyles>
  >,
  Assert<Rejects<{ color: 'red' }, Parameters<typeof direct.include>[0]>>,
  Assert<Rejects<typeof included, StyleXStyles>>,
  Assert<Accepts<typeof _inherited.root, StyleXStyles<{ color?: string }>>>,
  Assert<Rejects<typeof _inherited.root, StyleXStyles<{ color?: number }>>>,
  Assert<Rejects<{ primary: string }, StyleXVarGroup<'primary'>>>,
  Assert<Equal<typeof colors.text, string>>,
  Assert<Equal<typeof contract.size, string>>,
  Assert<Rejects<{ primary: { nested: null } }, ContractInput>>,
  Assert<Rejects<{ primary: number }, ContractInput>>,
  Assert<Rejects<{ primary: undefined }, ContractInput>>,
  Assert<Rejects<{ primary: boolean }, ContractInput>>,
  Assert<Rejects<{ missing: 'red' }, ExtraThemeInput>>,
  Assert<Rejects<{ text: InvalidVariableCondition }, InvalidConditionInput>>,
  Assert<Rejects<InvalidVariableGroup, InvalidGroupInput>>,
  Assert<Rejects<{ color: ['red', 'blue'] }, StyleXRawStyles>>,
  Assert<Rejects<{ color: { default: 'red' } }, StyleXDynamicDeclarations>>,
  Assert<Rejects<{ top: { default: 0 } }, StyleXFlatDeclarations>>,
  Assert<Rejects<OfficialTransitionOptions, OfficialTransitionInput>>,
  Assert<Rejects<{ x: null }, ConstantsInput>>,
  Assert<Rejects<{ x: { default: 'red' } }, ConstantsInput>>,
  Assert<Rejects<number, Parameters<typeof direct.types.color>[0]>>,
  Assert<Rejects<string, Parameters<typeof direct.types.integer>[0]>>,
  Assert<Equal<(typeof scalarWrappers)[3], 1>>,
  Assert<Equal<(typeof conditionalWrappers)[3]['default'], 1>>,
  Assert<
    Equal<
      keyof typeof direct.types,
      | 'angle'
      | 'color'
      | 'image'
      | 'integer'
      | 'length'
      | 'lengthPercentage'
      | 'number'
      | 'percentage'
      | 'resolution'
      | 'time'
      | 'transformFunction'
      | 'transformList'
      | 'url'
    >
  >,
  Assert<Equal<Extract<'when' | 'env', keyof typeof direct>, never>>,
  Assert<Equal<Extract<'when' | 'env', keyof typeof stylex>, never>>,
  Assert<Equal<typeof direct.create, typeof namedCreate>>,
  Assert<Equal<keyof typeof reactProps, 'className' | 'style'>>,
  Assert<Equal<keyof typeof domAttrs, 'class' | 'style'>>,
  Assert<Rejects<string, NonNullable<typeof domAttrs.style>>>,
  Assert<
    Accepts<{ '--n': 1; '--empty': null }, NonNullable<typeof reactProps.style>>
  >,
]
