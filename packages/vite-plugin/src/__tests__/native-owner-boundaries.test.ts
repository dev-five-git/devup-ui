import { mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { ProductionNumberingError } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import { createServer, parseAst } from 'vite'

import { compiledFacts } from '../../../plugin-utils/src/guard-facts'
import { watchConfigFiles } from '../config-refresh'
import { NonphysicalModules } from '../nonphysical-modules'
import { ProductionGeneration } from '../production-engine'
import { productionRootClosure } from '../production-root-closure'
import { productionRootSources } from '../production-root-sources'
import { createMissingInputWatch } from '../resolution-watch'
import { productionFixture } from './production-fixture'

it('watches the existing ancestor of a deeply missing configuration', () => {
  // Given a real root and multiple absent parent directories.
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'vite-config-owner-')),
  )
  const path = join(root, 'missing', 'nested', 'devup.json')
  const watched: string[] = []
  try {
    // When enrolling the absent configuration.
    watchConfigFiles([path], (file) => watched.push(file))
    // Then repair observes the exact file and nearest existing ancestor.
    expect(watched).toEqual(
      [path, root].map((file) => file.replaceAll('\\', '/')),
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('leaves opaque namespace rest and computed destructures unclassified', () => {
  // Given actual bundler AST nodes for opaque destructuring.
  const ast = parseAst(
    'import * as DU from "library"; const {[key]: value, ...rest}=DU;',
  )
  // When recording compile-time facts.
  const facts = compiledFacts(ast)
  // Then no guessed member identity is invented.
  expect(facts.references).toEqual([])
})

it('rejects an unknown runtime context before touching engine state', () => {
  // Given a fresh generation without a prepared context.
  const generation = new ProductionGeneration()
  try {
    // When a consumer requests an unknown context.
    // Then the public lifecycle reports the invalid plan.
    expect(() => generation.runtime('missing')).toThrow(
      ProductionNumberingError,
    )
  } finally {
    generation.dispose()
  }
})

it('retains compiled-MDX attachments while invalidating pending root evidence', () => {
  // Given a current extraction root and a resolved compiled Markdown source.
  const modules = new NonphysicalModules([])
  const root = '\0owner:root'
  const child = '\0owner:child'
  modules.direct(root, root, { code: 'export {}' })
  modules.resolved(root, { ignored: true })
  modules.resolved(root, {
    path: child,
    code: 'export const x=1',
    sourceType: 'compiled-mdx',
  })
  expect(modules.source(child)).toEqual({
    code: 'export const x=1',
    sourceType: 'compiled-mdx',
  })
  // When the observed dependency changes before publication.
  modules.invalidate(child)
  // Then stale pending code cannot be reused.
  expect(modules.source(child)).toBeUndefined()
})

it('censuses ordinary JS without an explicit Markdown selection', async () => {
  // Given ordinary source and unselected Markdown in a real root.
  const fixture = await productionFixture()
  const source = await fixture.file('app/src/ordinary.js', 'export {}')
  await fixture.file('app/src/page.mdx', '# Page')
  try {
    // When no includeMdx option is supplied.
    const files = productionRootSources(
      { roots: [join(fixture.root, 'src')] },
      [],
    )
    // Then the census contains only the ordinary source.
    expect(files.map((file) => file.path.replaceAll('\\', '/'))).toEqual([
      source,
    ])
  } finally {
    await fixture.close()
  }
})

it('cuts an explicitly excluded root before reading its missing source', async () => {
  // Given a root file under an excluded output directory, with no file to read.
  const fixture = await productionFixture()
  const path = join(fixture.root, 'output', 'missing.js')
  try {
    // When a caller includes that root in the initial closure frontier.
    const files = productionRootClosure(
      {
        key: 'owner',
        files: [{ path, realPath: path }],
        resolverOptions: { cwd: fixture.root },
        toId: (file) => file,
      },
      { exclude: [join(fixture.root, 'output')] },
      {
        selected: () => true,
        relevant: () => true,
      },
    )
    // Then exclusion prevents IO rather than turning it into a read error.
    expect(files).toEqual([])
  } finally {
    await fixture.close()
  }
})

it('avoids enrolling the entire project when its root contains a missing source', async () => {
  // Given an actual Vite server and its current client environment.
  const fixture = await productionFixture()
  const server = await createServer({
    root: fixture.root,
    configFile: false,
    server: { port: 0 },
    optimizeDeps: { noDiscovery: true },
  })
  const watch = createMissingInputWatch()
  try {
    const environment = server.environments.client
    if (environment === undefined)
      throw new TypeError('Missing native client environment')
    watch.attach(server)
    const importer = join(fixture.root, 'entry.js')
    watch.start(importer, environment)
    const add = spyOn(server.watcher, 'add')
    try {
      // When the missing source is immediately under the project root.
      watch.observe(importer, join(fixture.root, 'missing.js'), environment)
      // Then only exact missing-input bookkeeping is needed, not recursive root enrollment.
      expect(add).not.toHaveBeenCalled()
    } finally {
      add.mockRestore()
    }
  } finally {
    watch.close()
    await server.close()
    await fixture.close()
  }
}, 30000)
