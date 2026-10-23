import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, afterEach, beforeAll, expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import { BuildGeneration } from '../build-generation'
import { createModuleResolver } from '../import-graph'
import {
  ProductionNumbering,
  ProductionNumberingError,
  type ProductionNumberingPlan,
} from '../production-numbering'
import {
  canonical,
  createFixture,
  expected,
  ids,
} from './production-numbering-wasm-fixture'

let root: string
let plans: readonly ProductionNumberingPlan[]
let toId: (path: string) => string
const debug = wasm.isDebug()
beforeAll(async () => {
  root = realpathSync.native(mkdtempSync(join(tmpdir(), 'devup-p3-wasm-')))
  ;({ plans, toId } = await createFixture(root))
})
afterEach(() => {
  wasm.resetBuildState()
  wasm.setModuleResolver(undefined)
  wasm.registerTheme({})
  wasm.registerShorthands({})
  wasm.setDebug(debug)
})
afterAll(() => {
  try {
    wasm.setModuleResolver(undefined)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

function extract(id: string) {
  const output = wasm.codeExtract(
    id,
    readFileSync(join(root, id), 'utf8'),
    '@devup-ui/react',
    'df',
    false,
    false,
    false,
    { '@vanilla-extract/css': null },
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
function physical(context: string, id: string, importer = id) {
  const path = join(root, id)
  return {
    context,
    id,
    source: {
      kind: 'physical' as const,
      path,
      realPath: realpathSync.native(path),
    },
    location: { filename: path, importer },
  }
}
function resolverFor(numbering: ProductionNumbering, context: string) {
  const resolver = createModuleResolver({
    cwd: root,
    conditions: [context],
    toId,
    alias: { ignored: false },
  })
  return (specifier: string, importer: string) => {
    const result = resolver(specifier, importer)
    if (result === undefined) return result
    switch (result.ignored) {
      case true:
        return result
      case undefined:
        numbering.assertReserved(physical(context, result.path, importer))
        return result
      default:
        return result satisfies never
    }
  }
}
type Control = 'complete' | 'omit-bucket' | 'omit-original' | 'wrong-canonical'
function build(
  reverse: boolean,
  active: string,
  control: Control = 'complete',
) {
  wasm.resetBuildState()
  wasm.setDebug(false)
  const ordered = reverse ? plans.toReversed() : plans
  const numbering = new ProductionNumbering(new BuildGeneration(), ordered)
  const engine = {
    ...wasm,
    seedFileMap(files: string[]) {
      switch (control) {
        case 'complete':
          return wasm.seedFileMap(files)
        case 'omit-bucket':
          return wasm.seedFileMap(
            files.filter((id) => id !== 'bucket/raw-server'),
          )
        case 'omit-original':
          return wasm.seedFileMap(
            files.filter((id) => id !== 'outside/raw.css.ts'),
          )
        case 'wrong-canonical':
          wasm.importCanonicalMap(canonical['react-server'])
          return wasm.seedFileMap(files)
        default:
          return control satisfies never
      }
    },
  }
  numbering.seed(engine, active)
  const seeded = wasm.exportFileMap()
  const restored = wasm.exportCanonicalMap()
  const outputs: Record<string, Omit<ReturnType<typeof extract>, 'css'>> = {}
  const publications: Record<string, string> = {}
  for (const plan of ordered) {
    wasm.importCanonicalMap(plan.canonical)
    wasm.setModuleResolver(resolverFor(numbering, plan.context))
    const direct = [
      'outside/raw.css.ts',
      'src/a.tsx',
      'alias/a.tsx',
      'src/b.tsx',
      'src/global.tsx',
      'src/é.tsx',
      'src/\ue000.tsx',
      'src/𐀀.tsx',
    ]
    for (const id of reverse ? direct.toReversed() : direct) {
      numbering.assertReserved(physical(plan.context, id))
      const { css, ...output } = extract(id)
      outputs[`${plan.context}:${id}`] = output
      if (css !== undefined) publications[output.cssFile ?? id] = css
    }
  }
  return {
    seeded,
    restored,
    outputs,
    publications,
    files: wasm.exportFileMap(),
    classes: wasm.exportClassMap(),
    css: ids.map((_, number) => wasm.getCss(number, false)),
    global: wasm.getCss(undefined, false),
  }
}

it.each(['browser', 'react-server'])(
  'keeps raw AB/BA surfaces equal when seed restores %s',
  (active) => {
    // Given the same physical fixture, both frozen contexts and an independent Rust-order universe.
    const AB = build(false, active)
    // When every direct/recursive first allocation and context order reverses.
    const BA = build(true, active)
    // Then originals, single-lookup buckets, maps, code and finished CSS match without normalization.
    expect(AB.seeded).toBe(expected)
    expect(BA.seeded).toBe(expected)
    expect(AB.files).toBe(expected)
    expect(BA.files).toBe(expected)
    expect(BA).toEqual(AB)
    expect(JSON.parse(BA.restored)).toEqual(
      active === 'browser' ? canonical.browser : canonical['react-server'],
    )
    expect(AB.outputs['browser:src/a.tsx']?.code).toContain('df/devup-ui-4.css')
    expect(AB.outputs['react-server:src/a.tsx']?.code).toContain(
      'df/devup-ui-5.css',
    )
    expect(AB.css.join('')).toContain('color:purple')
    expect(AB.global).toContain('width:11px')
    console.info(JSON.stringify({ active, expected, AB, BA }))
  },
)

it.each(['omit-bucket', 'omit-original', 'wrong-canonical'] as const)(
  'breaks the complete raw allocation gate when control is %s',
  (control) => {
    // Given an independently specified complete map and an intentionally defective real delegate.
    const correct = build(false, 'browser')
    // When a second-context bucket, raw original or identity seeding is omitted.
    const defective = build(false, 'browser', control)
    // Then the complete-map gate fails and extraction cannot repair the sorted assignment.
    expect(defective.seeded).not.toBe(expected)
    expect(defective.files).not.toBe(expected)
    expect(defective.outputs).not.toEqual(correct.outputs)
    expect(() => expect(defective.files).toBe(expected)).toThrow()
    console.info(JSON.stringify({ control, defective }))
  },
)

it('rejects a recursive physical child when its original membership is absent', () => {
  // Given allocated union buckets but a defective browser membership.
  wasm.resetBuildState()
  const numbering = new ProductionNumbering(
    new BuildGeneration(),
    plans.map((plan) => ({
      ...plan,
      files: plan.files.filter(
        (file) =>
          plan.context !== 'browser' || file.id !== 'outside/raw.css.ts',
      ),
    })),
  )
  numbering.seed(wasm, 'browser')
  const resolver = resolverFor(numbering, 'browser')
  wasm.setModuleResolver(resolver)
  // When real WASM requests the physical stylesheet recursively.
  expect(() => extract('src/a.tsx')).toThrow('src/a.tsx:1:1')
  // Then the local structured defect survives without claiming JS identity across WASM.
  expect(() => resolver('../outside/raw.css', 'src/a.tsx')).toThrow(
    ProductionNumberingError,
  )
})

it('sorts normalized nonphysical originals with physical and single-lookup buckets when reserved', () => {
  // Given normalized IDs interleaved with originals and cross-context bucket keys.
  const selected = plans.map((plan) => ({
    ...plan,
    nonphysical: ['bucket/alias-browser!', 'outside/raw.css.ts!', 'src/é.tsx!'],
    canonical: { ...plan.canonical, 'outside/raw.css.ts!': 'alias/a.tsx!' },
  }))
  const universe = [
    ...ids.slice(0, 2),
    'alias/a.tsx!',
    ids[2],
    'bucket/alias-browser!',
    ...ids.slice(3, 13),
    'outside/raw.css.ts!',
    ...ids.slice(13, 18),
    'src/é.tsx!',
    ...ids.slice(18),
  ]
  const numbering = new ProductionNumbering(new BuildGeneration(), selected)
  // When one Rust sort/dense seed runs under the identity map.
  numbering.seed(wasm, 'browser')
  // Then the independent mixed universe is dense and the frozen selected map is restored.
  expect(wasm.exportFileMap()).toBe(
    JSON.stringify(
      Object.fromEntries(universe.map((id, number) => [id, number])),
    ),
  )
  expect(JSON.parse(wasm.exportCanonicalMap())).toEqual(selected[0]?.canonical)
})
