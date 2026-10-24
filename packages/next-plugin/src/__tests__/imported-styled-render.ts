import assert from 'node:assert/strict'

import * as wasm from '@devup-ui/wasm'
import { expect } from 'bun:test'

const moduleUrl = (code: string) =>
  `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`

const invocationModule = moduleUrl(`
export const forwardRef = render => ({render});
export const jsx = (target, props) => ({target, props});
export const jsxs = jsx;
export const jsxDEV = jsx;
export const render = element => typeof element.target === 'string' ? element
  : render(typeof element.target === 'function' ? (0,element.target)(element.props)
    : element.target.render(element.props, element.props.ref));
`)

function record(value: unknown): Record<string, unknown> {
  assert(typeof value === 'object' && value !== null && !Array.isArray(value))
  return Object.fromEntries(Object.entries(value))
}

export async function assertImportedStyledRendering(
  codes: Readonly<Record<'app' | 'base' | 'card', string>>,
  css: string,
) {
  const transpiler = new Bun.Transpiler({
    loader: 'tsx',
    target: 'browser',
    tsconfig: {
      compilerOptions: { jsx: 'react', jsxFactory: '__testJsx' },
    },
  })
  const emit = (name: keyof typeof codes | 'retarget'): string => {
    const emitted =
      name === 'retarget'
        ? wasm.codeExtractWithoutSourceMap(
            'src/Retargeted.tsx',
            "import styled from '@emotion/styled'\nimport { Dyn } from '@/Base'\nexport const Wrapped = styled(Dyn)``\nexport const Retargeted = Wrapped.withComponent('a')\n",
            '@devup-ui/react',
            '@devup-ui/react',
            true,
            false,
            false,
            { '@emotion/react': null, '@emotion/styled': 'styled' },
          ).code
        : codes[name]
    const source = emitted.replace(/^import ["'][^"']+\.css["'];?\s*$/gm, '')
    const code = transpiler
      .transformSync(source)
      .replace(
        /(from\s*|import\s*)["']([^"']+)["']/g,
        (_, prefix, specifier) => {
          switch (specifier) {
            case 'react':
            case 'react/jsx-runtime':
            case 'react/jsx-dev-runtime':
              return `${prefix}${JSON.stringify(invocationModule)}`
            case '@/Base':
              return `${prefix}${JSON.stringify(emit('base'))}`
            case '@acme/ui':
              return `${prefix}${JSON.stringify(emit('card'))}`
            default:
              throw new SyntaxError(`Unexpected emitted import ${specifier}`)
          }
        },
      )
    return moduleUrl(
      `import {jsx as __testJsx} from ${JSON.stringify(invocationModule)};\n${code}`,
    )
  }
  // Given: complete original app/base/card emissions, linked at their original aliases.
  const observer = moduleUrl(`
import {A,B,C,W} from ${JSON.stringify(emit('app'))};
import {Base,Dyn} from ${JSON.stringify(emit('base'))};
import {Wrapped,Retargeted} from ${JSON.stringify(emit('retarget'))};
import {jsx,render} from ${JSON.stringify(invocationModule)};
const props={ref:7,children:'child',id:'kept',className:'caller',style:{opacity:0.5}};
export const observed=JSON.stringify([
  render(jsx(A,props)),render(jsx(B,{...props,big:false})),
  render(jsx(B,{...props,big:true})),render(jsx(C,props)),render(jsx(W,props)),
  render(jsx(Base,props)),render(jsx(Dyn,{...props,big:false})),
  render(jsx(Dyn,{...props,big:true})),render(jsx(Wrapped,{...props,big:true})),
  render(jsx(Retargeted,{...props,big:true}))
]);`)
  // When: Bun evaluates the actual whole modules and invokes their concrete renderers.
  const loaded = record(await import(observer))
  assert(typeof loaded.observed === 'string')
  const parsed: unknown = JSON.parse(loaded.observed)
  assert(Array.isArray(parsed))
  const elements: readonly unknown[] = parsed
  const rules = [...css.matchAll(/\.([\w-]+)\{([^{}]*)\}/g)]
  const actual = elements.map((value) => {
    const element = record(value)
    const { className, style: inline, ...props } = record(element.props)
    assert(typeof className === 'string')
    const tokens = className.split(/\s+/)
    const style = record(inline)
    const variables = new Set<string>()
    const selected = rules
      .filter((rule) => tokens.includes(rule[1] ?? ''))
      .flatMap((rule) => (rule[2] ?? '').split(';').filter(Boolean))
      .map((declaration) =>
        declaration.replace(/var\((--[\w-]+)\)/g, (_, variable: string) => {
          variables.add(variable)
          const value = style[variable]
          assert(typeof value === 'string' || typeof value === 'number')
          return String(value)
        }),
      )
      .sort()
    expect(Object.keys(style).sort()).toEqual(['opacity', ...variables].sort())
    expect(style.opacity).toBe(0.5)
    return {
      target: element.target,
      props,
      caller: tokens.filter((token) => token === 'caller').length,
      selected,
    }
  })
  // Then: both origin callback states and the other original exported components stay exact.
  const props = { ref: 7, children: 'child', id: 'kept' }
  expect(actual).toEqual([
    {
      target: 'div',
      props: { ...props, role: 'note' },
      caller: 1,
      selected: ['color:blue', 'padding:4px'],
    },
    {
      target: 'button',
      props,
      caller: 1,
      selected: ['font-size:10px', 'margin:0'],
    },
    {
      target: 'button',
      props,
      caller: 1,
      selected: ['font-size:20px', 'margin:0'],
    },
    { target: 'section', props, caller: 1, selected: ['border:17px dotted'] },
    {
      target: 'section',
      props,
      caller: 1,
      selected: ['color:red', 'padding:4px'],
    },
    {
      target: 'div',
      props,
      caller: 1,
      selected: ['color:red', 'padding:4px'],
    },
    { target: 'button', props, caller: 1, selected: ['font-size:10px'] },
    { target: 'button', props, caller: 1, selected: ['font-size:20px'] },
    { target: 'button', props, caller: 1, selected: ['font-size:20px'] },
    { target: 'a', props, caller: 1, selected: ['font-size:20px'] },
  ])
}
