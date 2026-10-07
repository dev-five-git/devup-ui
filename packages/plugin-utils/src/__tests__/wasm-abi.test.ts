import { describe, expect, it, mock } from 'bun:test'

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
  wasm.setDebug(state.debug)
  wasm.setPrefix(state.prefix ?? null)
}

function extract(code: string, filename = 'abi.tsx') {
  return wasm.codeExtract(filename, code, PACKAGE, 'df', true, false, false, {})
}

describe('@devup-ui/wasm exports', () => {
  it('exposes exactly the functions the plugins call', () => {
    // default is the CommonJS module object a Node-style import adds
    expect(
      Object.keys(wasm)
        .filter((name) => name !== 'default')
        .sort(),
    ).toEqual([
      'Output',
      'codeExtract',
      'codeExtractWithoutSourceMap',
      'exportCanonicalMap',
      'exportClassMap',
      'exportFileMap',
      'exportSheet',
      'getCss',
      'getDefaultTheme',
      'getPrefix',
      'getThemeInterface',
      'hasDevupUI',
      'importCanonicalMap',
      'importClassMap',
      'importFileMap',
      'importFileRoutes',
      'importSheet',
      'isDebug',
      'registerShorthands',
      'registerTheme',
      'resetStateForTesting',
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
      wasm.hasDevupUI('a.tsx', `import { Box } from '${PACKAGE}'`, PACKAGE),
    ).toBe(true)
    expect(
      wasm.hasDevupUI('a.tsx', `import { Box } from 'other'`, PACKAGE),
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
