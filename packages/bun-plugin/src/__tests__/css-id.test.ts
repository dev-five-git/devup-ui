import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { cssDirName, cssNamespace, resolveCssId } from '../css-id'

const distDir = 'df'

// Two checkouts of one repository, as produced by `git worktree add`. They hold
// byte-identical sources at identical repository-relative paths and differ only
// in their root, which is exactly the situation Bun's content-keyed transpiler
// cache collapses into a single entry.
const checkoutA = join('/repos', 'app', 'worktree-a')
const checkoutB = join('/repos', 'app', 'worktree-b')
const importerIn = (checkout: string) =>
  join(checkout, 'src', 'components', 'Card.tsx')

// The specifier the extractor injects: relative to the importing file.
const injectedSpecifier = '../../df/devup-ui/devup-ui.css'

describe('resolveCssId', () => {
  it('resolves the injected stylesheet onto the virtual namespace', () => {
    expect(
      resolveCssId(injectedSpecifier, importerIn(checkoutA), distDir),
    ).toEqual({
      path: 'devup-ui.css',
      namespace: cssNamespace,
    })
  })

  it('resolves numbered per-file stylesheets', () => {
    expect(
      resolveCssId(
        '../../df/devup-ui/devup-ui-12.css',
        importerIn(checkoutA),
        distDir,
      ),
    ).toEqual({
      path: 'devup-ui-12.css',
      namespace: cssNamespace,
    })
  })

  it('strips a query suffix from the stylesheet name', () => {
    expect(
      resolveCssId(
        '../../df/devup-ui/devup-ui.css?inline',
        importerIn(checkoutA),
        distDir,
      ),
    ).toEqual({
      path: 'devup-ui.css',
      namespace: cssNamespace,
    })
  })

  it('resolves against the cwd when there is no importer', () => {
    expect(
      resolveCssId(
        join(distDir, cssDirName, 'devup-ui.css'),
        undefined,
        distDir,
      ),
    ).toEqual({
      path: 'devup-ui.css',
      namespace: cssNamespace,
    })
  })

  // --- The regression this module exists for -------------------------------
  //
  // Bun stores transpiled modules in a machine-wide cache keyed by module
  // contents, with plugin-resolved specifiers baked in and neither the cwd nor
  // the importer in the key. Any id that varies per checkout therefore leaks
  // into the other checkout as
  //   "Cannot find module '<other checkout>/df/devup-ui/devup-ui.css'".

  it('yields the same, path-free id for two checkouts of one repository', () => {
    const fromA = resolveCssId(
      injectedSpecifier,
      importerIn(checkoutA),
      distDir,
    )
    const fromB = resolveCssId(
      injectedSpecifier,
      importerIn(checkoutB),
      distDir,
    )

    expect(fromA).toEqual(fromB)
    // Nothing checkout-specific may survive into the resolved id.
    expect(fromA?.path).not.toContain(checkoutA)
    expect(fromB?.path).not.toContain(checkoutB)
  })

  it('repairs a foreign absolute path baked in by an older plugin version', () => {
    // What a poisoned cache entry hands back: checkout A's absolute stylesheet
    // path, replayed while checkout B is the one being loaded.
    const poisoned = join(checkoutA, distDir, cssDirName, 'devup-ui.css')

    expect(resolveCssId(poisoned, importerIn(checkoutB), distDir)).toEqual({
      path: 'devup-ui.css',
      namespace: cssNamespace,
    })
  })

  // --- Stylesheets that are not ours ---------------------------------------

  it('ignores a devup-ui.css that does not live in the dist css dir', () => {
    expect(
      resolveCssId('../../vendor/devup-ui.css', importerIn(checkoutA), distDir),
    ).toBeUndefined()
  })

  it('ignores a css dir nested under a different dist dir', () => {
    expect(
      resolveCssId(
        '../../other/devup-ui/devup-ui.css',
        importerIn(checkoutA),
        distDir,
      ),
    ).toBeUndefined()
  })

  it('ignores a differently named stylesheet in the dist css dir', () => {
    expect(
      resolveCssId(
        '../../df/devup-ui/theme.css',
        importerIn(checkoutA),
        distDir,
      ),
    ).toBeUndefined()
  })

  it('ignores an empty specifier', () => {
    expect(resolveCssId('', importerIn(checkoutA), distDir)).toBeUndefined()
  })
})
