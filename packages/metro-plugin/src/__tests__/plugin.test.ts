import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  spyOn,
} from 'bun:test'

import {
  REANIMATED_PACKAGE,
  TRANSFORMER_PATH,
  UpstreamTransformerNotFoundError,
  withDevupUI,
} from '../plugin'
import { readSettings, SETTINGS_ENV } from '../settings'
import { NATIVE_TYPES } from '../types'

const wasm = pluginUtils.loadCompiler('wasm')
let root: string
let previous: string | undefined
let selection: Mock<typeof pluginUtils.loadCompiler>

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-metro-plugin-'))
  previous = process.env[SETTINGS_ENV]
  delete process.env[SETTINGS_ENV]
  selection = spyOn(pluginUtils, 'loadCompiler').mockReturnValue(wasm)
})

afterEach(() => {
  selection.mockRestore()
  rmSync(root, { recursive: true, force: true })
  if (previous === undefined) delete process.env[SETTINGS_ENV]
  else process.env[SETTINGS_ENV] = previous
})

/** Write `path` under the project root, with its directories */
function write(path: string, content = 'module.exports = {}'): string {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content)
  return file
}

/** A package installed in the project's node_modules */
function install(name: string, file: string): string {
  write(
    `node_modules/${name}/package.json`,
    JSON.stringify({ name, version: '1.0.0' }),
  )
  return write(`node_modules/${name}/${file}`)
}

describe('withDevupUI', () => {
  it('points Metro at the transformer, keeping the rest of the config', () => {
    const resolver = { sourceExts: ['tsx'] }
    const config = {
      projectRoot: root,
      resolver,
      transformer: {
        babelTransformerPath: '/upstream.js',
        minifierPath: 'minifier',
      },
    }
    const wrapped = withDevupUI(config)
    expect(wrapped).toEqual({
      projectRoot: root,
      resolver,
      transformer: {
        babelTransformerPath: TRANSFORMER_PATH,
        minifierPath: 'minifier',
      },
    })
    expect(config.transformer.babelTransformerPath).toBe('/upstream.js')
    expect(TRANSFORMER_PATH).toEndWith('transformer.cjs')
    expect(readSettings()).toEqual({
      upstreamTransformer: '/upstream.js',
      package: '@devup-ui/react',
      include: [],
      runtime: true,
      reanimated: false,
      devupFile: join(root, 'devup.json'),
      engine: undefined,
    })
  })

  it('tells the transformer whether the project has react-native-reanimated', () => {
    install(REANIMATED_PACKAGE, 'index.js')
    withDevupUI({
      projectRoot: root,
      transformer: { babelTransformerPath: '/up.js' },
    })
    expect(readSettings().reanimated).toBe(true)
  })

  it('passes the options to the transformer', () => {
    withDevupUI(
      { projectRoot: root, transformer: { babelTransformerPath: '/up.js' } },
      {
        package: '@acme/ui',
        include: ['lib'],
        runtime: false,
        engine: 'wasm',
        devupFile: 'theme/devup.json',
      },
    )
    expect(readSettings()).toEqual({
      upstreamTransformer: '/up.js',
      package: '@acme/ui',
      include: ['lib'],
      runtime: false,
      reanimated: false,
      devupFile: join(root, 'theme', 'devup.json'),
      engine: 'wasm',
    })
    expect(selection.mock.calls).toEqual([['wasm']])
  })

  it('writes the types of the theme and of React Native into df', () => {
    write(
      'devup.json',
      JSON.stringify({ theme: { colors: { default: { primary: '#000' } } } }),
    )
    withDevupUI({
      projectRoot: root,
      transformer: { babelTransformerPath: '/up.js' },
    })
    expect(readFileSync(join(root, 'df', 'theme.d.ts'), 'utf-8')).toContain(
      'primary',
    )
    expect(readFileSync(join(root, 'df', 'native.d.ts'), 'utf-8')).toBe(
      NATIVE_TYPES,
    )
    expect(readFileSync(join(root, 'df', '.gitignore'), 'utf-8')).toBe('*')
  })

  it('writes the types into the distDir option', () => {
    withDevupUI(
      { projectRoot: root, transformer: { babelTransformerPath: '/up.js' } },
      { distDir: 'types/devup' },
    )
    expect(existsSync(join(root, 'types', 'devup', 'theme.d.ts'))).toBe(true)
    expect(existsSync(join(root, 'types', 'devup', 'native.d.ts'))).toBe(true)
    expect(existsSync(join(root, 'df'))).toBe(false)
  })

  it('resolves the upstreamTransformer option from the project root', () => {
    const local = write('transformer.cjs')
    const config = {
      projectRoot: root,
      transformer: { babelTransformerPath: '/up.js' },
    }
    withDevupUI(config, { upstreamTransformer: './transformer.cjs' })
    expect(readSettings().upstreamTransformer).toBe(local)
    const installed = install('custom-transformer', 'index.js')
    withDevupUI(config, { upstreamTransformer: 'custom-transformer' })
    expect(readSettings().upstreamTransformer).toBe(installed)
    expect(() =>
      withDevupUI(config, { upstreamTransformer: './missing.cjs' }),
    ).toThrow()
  })

  it("prefers Expo's transformer", () => {
    install('@react-native/metro-babel-transformer', 'index.js')
    const expo = install('@expo/metro-config', 'babel-transformer.js')
    withDevupUI({ projectRoot: root, transformer: {} })
    expect(readSettings().upstreamTransformer).toBe(expo)
  })

  it("falls back to React Native's transformer", () => {
    const reactNative = install(
      '@react-native/metro-babel-transformer',
      'index.js',
    )
    withDevupUI({ projectRoot: root })
    expect(readSettings().upstreamTransformer).toBe(reactNative)
  })

  it('tells what to install when no transformer is found', () => {
    expect(() => withDevupUI({ projectRoot: root })).toThrow(
      UpstreamTransformerNotFoundError,
    )
    expect(() => withDevupUI({ projectRoot: root })).toThrow(
      '`upstreamTransformer` option',
    )
  })

  it('keeps the first upstream transformer when wrapped twice', () => {
    const wrapped = withDevupUI({
      projectRoot: root,
      transformer: { babelTransformerPath: '/up.js' },
    })
    expect(withDevupUI(wrapped).transformer.babelTransformerPath).toBe(
      TRANSFORMER_PATH,
    )
    expect(readSettings().upstreamTransformer).toBe('/up.js')
  })

  it('uses the working directory without a project root', () => {
    const cwd = spyOn(process, 'cwd').mockReturnValue(root)
    try {
      withDevupUI({ transformer: { babelTransformerPath: '/up.js' } })
      expect(readSettings().devupFile).toBe(join(root, 'devup.json'))
    } finally {
      cwd.mockRestore()
    }
  })
})
