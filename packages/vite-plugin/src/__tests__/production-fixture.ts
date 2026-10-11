import {
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  realpath,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { build, type Plugin, resolveConfig, type UserConfig } from 'vite'

import { DevupUI } from '../plugin'
import { productionEngine } from '../production-engine'
import { ProductionActivation } from '../production-numbering'

export function extractOwned(color: string, id = 'owned.tsx') {
  wasm.registerTheme({})
  wasm.registerShorthands({})
  wasm.setDebug(false)
  wasm.setPrefix(null)
  wasm.setModuleResolver(undefined)
  const output = wasm.codeExtract(
    id,
    `import {Box} from '@devup-ui/react'; export const view=<Box bg='${color}'/>;`,
    '@devup-ui/react',
    'df',
    false,
    false,
    false,
    {},
  )
  try {
    return {
      css: output.css ?? '',
      state: productionEngine.capture(),
    }
  } finally {
    output.free()
  }
}

export function nativeCompletionDeadline(
  reject: (reason: Error) => void,
  milliseconds = 30000,
): ReturnType<typeof setTimeout> {
  return setTimeout(
    reject,
    milliseconds,
    new RangeError('Native watch completion deadline'),
  )
}

export function nativeCompletions(watcher: {
  on(
    event: 'event',
    listener: (event: { code: string; error?: unknown }) => void,
  ): unknown
}) {
  const events: { code: string; error?: unknown }[] = []
  let wake: (() => void) | undefined
  watcher.on('event', (event) => {
    if (event.code !== 'BUNDLE_END' && event.code !== 'ERROR') return
    events.push(event)
    wake?.()
  })
  return async function next(): Promise<void> {
    let timer: ReturnType<typeof setTimeout> | undefined
    try {
      if (events.length === 0)
        await new Promise<void>((resolve, reject) => {
          wake = resolve
          timer = nativeCompletionDeadline(reject)
        })
      const event = events.shift()
      if (event?.code === 'ERROR') throw event.error
    } finally {
      clearTimeout(timer)
      wake = undefined
    }
  }
}

export async function productionFixture() {
  const directory = (
    await realpath(await mkdtemp(join(tmpdir(), 'devup-v1-native-')))
  ).replaceAll('\\', '/')
  const root = `${directory}/app`
  async function file(path: string, code: string) {
    const target = join(directory, path)
    await mkdir(dirname(target), { recursive: true })
    await writeFile(target, code)
    return target.replaceAll('\\', '/')
  }
  await file('app/package.json', '{"name":"v1-native","type":"module"}')
  await file('app/tsconfig.json', '{}')
  async function activation() {
    const owner = new ProductionActivation(
      {
        package: '@devup-ui/react',
        devupFile: 'devup.json',
        distDir: 'df',
        cssDir: undefined,
        extractCss: true,
        debug: false,
        include: [],
        singleCss: false,
        prefix: undefined,
        shorthands: undefined,
        sourceDirs: undefined,
        mdxExtensions: ['.mdx'],
        atomHoist: undefined,
        importAliases: {},
      },
      () => {},
    )
    owner.configure(
      await resolveConfig(
        {
          root,
          configFile: false,
          logLevel: 'silent',
          build: {
            watch: {},
            lib: { entry: `${root}/src/main.js`, formats: ['es'] },
          },
        },
        'build',
      ),
    )
    return owner
  }
  async function run(
    singleCss: boolean,
    plugins: readonly Plugin[] = [],
    options: UserConfig = {},
  ) {
    const result = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      plugins: options.plugins ?? [DevupUI({ singleCss }), ...plugins],
      build: {
        write: false,
        lib: { entry: join(root, 'src/main.js'), formats: ['es'] },
      },
      ...options,
    })
    return (Array.isArray(result) ? result : [result]).flatMap((bundle) => {
      if (!('output' in bundle))
        throw new TypeError('Expected completed native output')
      return bundle.output
    })
  }
  async function learned() {
    const path = join(root, 'df/numbering')
    const ids = new Set<string>()
    for (const name of await readdir(path)) {
      const list: unknown = JSON.parse(await readFile(join(path, name), 'utf8'))
      if (!Array.isArray(list)) throw new TypeError('Expected ID list')
      for (const id of list) {
        if (typeof id !== 'string') throw new TypeError('Expected string ID')
        ids.add(id)
      }
    }
    return [...ids].sort()
  }
  async function storeBytes() {
    const path = join(root, 'df/numbering')
    const entries = await readdir(path)
    return Object.fromEntries(
      await Promise.all(
        entries.map(async (name) => [
          name,
          await readFile(join(path, name), 'utf8'),
        ]),
      ),
    )
  }
  async function watchGeneration(singleCss: boolean) {
    const unrelated = await file(
      'app/src/unrelated.js',
      'export const value=1;',
    )
    await file(
      'app/index.html',
      '<script type="module" src="/src/main.js"></script>',
    )
    await file(
      'app/src/main.js',
      "import {cls} from 'virtual-style'; import {value} from './unrelated.js'; globalThis.nativeStyle=cls; globalThis.nativeValue=value;",
    )
    const observations: {
      readonly files: string
      readonly code: string
      readonly css: string
    }[] = []
    const watcher = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      build: { watch: {}, write: false },
      plugins: [
        DevupUI({ singleCss }),
        {
          name: 'watch-generation-provider',
          resolveId(id) {
            if (id === 'virtual-style') return '\0native:watch-style.js'
          },
          load(id) {
            if (id === '\0native:watch-style.js')
              return "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});"
          },
          generateBundle(_options, bundle) {
            observations.push({
              files: wasm.exportFileMap(),
              code: this.getModuleInfo(unrelated)?.code ?? '',
              css: Object.values(bundle)
                .filter((output) => output.type === 'asset')
                .map((output) => output.source)
                .join(''),
            })
          },
        },
      ],
    })
    if (Array.isArray(watcher) || !('on' in watcher))
      throw new TypeError('Expected native watcher')
    const next = nativeCompletions(watcher)
    try {
      await next()
      const first = observations.at(-1)
      await writeFile(unrelated, 'export const value=2;')
      await next()
      while (!/value\s*=\s*2/.test(observations.at(-1)?.code ?? ''))
        await next()
      return { first, second: observations.at(-1), ids: await learned() }
    } finally {
      await watcher.close()
    }
  }
  return {
    directory,
    root,
    file,
    run,
    learned,
    storeBytes,
    activation,
    watchGeneration,
    close: () => rm(directory, { recursive: true, force: true }),
  }
}
