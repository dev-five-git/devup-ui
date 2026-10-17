import {
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import {
  BuildGeneration,
  ClosedBuildGenerationError,
} from '../build-generation'
import { runBuildOperation } from '../build-session'
import {
  ProductionNumbering,
  ProductionNumberingError,
} from '../production-numbering'

const capture = () => ({
  sheet: wasm.exportSheet(),
  classes: wasm.exportClassMap(),
  files: wasm.exportFileMap(),
  canonical: wasm.exportCanonicalMap(),
})
type State = Readonly<ReturnType<typeof capture>>
const readMap = (serialized: string): unknown => JSON.parse(serialized)
const canonicalFor = (context: string) => ({
  'a.tsx': `${context}/a`,
  'b.tsx': `${context}/b`,
})
const engine = {
  reset: wasm.resetBuildState,
  restore(state: State) {
    wasm.importSheet(readMap(state.sheet))
    wasm.importClassMap(readMap(state.classes))
    wasm.importFileMap(readMap(state.files))
    wasm.importCanonicalMap(readMap(state.canonical))
  },
  capture,
}
let root: string
let debug: boolean
beforeEach(() => {
  root = realpathSync.native(mkdtempSync(join(tmpdir(), 'devup-p3-owner-')))
  for (const id of ['a.tsx', 'b.tsx'])
    writeFileSync(
      join(root, id),
      "import {Box} from '@devup-ui/react'; export const view=<Box color='red'/>;",
    )
  debug = wasm.isDebug()
  wasm.resetBuildState()
})
afterEach(() => {
  try {
    wasm.resetBuildState()
    wasm.setModuleResolver(undefined)
    wasm.registerTheme({})
    wasm.registerShorthands({})
    wasm.setDebug(debug)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
function reservation(owner: BuildGeneration<State>, context: string) {
  return new ProductionNumbering(owner, [
    {
      context,
      files: ['a.tsx', 'b.tsx'].map((id) => ({
        context,
        id,
        path: join(root, id),
        realPath: join(root, id),
      })),
      nonphysical: [],
      canonical: canonicalFor(context),
    },
  ])
}
function extract(
  numbering: ProductionNumbering,
  context: string,
  id = 'a.tsx',
) {
  const path = join(root, id)
  numbering.assertReserved({
    context,
    id,
    source: { kind: 'physical', path, realPath: path },
    location: { filename: path },
  })
  const output = wasm.codeExtract(
    id,
    readFileSync(path, 'utf8'),
    '@devup-ui/react',
    'df',
    false,
    false,
    false,
    {},
  )
  try {
    return {
      code: output.code,
      map: output.map,
      css: output.css,
      cssFile: output.cssFile,
      dependencies: output.dependencies,
      updatedBaseStyle: output.updatedBaseStyle,
    }
  } finally {
    output.free()
  }
}
function run<T>(
  owner: BuildGeneration<State>,
  context: string,
  action: () => T,
): T {
  return runBuildOperation({ integration: 'Webpack P3 test', root }, () =>
    owner.run(engine, () => {
      wasm.setDebug(false)
      wasm.setModuleResolver(undefined)
      wasm.registerTheme({})
      wasm.registerShorthands({})
      wasm.importCanonicalMap(canonicalFor(context))
      return action()
    }),
  )
}

it('restores four real maps without reseeding when A resumes after B', () => {
  // Given separate owners with independently specified original/bucket universes.
  const A = new BuildGeneration<State>()
  const B = new BuildGeneration<State>()
  const a = reservation(A, 'A')
  const b = reservation(B, 'B')
  let seeds = 0
  const counting = {
    ...wasm,
    seedFileMap(files: string[]) {
      seeds += 1
      wasm.seedFileMap(files)
    },
  }
  const first = run(A, 'A', () => {
    a.seed(counting, 'A')
    return extract(a, 'A')
  })
  const captured = engine.capture()
  expect(captured.files).toBe('{"A/a":0,"A/b":1,"a.tsx":2,"b.tsx":3}')
  const other = run(B, 'B', () => {
    b.seed(wasm, 'B')
    return extract(b, 'B', 'b.tsx')
  })
  expect(engine.capture().files).toBe('{"B/a":0,"B/b":1,"a.tsx":2,"b.tsx":3}')
  // When the admitted A operation resets/restores/configures and calls its retained primitive.
  const resumed = run(A, 'A', () => {
    a.seed(counting, 'A')
    expect(engine.capture()).toEqual(captured)
    return extract(a, 'A')
  })
  // Then raw output and all four captured maps survive the unrelated B operation.
  expect(resumed).toEqual({ ...first, css: undefined })
  expect(other.code).toContain('df/devup-ui-1.css')
  expect(first.code).toContain('df/devup-ui-0.css')
  expect(engine.capture()).toEqual(captured)
  expect(seeds).toBe(1)
  expect(wasm.getCss(0, false)).toContain('color:red')
})

it.each(['action', 'capture'] as const)(
  'requires caller abort when %s fails after real seed',
  (mode) => {
    // Given actual seed/allocation and a narrow fault at the selected owner boundary.
    const owner = new BuildGeneration<State>()
    const numbering = reservation(owner, 'A')
    const fault = new RangeError(mode)
    const selected =
      mode === 'capture'
        ? {
            ...engine,
            capture(): State {
              throw fault
            },
          }
        : engine
    // When the admitted operation fails and the integration aborts its owner.
    expect(() => {
      try {
        runBuildOperation({ integration: 'Webpack P3 test', root }, () =>
          owner.run(selected, () => {
            numbering.seed(wasm, 'A')
            extract(numbering, 'A')
            if (mode === 'action') throw fault
          }),
        )
      } catch (error) {
        owner.dispose()
        throw error
      }
    }).toThrow(fault)
    // Then no retained primitive or owner can reuse a successful-but-uncommitted seed.
    expect(owner.disposed).toBe(true)
    expect(() => numbering.seed(wasm, 'A')).toThrow(ClosedBuildGenerationError)
    expect(() => run(owner, 'A', () => extract(numbering, 'A'))).toThrow(
      ClosedBuildGenerationError,
    )
  },
)

it.each(['seed', 'restore'] as const)(
  'keeps the primitive terminal when %s fails inside an owner',
  (mode) => {
    // Given a real owner and unavailable downstream fault injected only at that seam.
    const owner = new BuildGeneration<State>()
    const numbering = reservation(owner, 'A')
    const fault = new RangeError(mode)
    const faulty = {
      seedFileMap(files: string[]) {
        if (mode === 'seed') throw fault
        wasm.seedFileMap(files)
      },
      importCanonicalMap(map: Readonly<Record<string, string>>) {
        if (mode === 'restore' && Object.keys(map).length > 0) throw fault
        wasm.importCanonicalMap(map)
      },
    }
    // When seed fails and subsequent owner operations try the same primitive.
    expect(() => run(owner, 'A', () => numbering.seed(faulty, 'A'))).toThrow(
      ProductionNumberingError,
    )
    // Then replay of real maps cannot revive that primitive; the caller aborts.
    expect(() => run(owner, 'A', () => numbering.seed(wasm, 'A'))).toThrow(
      ProductionNumberingError,
    )
    owner.dispose()
    expect(() => numbering.seed(wasm, 'A')).toThrow(ClosedBuildGenerationError)
  },
)

it('replays fresh reservations when an old owner and primitive have disposed', () => {
  // Given an old real captured generation followed by actual disposal.
  const old = new BuildGeneration<State>()
  const retained = reservation(old, 'A')
  const first = run(old, 'A', () => {
    retained.seed(wasm, 'A')
    return extract(retained, 'A')
  })
  const state = engine.capture()
  old.dispose()
  const fresh = new BuildGeneration<State>()
  const numbering = reservation(fresh, 'A')
  // When a fresh owner constructs and seeds its own primitive.
  const replayed = run(fresh, 'A', () => {
    numbering.seed(wasm, 'A')
    return extract(numbering, 'A')
  })
  // Then raw output/four maps replay, while the disposed primitive remains rejected.
  expect(replayed).toEqual(first)
  expect(engine.capture()).toEqual(state)
  expect(() => retained.seed(wasm, 'A')).toThrow(ClosedBuildGenerationError)
})
