import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { collectDevupConfigFiles } from './config-files'

describe('collectDevupConfigFiles', () => {
  let dir: string

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'devup-config-files-'))
  })

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true })
  })

  it('returns an absolute path when the relative root is missing', () => {
    const root = join(dir, 'devup.json')

    const files = collectDevupConfigFiles(relative(process.cwd(), root))

    expect(files).toEqual([root])
  })

  it.each(['{}', '{"extends":[]}'])(
    'returns only the root when it has no parents: %s',
    (content) => {
      const root = join(dir, 'devup.json')
      writeFileSync(root, content)

      const files = collectDevupConfigFiles(root)

      expect(files).toEqual([root])
    },
  )

  it('retains missing bases when other bases exist', () => {
    const root = join(dir, 'devup.json')
    const missing = join(dir, 'missing.json')
    const base = join(dir, 'base.json')
    writeFileSync(
      root,
      JSON.stringify({ extends: ['./missing.json', './base.json'] }),
    )
    writeFileSync(base, '{}')

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root, missing, base])
  })

  it('resolves nested string and array extends relative to each declaring file', () => {
    const root = join(dir, 'devup.json')
    const parent = join(dir, 'nested', 'parent.json')
    const grandparent = join(dir, 'base.json')
    mkdirSync(join(dir, 'nested'))
    writeFileSync(root, JSON.stringify({ extends: './nested/parent.json' }))
    writeFileSync(parent, JSON.stringify({ extends: ['../base.json'] }))
    writeFileSync(grandparent, '{}')

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root, parent, grandparent])
  })

  it('visits each base depth-first in the loader merge order', () => {
    const root = join(dir, 'devup.json')
    const first = join(dir, 'first.json')
    const firstBase = join(dir, 'first-base.json')
    const second = join(dir, 'second.json')
    const secondBase = join(dir, 'second-base.json')
    writeFileSync(
      root,
      JSON.stringify({ extends: ['./first.json', './second.json'] }),
    )
    writeFileSync(first, JSON.stringify({ extends: ['./first-base.json'] }))
    writeFileSync(second, JSON.stringify({ extends: ['./second-base.json'] }))
    writeFileSync(firstBase, '{}')
    writeFileSync(secondBase, '{}')

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root, first, firstBase, second, secondBase])
  })

  it('deduplicates normalized paths and shared ancestors at their first visit', () => {
    const root = join(dir, 'devup.json')
    const first = join(dir, 'first.json')
    const shared = join(dir, 'shared.json')
    const second = join(dir, 'second.json')
    writeFileSync(
      root,
      JSON.stringify({
        extends: [
          './first.json',
          './nested/../first.json',
          second,
          './shared.json',
        ],
      }),
    )
    writeFileSync(first, JSON.stringify({ extends: ['./shared.json'] }))
    writeFileSync(second, JSON.stringify({ extends: ['./shared.json'] }))
    writeFileSync(shared, '{}')

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root, first, shared, second])
  })

  it('continues to later bases when an inheritance cycle reaches the root', () => {
    const root = join(dir, 'devup.json')
    const base = join(dir, 'base.json')
    const missing = join(dir, 'missing.json')
    writeFileSync(
      root,
      JSON.stringify({ extends: ['./base.json', './missing.json'] }),
    )
    writeFileSync(base, JSON.stringify({ extends: './devup.json' }))

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root, base, missing])
  })

  it('terminates when a config directly extends itself', () => {
    const root = join(dir, 'devup.json')
    writeFileSync(root, JSON.stringify({ extends: './devup.json' }))

    const files = collectDevupConfigFiles(root)

    expect(files).toEqual([root])
  })

  it.each(['devup.json', 'base.json'])(
    'locates malformed JSON in %s',
    (name) => {
      const root = join(dir, 'devup.json')
      const invalid = join(dir, name)
      writeFileSync(root, JSON.stringify({ extends: './base.json' }))
      writeFileSync(invalid, '{ broken')

      const collect = () => collectDevupConfigFiles(root)

      expect(collect).toThrow(SyntaxError)
      expect(collect).toThrow(
        `${invalid}:1:1: devup config cannot use \`JSON\` at build time: SyntaxError:`,
      )
      expect(collect).toThrow('needs valid JSON with an object at the root')
    },
  )

  it.each(['null', '[]', 'true', '"config"'])(
    'rejects a non-object config: %s',
    (content) => {
      const root = join(dir, 'devup.json')
      writeFileSync(root, content)

      const collect = () => collectDevupConfigFiles(root)

      expect(collect).toThrow(
        new TypeError(
          `${root}:1:1: devup config cannot use \`config\` at build time: the JSON root is not a config object; needs a non-null object, not an array or primitive`,
        ),
      )
    },
  )

  it.each([null, 42, {}])('rejects unsupported extends values: %j', (value) => {
    const root = join(dir, 'devup.json')
    writeFileSync(root, JSON.stringify({ extends: value }))

    const collect = () => collectDevupConfigFiles(root)

    expect(collect).toThrow(
      new TypeError(
        `${root}:1:1: devup config cannot use \`extends\` at build time: extends is neither a string nor an array; needs a path string or an array of path strings`,
      ),
    )
  })

  it('rejects a non-string entry when extends is an array', () => {
    const root = join(dir, 'devup.json')
    writeFileSync(root, JSON.stringify({ extends: [42] }))

    const collect = () => collectDevupConfigFiles(root)

    expect(collect).toThrow(
      new TypeError(
        `${root}:1:1: devup config cannot use \`extends entry\` at build time: an extends array entry is not a string; needs every entry to be a path string`,
      ),
    )
  })
})
