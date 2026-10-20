import { describe, expect, it, mock } from 'bun:test'

const wasm = await import('../../../../bindings/devup-ui-wasm/pkg/index.js')
const pkg = '@devup-ui/react'
const source = `import { Box } from '${pkg}'; import { tone } from './tokens'; export const view = <Box bg={tone} />`

function extract(code: string, file = 'reset.tsx', single = true) {
  return wasm.codeExtract(file, code, pkg, 'df', single, false, false, {})
}

describe('normal WASM full reset', () => {
  it('removes the resolver on a path that needs it and accepts a different resolver', () => {
    const old = mock(() => ({
      path: 'tokens.ts',
      code: "export const tone = 'red'",
    }))
    wasm.setModuleResolver(old)
    expect(extract(source).dependencies).toEqual(['tokens.ts'])
    expect(wasm.getCss(null, false)).toContain('background:red')

    wasm.resetStateForTesting()
    old.mockClear()
    const unresolved = extract(source)
    expect(old).not.toHaveBeenCalled()
    expect(unresolved.dependencies).toEqual([])
    expect(wasm.getCss(null, false)).not.toContain('background:red')
    const fresh = mock(() => ({
      path: 'tokens.ts',
      code: "export const tone = 'blue'",
    }))
    wasm.setModuleResolver(fresh)
    expect(extract(source).dependencies).toEqual(['tokens.ts'])
    expect(fresh).toHaveBeenCalled()
    expect(wasm.getCss(null, false)).toContain('background:blue')
  })

  it('evicts imported stylesheet results before a different source is registered', () => {
    const entry = `import { style } from '${pkg}'; import { card } from './card.css'; export const view = style([card, {width:'12px'}])`
    const supplier = (color: string) => ({
      path: 'card.css.ts',
      code: `import { style } from '${pkg}'; export const card = style({color:'${color}'})`,
    })
    const old = mock(() => supplier('red'))
    wasm.setModuleResolver(old)
    extract(supplier('red').code, 'card.css.ts')
    expect(extract(entry, 'root.css.ts').dependencies).toContain('card.css.ts')
    expect(wasm.getCss(null, false)).toContain('color:red')
    wasm.resetStateForTesting()
    old.mockClear()
    expect(() => extract(entry, 'root.css.ts')).toThrow()
    expect(old).not.toHaveBeenCalled()
    expect(wasm.getCss(null, false)).not.toContain('color:red')
    wasm.setModuleResolver(() => supplier('blue'))
    extract(supplier('blue').code, 'card.css.ts')
    expect(extract(entry, 'root.css.ts').dependencies).toContain('card.css.ts')
    expect(wasm.getCss(null, false)).toContain('color:blue')
    expect(wasm.getCss(null, false)).not.toContain('color:red')
  })

  it('returns reservation accounting to Legacy before the next reservation', () => {
    const red = `import {Box} from '${pkg}'; export const v=<Box color='red'/>`
    const blue = red.replace('red', 'blue')
    wasm.setDebug(false)
    wasm.reserveClassSlots(
      [['reserved.tsx', red, 'df']],
      pkg,
      true,
      false,
      false,
      {},
    )
    extract(blue, 'reserved.tsx')
    expect(wasm.getUnreservedClassSlotCount()).toBe(1)
    for (let round = 0; round < 2; round += 1) {
      wasm.resetStateForTesting()
      wasm.resetStateForTesting()
      expect(wasm.getUnreservedClassSlotCount()).toBe(0)
      expect(wasm.exportClassMap()).toBe('{}')
      extract(blue, 'legacy.tsx')
      expect(wasm.getUnreservedClassSlotCount()).toBe(0)
      wasm.reserveClassSlots(
        [['reserved.tsx', red, 'df']],
        pkg,
        true,
        false,
        false,
        {},
      )
      extract(red, 'reserved.tsx')
      expect(wasm.getUnreservedClassSlotCount()).toBe(0)
      const green = red.replace('red', 'green')
      extract(green, 'reserved.tsx')
      expect(wasm.getUnreservedClassSlotCount()).toBe(1)
      extract(green, 'reserved.tsx')
      expect(wasm.getUnreservedClassSlotCount()).toBe(1)
      const raw = `import {createVar} from '${pkg}'; export const v=createVar()`
      extract(raw, 'virtual.css.ts')
      expect(wasm.getUnreservedClassSlotCount()).toBe(2)
      extract(raw, 'virtual.css.ts')
      expect(wasm.getUnreservedClassSlotCount()).toBe(2)
    }
  })

  it('freezes only newly configured buckets after full reset', () => {
    const code = `import {Box} from '${pkg}'; export const v=<Box color='red'/>`
    wasm.resetStateForTesting()
    wasm.setAtomHoist(2)
    wasm.importCanonicalMap({ 'old.tsx': 'old-bucket.tsx' })
    wasm.importFileRoutes({ 'old-bucket.tsx': [0, 1] })
    extract(code, 'old.tsx', false)
    expect(JSON.parse(wasm.exportSheet()).atom_plan).toEqual(['old-bucket.tsx'])

    wasm.resetStateForTesting()
    wasm.setAtomHoist(2)
    wasm.importCanonicalMap({ 'fresh.tsx': 'fresh-bucket.tsx' })
    wasm.importFileRoutes({ 'fresh-bucket.tsx': [2, 3], 'private.tsx': [2] })
    const shared = extract(code.replace('red', 'blue'), 'fresh.tsx', false)
    const local = extract(code.replace('red', 'green'), 'private.tsx', false)
    expect(JSON.parse(wasm.exportSheet()).atom_plan).toEqual([
      'fresh-bucket.tsx',
    ])
    expect(wasm.getCss(null, false)).toContain('color:blue')
    expect(wasm.getCss(null, false)).not.toContain('color:green')
    expect(shared.css).not.toContain('color:blue')
    expect(local.css).toContain('color:green')
    expect(wasm.getCss(null, false)).not.toContain('color:red')
  })

  it('clears token literals, shorthands and Tailwind definitions before reconfiguration', () => {
    wasm.resetStateForTesting()
    const code = `import {Box} from '${pkg}'; export const v=<Box w='$space' boxShadow='$card' customInset={3} className='paint tab-4 night:paint'/>`
    const baseline = extract(code).code
    wasm.registerShorthands({ customInset: ['left', 'right'] })
    wasm.registerTheme({
      colors: { default: { stale: '#abcdef' } },
      length: { default: { space: ['11px', null, '29px'] } },
      shadow: { default: { card: ['0 3px 5px red', null, '0 5px 8px blue'] } },
      typography: { staleHeading: { fontSize: '37px' } },
      tailwindCss:
        '@theme {--color-*: initial; --color-brand: teal; @keyframes bounce {to {opacity: 0.5}}} @utility paint {color: red; @apply p-4;} @utility tab-* {tab-size: --value(integer);} @custom-variant night (&:where(.night *));',
    })
    const dirty = extract(code)
    expect(dirty.code).not.toBe(baseline)
    expect(wasm.getCss(null, false)).toContain('color:red')
    expect(wasm.getCss(null, false)).toContain('tab-size:4')
    expect(wasm.getCss(null, false)).toContain('left:12px')
    for (let round = 0; round < 2; round += 1) {
      wasm.resetStateForTesting()
      expect(extract(code).code).toBe(baseline)
      const css = wasm.getCss(null, false)
      expect(css).not.toContain('29px')
      expect(css).not.toContain('color:red')
      expect(css).not.toContain('tab-size:4')
      expect(css).not.toContain('left:12px')
      const types = wasm.getThemeInterface(
        pkg,
        'Colors',
        'Typography',
        'Length',
        'Shadows',
        'Theme',
      )
      expect(types).not.toContain('staleHeading')
      expect(types).not.toContain('$stale')
    }
    wasm.registerTheme({ tailwindCss: '@utility paint {color: blue;}' })
    extract(code)
    expect(wasm.getCss(null, false)).toContain('color:blue')
    expect(wasm.getCss(null, false)).not.toContain('color:red')
  })
})
