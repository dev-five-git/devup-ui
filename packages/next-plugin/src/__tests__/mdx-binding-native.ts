import { createHash } from 'node:crypto'
import {
  mkdtempSync,
  readFile,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import {
  captureMdxBindingReceipt,
  createMdxBinding,
  MdxBindingError,
} from '../mdx-binding'
import { createWasm } from '../wasm'

type Runner = (
  input: {
    readonly resource: string
    readonly loaders: readonly {
      readonly loader: string
      readonly options: object
      readonly ident: string
    }[]
    readonly readResource: typeof readFile
    readonly context: {
      readonly sourceMap: boolean
      readonly _compiler: object
      readonly getOptions: (this: { readonly query: unknown }) => unknown
    }
  },
  callback: (error: unknown, result: unknown) => void,
) => void

function isRunner(value: unknown): value is Runner {
  return typeof value === 'function'
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
function packageVersion(path: string): string {
  const metadata: unknown = JSON.parse(readFileSync(path, 'utf8'))
  if (!isRecord(metadata) || typeof metadata.version !== 'string')
    throw new TypeError('missing installed package version')
  return metadata.version
}

const workspace = process.argv[2]
if (!workspace) throw new TypeError('workspace argument required')
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const loader = installed.resolve('@next/mdx/mdx-js-loader')
const compiler = createRequire(loader).resolve('@mdx-js/loader')
const compilerPackage = join(dirname(compiler), 'package.json')
const native: unknown = installed(
  'next/dist/compiled/loader-runner/LoaderRunner.js',
)
if (!isRecord(native) || !isRunner(native.runLoaders))
  throw new TypeError('installed runner unavailable')
const runLoaders = native.runLoaders
const root = mkdtempSync(join(tmpdir(), 'devup-mdx-binding-'))
try {
  const filename = join(root, 'page.mdx')
  const plugin = join(root, 'remark.cjs')
  writeFileSync(
    plugin,
    'module.exports = function(options) { options.visits += 1; return function() {} }',
  )
  writeFileSync(
    filename,
    `import { Box, css, globalCss } from '@devup-ui/react'

export const accent = css({ color: 'purple' })

export const globalStyle = globalCss({ body: { margin: 0 } })

# Native maps parity

<Box bg={['red', null, 'blue']} p={2} _hover={{ color: 'green' }} />
`,
  )
  const identity = {
    resolvedPath: loader,
    packageVersion: packageVersion(join(dirname(loader), 'package.json')),
    verifiedFileHash: createHash('sha256')
      .update(readFileSync(loader))
      .digest('hex'),
  }
  if (
    identity.packageVersion !== '16.3.6' ||
    packageVersion(compilerPackage) !== '3.1.1' ||
    identity.verifiedFileHash !==
      '2ba24dda327d48e82b4fbcd31f4b5521ebd76e9cdbeb591fd42a0a10beb2465a'
  )
    throw new TypeError('exact installed MDX compiler changed')
  const ownerFacts = {
    ownerID: 'native/maps-parity',
    configFile: join(root, 'next.config.mjs'),
  }
  const binding = createMdxBinding(ownerFacts)
  const outputs: {
    readonly engine: ReturnType<typeof createWasm>
    readonly sourceMap: boolean
    readonly compilerCode: string
    readonly code: string
    readonly css: string
    readonly classes: string
    readonly compilerMap: boolean
    readonly wasmMap: boolean
    readonly visits: number
  }[] = []
  let original: ReturnType<typeof captureMdxBindingReceipt> | undefined
  for (const sourceMap of [false, true]) {
    const shared = { visits: 0 }
    const options = { jsx: true, remarkPlugins: [[plugin, shared]] }
    const receipt = captureMdxBindingReceipt({
      owner: ownerFacts,
      evaluation: {
        evaluationID: 'native-evaluation',
        configFile: ownerFacts.configFile,
        projectDir: root,
        phase: 'phase-production-build',
        isolateID: 'native-driver',
      },
      delivery: {
        sourceMap,
        layer: sourceMap ? 'rsc' : undefined,
        isServer: sourceMap,
        compilerName: sourceMap ? 'server' : 'client',
      },
      loaders: [{ ...identity, options }],
    })
    binding.accept(receipt)
    original ??= receipt
    const result = await new Promise<unknown>((resolve, reject) =>
      runLoaders(
        {
          resource: filename,
          loaders: [{ loader, options, ident: 'binding-native' }],
          readResource: readFile,
          context: {
            sourceMap,
            _compiler: {},
            getOptions() {
              return this.query
            },
          },
        },
        (error, output) => (error ? reject(error) : resolve(output)),
      ),
    )
    if (!isRecord(result) || !Array.isArray(result.result))
      throw new TypeError('invalid native loader output')
    const source: unknown = result.result[0]
    const map: unknown = result.result[1]
    if (typeof source !== 'string' && !Buffer.isBuffer(source))
      throw new TypeError('missing real compiler source')
    const code = source.toString()
    const engine = createWasm(workspace)
    engine.setPrefix('binding-')
    engine.seedFileMap([filename])
    engine.importCanonicalMap({})
    engine.importClassMap({})
    const extract = sourceMap
      ? engine.codeExtract
      : engine.codeExtractWithoutSourceMap
    const output = extract(
      filename,
      code,
      '@devup-ui/react',
      './df',
      true,
      false,
      false,
      {},
    )
    try {
      outputs.push({
        engine,
        sourceMap,
        compilerCode: code,
        code: output.code,
        css: engine.getCss(null, false),
        classes: engine.exportClassMap(),
        compilerMap: map !== undefined,
        wasmMap: output.map !== undefined,
        visits: shared.visits,
      })
    } finally {
      output.free()
    }
  }
  const [off, on] = outputs
  if (
    !off ||
    !on ||
    off.compilerCode !== on.compilerCode ||
    off.code !== on.code ||
    off.css !== on.css ||
    off.classes !== on.classes ||
    !off.css.includes('background:red') ||
    !off.css.includes('background:blue') ||
    !off.css.includes('color:green') ||
    !off.css.includes('color:purple') ||
    !off.css.includes('body{margin:0}') ||
    off.compilerMap ||
    !on.compilerMap ||
    off.wasmMap ||
    !on.wasmMap ||
    off.visits !== 1 ||
    on.visits !== 1
  )
    throw new TypeError('real compiler/WASM maps parity failed')
  const before = binding.state()
  if (!original) throw new TypeError('missing original native receipt')
  const changed = captureMdxBindingReceipt({
    ...original,
    loaders: [
      { ...identity, options: { jsx: true, remarkPlugins: [() => undefined] } },
    ],
  })
  let cause: unknown
  try {
    binding.accept(changed)
  } catch (error) {
    if (!(error instanceof MdxBindingError)) throw error
    cause = error
  }
  if (
    !(cause instanceof MdxBindingError) ||
    binding.state() !== before ||
    on.engine.getCss(null, false) !== on.css ||
    on.engine.exportClassMap() !== on.classes
  )
    throw new TypeError('negative binding published state')
  console.info(
    JSON.stringify({
      identity,
      compilerVersion: packageVersion(compilerPackage),
      parity: true,
      rejectedPath: cause.path,
      css: on.css,
      classes: on.classes,
      outputs: outputs.map(({ sourceMap, compilerMap, wasmMap, visits }) => ({
        sourceMap,
        compilerMap,
        wasmMap,
        visits,
      })),
    }),
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
