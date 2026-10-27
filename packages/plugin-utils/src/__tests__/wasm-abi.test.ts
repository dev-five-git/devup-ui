import { beforeEach, describe, expect, it, mock } from 'bun:test'

// The compiled module as a plugin loads it: the Rust tests cover the crates, not
// this boundary (every wrapper is excluded from tarpaulin), so the contract the
// plugins rely on is checked here, through the JS bindings.
const wasm = await import('../../../../bindings/devup-ui-wasm/pkg/index.js')

const PACKAGE = '@devup-ui/react'

/** What the module keeps for the process, so a test leaves it as it found it */
function saveState() {
  return {
    sheet: JSON.parse(wasm.exportSheet()),
    classMap: JSON.parse(wasm.exportClassMap()),
    fileMap: JSON.parse(wasm.exportFileMap()),
    canonicalMap: JSON.parse(wasm.exportCanonicalMap()),
    reservation: JSON.parse(wasm.exportReservationState()),
    css: wasm.getCss(null, false),
    debug: wasm.isDebug(),
    prefix: wasm.getPrefix(),
  }
}

function restoreState(state: ReturnType<typeof saveState>) {
  wasm.importSheet(state.sheet)
  wasm.importClassMap(state.classMap)
  wasm.importFileMap(state.fileMap)
  wasm.importCanonicalMap(state.canonicalMap)
  wasm.importReservationState(state.reservation)
  wasm.setDebug(state.debug)
  wasm.setPrefix(state.prefix ?? null)
}

function extract(code: string, filename = 'abi.tsx') {
  return wasm.codeExtract(filename, code, PACKAGE, 'df', true, false, false, {})
}

describe('@devup-ui/wasm exports', () => {
  it('exposes exactly the compiled engine functions', () => {
    // default is the CommonJS module object a Node-style import adds
    expect(
      Object.keys(wasm)
        .filter((name) => name !== 'default')
        .sort(),
    ).toEqual([
      'Output',
      'codeExtract',
      'codeExtractWithoutSourceMap',
      'compiledGuardFacts',
      'exportCanonicalMap',
      'exportClassMap',
      'exportFileMap',
      'exportReservationState',
      'exportSheet',
      'getCss',
      'getDefaultTheme',
      'getPrefix',
      'getSuppressedReservationRowCount',
      'getThemeInterface',
      'getUnreservedClassSlotCount',
      'hasDevupUI',
      'importCanonicalMap',
      'importCanonicalMapJson',
      'importClassMap',
      'importClassMapJson',
      'importFileMap',
      'importFileMapJson',
      'importFileRoutes',
      'importReservationState',
      'importReservationStateJson',
      'importSheet',
      'importSheetJson',
      'install_panic_hook',
      'isDebug',
      'registerShorthands',
      'registerTheme',
      'reserveClassSlots',
      'resetBuildState',
      'resetStateForTesting',
      'seedFileMap',
      'setAtomHoist',
      'setDebug',
      'setModuleResolver',
      'setPrefix',
    ])
  })
})

describe('@devup-ui/wasm extraction', () => {
  it('compiles a component and hands back the code, the css and what it read', () => {
    wasm.setDebug(false)
    const output = extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box p={4} bg="red" />`,
    )

    expect(output.code).toContain('className=')
    expect(output.code).not.toContain('<Box')
    expect(output.css).toContain('padding')
    expect(output.dependencies).toEqual([])
    expect(output.map).toBeDefined()
    expect(typeof output.updatedBaseStyle).toBe('boolean')
    expect(wasm.getCss(null, false)).toContain('background')
  })

  it('compiles through the variant that skips the source map step', () => {
    const output = wasm.codeExtractWithoutSourceMap(
      'abi.tsx',
      `import { Box } from '${PACKAGE}'\nexport const a = <Box m={2} />`,
      PACKAGE,
      'df',
      true,
      false,
      false,
      {},
    )

    expect(output.code).toContain('className=')
    expect(output.css).toContain('margin')
  })

  it('tells a file that uses Devup UI from one that does not', () => {
    expect(
      wasm.hasDevupUI('a.tsx', `import { Box } from '${PACKAGE}'`, PACKAGE, {}),
    ).toBe(true)
    expect(
      wasm.hasDevupUI('a.tsx', `import { Box } from 'other'`, PACKAGE, {}),
    ).toBe(false)
  })

  it('throws the build error with its location and what the build needs', () => {
    expect(() =>
      extract(
        `import { css } from '${PACKAGE}'\nexport const a = css({ color: window.name })`,
      ),
    ).toThrow(
      /abi\.tsx:2:\d+: `css\(\)` cannot use `window\.name` at build time: its values must be literals/,
    )
  })

  it('leaves a file that does not use Devup UI as it is', () => {
    const code = 'export const a = 1'
    expect(extract(code).code).toBe(code)
  })

  it('applies the class name prefix, in debug mode too', () => {
    wasm.setDebug(false)
    wasm.setPrefix('du-')
    expect(wasm.getPrefix()).toBe('du-')
    const output = extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box p={1} />`,
    )

    expect(output.code).toMatch(/className="du-/)
    wasm.setPrefix(null)
    expect(wasm.getPrefix()).toBeUndefined()
  })
})

describe('@devup-ui/wasm theme', () => {
  it('registers a theme and describes it as types', () => {
    wasm.registerTheme({
      colors: { default: { primary: '#000' }, dark: { primary: '#fff' } },
      typography: { heading: { fontSize: '24px' } },
    })
    const types = wasm.getThemeInterface(
      PACKAGE,
      'CustomColors',
      'DevupThemeTypography',
      'CustomLength',
      'CustomShadows',
      'DevupTheme',
    )

    expect(types).toContain('$primary')
    expect(types).toContain('heading')
    expect(types).toContain('dark')
    expect(wasm.getDefaultTheme()).toBe('default')
  })

  it('reads a theme token in a style prop', () => {
    wasm.registerTheme({ colors: { default: { primary: '#0070f3' } } })
    wasm.setDebug(false)
    const output = extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box color="$primary" />`,
    )

    expect(output.css).toContain('var(--primary)')
  })
})

describe('@devup-ui/wasm state', () => {
  it('resets every compiler-state group through the real WASM ABI', () => {
    const source = `import { Box } from '${PACKAGE}'; export const view = <Box p={3} w='$gutter' />`
    wasm.resetStateForTesting()
    const emptyCss = wasm.getCss(null, false)
    const baseline = extract(source, 'fresh.tsx')
    const baselineCode = baseline.code
    const baselineCss = wasm.getCss(null, false)
    const resolver = mock(() => undefined)
    wasm.setPrefix('dirty-')
    wasm.setDebug(true)
    wasm.setAtomHoist(2)
    wasm.importCanonicalMap({ 'fresh.tsx': 'previous.tsx' })
    wasm.importFileRoutes({ 'previous.tsx': [1, 2] })
    wasm.registerShorthands({ customInset: ['left', 'right'] })
    wasm.registerTheme({
      colors: { default: { stale: '#abcdef' } },
      typography: {
        staleHeading: [{ fontSize: '12px' }, { fontSize: '18px' }],
      },
      length: { default: { gutter: ['5px', '9px'] } },
      shadow: { default: { staleShadow: ['0 1px 2px red', '0 2px 4px blue'] } },
    })
    wasm.setModuleResolver(resolver)
    extract(
      `import { Box } from '${PACKAGE}'; export const dirty = <Box customInset={4} typography='staleHeading' />`,
      'dirty.tsx',
    )

    wasm.resetStateForTesting()

    expect(wasm.getPrefix()).toBeUndefined()
    expect(wasm.isDebug()).toBe(false)
    expect(wasm.getDefaultTheme()).toBeUndefined()
    expect(JSON.parse(wasm.exportClassMap())).toEqual({})
    expect(JSON.parse(wasm.exportFileMap())).toEqual({})
    expect(JSON.parse(wasm.exportCanonicalMap())).toEqual({})
    expect(wasm.getCss(null, false)).toBe(emptyCss)
    resolver.mockClear()
    const fresh = extract(source, 'fresh.tsx')
    expect(fresh.code).toBe(baselineCode)
    expect(wasm.getCss(null, false)).toBe(baselineCss)
    expect(resolver).not.toHaveBeenCalled()
  })

  it('starts each test from the preload baseline after previous mutations', () => {
    expect(wasm.getPrefix()).toBeUndefined()
    expect(wasm.isDebug()).toBe(true)
    expect(JSON.parse(wasm.exportSheet()).css).toEqual({})
    expect(JSON.parse(wasm.exportCanonicalMap())).toEqual({})
  })

  it('seeds deterministic file numbers when paths are unsorted and duplicated', () => {
    wasm.resetBuildState()

    wasm.seedFileMap(['src/c.tsx', 'src/b.tsx', 'src/a.tsx', 'src/b.tsx'])

    expect(wasm.exportFileMap()).toBe(
      '{"src/a.tsx":0,"src/b.tsx":1,"src/c.tsx":2}',
    )
  })

  it('clears maps, styles and prefix when resetting a populated build', () => {
    wasm.resetBuildState()
    const emptyCss = wasm.getCss(null, false)
    const emptySheet = wasm.exportSheet()
    wasm.setDebug(false)
    wasm.setPrefix('du-')
    wasm.seedFileMap(['abi.tsx'])
    wasm.importCanonicalMap({ 'child.tsx': 'abi.tsx' })
    const output = extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box p={4} bg="red" />`,
    )
    expect(output.code).toMatch(/className="du-/)
    expect(wasm.exportClassMap()).not.toBe('{}')
    expect(wasm.exportFileMap()).toBe('{"abi.tsx":0}')
    expect(wasm.exportCanonicalMap()).toBe('{"child.tsx":"abi.tsx"}')
    expect(wasm.getCss(null, false)).toContain('background:red')

    wasm.resetBuildState()

    expect(wasm.exportClassMap()).toBe('{}')
    expect(wasm.exportFileMap()).toBe('{}')
    expect(wasm.exportCanonicalMap()).toBe('{}')
    expect(wasm.exportSheet()).toBe(emptySheet)
    expect(wasm.getCss(null, false)).toBe(emptyCss)
    expect(wasm.getPrefix()).toBeUndefined()
  })

  it('puts the process back as a test found it', () => {
    const before = saveState()
    wasm.setDebug(!before.debug)
    wasm.setPrefix('x-')
    extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box p={9} mt={7} />`,
    )
    expect(wasm.getCss(null, false)).not.toBe(before.css)

    restoreState(before)

    expect(wasm.isDebug()).toBe(before.debug)
    expect(wasm.getPrefix()).toBe(before.prefix)
    expect(wasm.getCss(null, false)).toBe(before.css)
    expect(JSON.parse(wasm.exportClassMap())).toEqual(before.classMap)
  })

  it('carries the collected styles to another module state through the maps', () => {
    wasm.setDebug(false)
    extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box p={5} />`,
      'carried.tsx',
    )
    const carried = saveState()

    wasm.importSheet(JSON.parse('{}'))
    restoreState(carried)

    expect(wasm.getCss(null, false)).toBe(carried.css)
  })
})

describe('@devup-ui/wasm reservation', () => {
  beforeEach(() => {
    wasm.resetBuildState()
    wasm.setDebug(false)
  })

  it('reserves class and file slots without publishing scratch styles', () => {
    wasm.seedFileMap(['abi.tsx'])
    extract(`import { Box } from '${PACKAGE}'\nexport const a = <Box p={4} />`)
    const before = saveState()
    const source = `import { Box } from '${PACKAGE}'\nexport const a = <Box color="red" gap={value} />`

    wasm.reserveClassSlots(
      [['reserved.tsx', source, 'df']],
      PACKAGE,
      true,
      false,
      false,
      {},
    )

    expect(JSON.parse(wasm.exportSheet())).toEqual(before.sheet)
    expect(wasm.getCss(null, false)).toBe(before.css)
    expect(JSON.parse(wasm.exportClassMap())).not.toEqual(before.classMap)
    expect(JSON.parse(wasm.exportFileMap())).toEqual({
      'abi.tsx': 0,
      'reserved.tsx': 1,
    })
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)
    const slots = wasm.exportClassMap()
    const output = extract(source, 'reserved.tsx')
    expect(output.code).toContain('className=')
    expect(output.css).toContain('color:red')
    expect(output.css).toContain('gap:var(--')
    expect(wasm.exportClassMap()).toBe(slots)
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)
  })

  it('does not count live allocations before the first reservation', () => {
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)

    extract(
      `import { Box } from '${PACKAGE}'\nexport const a = <Box color="red" />`,
    )
    const output = extract(
      `import { createVar } from '${PACKAGE}'\nexport const variable = createVar()`,
      'legacy.css.ts',
    )

    expect(output.code).toContain('var(--')
    expect(output.code).not.toContain('createVar(')
    expect(wasm.exportClassMap()).not.toBe('{}')
    expect(wasm.exportFileMap()).not.toBe('{}')
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)
  })

  it('counts new live class and file slots once across later reservations', () => {
    const red = `import { Box } from '${PACKAGE}'\nexport const a = <Box color="red" />`
    const blue = `import { Box } from '${PACKAGE}'\nexport const a = <Box color="blue" />`
    wasm.reserveClassSlots(
      [['reserved.tsx', red, 'df']],
      PACKAGE,
      true,
      false,
      false,
      {},
    )
    extract(red, 'reserved.tsx')
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)

    const first = extract(blue, 'reserved.tsx')

    expect(wasm.getUnreservedClassSlotCount()).toBe(1)
    expect(extract(blue, 'reserved.tsx').code).toBe(first.code)
    expect(wasm.getUnreservedClassSlotCount()).toBe(1)
    const raw = `import { createVar } from '${PACKAGE}'\nexport const variable = createVar()`
    const variable = extract(raw, 'virtual.css.ts')
    expect(variable.code).toContain('var(--')
    expect(variable.code).not.toContain('createVar(')
    expect(wasm.getUnreservedClassSlotCount()).toBe(2)
    expect(extract(raw, 'virtual.css.ts').code).toBe(variable.code)
    expect(wasm.getUnreservedClassSlotCount()).toBe(2)
    wasm.reserveClassSlots(
      [['watch.tsx', red, 'df']],
      PACKAGE,
      true,
      false,
      false,
      {},
    )
    extract(red, 'watch.tsx')
    expect(wasm.getUnreservedClassSlotCount()).toBe(2)
    wasm.resetBuildState()
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)
  })

  it('restores the live accounting phase and prior misses across owner resets', () => {
    // Given
    const source = (color: string) =>
      `import { Box } from '${PACKAGE}'; export const a = <Box color="${color}" />`
    wasm.reserveClassSlots(
      [['owner.tsx', source('red'), 'df']],
      PACKAGE,
      true,
      false,
      false,
      {},
    )
    extract(source('blue'), 'owner.tsx').free()
    expect(wasm.getUnreservedClassSlotCount()).toBe(1)
    const owner = saveState()
    wasm.resetBuildState()
    extract(source('green'), 'sibling.tsx').free()
    expect(wasm.getUnreservedClassSlotCount()).toBe(0)
    // When
    restoreState(owner)
    extract(source('yellow'), 'owner.tsx').free()
    // Then
    expect(wasm.getUnreservedClassSlotCount()).toBe(2)
    const before = wasm.exportReservationState()
    expect(() => wasm.importReservationState(null)).toThrow()
    expect(wasm.exportReservationState()).toBe(before)
  })

  it.each([false, true])(
    'reserves compiler output under its actual TS filename (singleCss %s)',
    (singleCss) => {
      // Given
      const code = `import {Box} from '${PACKAGE}'; export const view = <Box color='red' gap={value}/>;`
      const before = wasm.getCss(null, false)
      // When
      wasm.reserveClassSlots(
        [['page.ts', code, 'df', 'compiled-mdx']],
        PACKAGE,
        singleCss,
        false,
        false,
        {},
      )
      const classes = wasm.exportClassMap()
      const output = wasm.codeExtract(
        'page.ts',
        code,
        PACKAGE,
        'df',
        singleCss,
        false,
        false,
        {},
        'compiled-mdx',
      )
      // Then
      try {
        expect(before).not.toContain('color:red')
        expect(output.code).not.toContain('<Box')
        expect(output.css).toContain('color:red')
        expect(wasm.exportClassMap()).toBe(classes)
        expect(wasm.getUnreservedClassSlotCount()).toBe(0)
      } finally {
        output.free()
      }
    },
  )

  it('rejects an invalid row grammar before any allocator or stylesheet mutation', () => {
    // Given
    const before = saveState()
    const code = `import {Box} from '${PACKAGE}'; export const view = <Box color='red'/>;`
    // When / Then
    expect(() =>
      wasm.reserveClassSlots(
        [
          ['valid.tsx', code, 'df'],
          ['invalid.mdown', code, 'df', 'tsx'],
        ],
        PACKAGE,
        true,
        false,
        false,
        {},
      ),
    ).toThrow(/invalid\.mdown:1:1: source type/)
    expect(saveState()).toEqual(before)
  })
})
