import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import {
  afterAll,
  beforeAll,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { DevupUI } from '../plugin'

type CodeExtractResult = ReturnType<typeof wasm.codeExtract>
type RsbuildPlugin = ReturnType<typeof DevupUI>
type RsbuildSetupContext = Parameters<RsbuildPlugin['setup']>[0]

// Two checkouts of one repository — git worktrees, a CI matrix, sibling clones.
// They hold byte-identical sources at identical repository-relative paths and
// differ only in their root.
const checkoutA = join('/repos', 'app', 'worktree-a')
const checkoutB = join('/repos', 'app', 'worktree-b')

const source = `import { Box } from '@devup-ui/react'
const App = () => <Box bg="red" />`

let codeExtractSpy: ReturnType<typeof spyOn>

beforeAll(() => {
  spyOn(fs, 'existsSync').mockReturnValue(true)
  spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  spyOn(wasm, 'getThemeInterface').mockReturnValue('')
  spyOn(wasm, 'getDefaultTheme').mockReturnValue(undefined)
  spyOn(wasm, 'getCss').mockReturnValue('')
  spyOn(wasm, 'setDebug').mockReturnValue(undefined)
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue({
    code: '<div></div>',
    css: '',
    cssFile: 'devup-ui-0.css',
    map: undefined,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
  } as unknown as CodeExtractResult)
})

afterAll(() => {
  mock.restore()
})

/**
 * Runs the code transform for a file inside `checkout` and returns the css dir
 * the plugin handed to the extractor — the value that ends up as the stylesheet
 * import specifier baked into the emitted module.
 */
async function extractedCssDirIn(checkout: string) {
  const transform = mock()
  const plugin = DevupUI({
    distDir: join(checkout, 'df'),
    cssDir: join(checkout, 'df', 'devup-ui'),
  })
  await plugin.setup({
    transform,
    modifyRsbuildConfig: mock(),
  } as unknown as RsbuildSetupContext)

  codeExtractSpy.mockClear()
  await transform.mock.calls[1][1]({
    code: source,
    resourcePath: join(checkout, 'src', 'App.tsx'),
  })
  return codeExtractSpy.mock.calls[0][3] as string
}

describe('checkout isolation', () => {
  // Regression: the emitted stylesheet import must not depend on where the
  // repository happens to be checked out. An absolute specifier makes the
  // transform output differ per checkout for byte-identical input, which breaks
  // every content-addressed or relocated build cache — the same defect that made
  // `bun test` import another worktree's stylesheet.
  it('emits the same, checkout-independent stylesheet import from any checkout', async () => {
    const fromA = await extractedCssDirIn(checkoutA)
    const fromB = await extractedCssDirIn(checkoutB)

    expect(fromA).toBe(fromB)
    expect(fromA).not.toContain(checkoutA)
    expect(fromB).not.toContain(checkoutB)
  })

  it('emits a relative specifier the bundler resolves from the importer', async () => {
    // ./../df/devup-ui, relative to <checkout>/src/App.tsx
    expect(await extractedCssDirIn(checkoutA)).toBe('./../df/devup-ui')
  })
})
