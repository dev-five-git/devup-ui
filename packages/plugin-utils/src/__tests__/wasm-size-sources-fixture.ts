import { execFileSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { gitEnvironment } from '../../../../wasm-size-sources.js'

export function sourceFixture(
  roots: string[],
  author = 'Source fixture',
  environment = process.env,
) {
  const root = mkdtempSync(join(tmpdir(), 'wasm-size-sources-'))
  roots.push(root)
  const git = (args: readonly string[], input = '') =>
    execFileSync('git', ['-C', root, ...args], {
      encoding: 'utf8',
      input,
      env: {
        ...gitEnvironment(environment),
        GIT_MASTER: '1',
        GIT_AUTHOR_NAME: author,
        GIT_AUTHOR_EMAIL: 'source@example.invalid',
        GIT_COMMITTER_NAME: 'Source fixture',
        GIT_COMMITTER_EMAIL: 'source@example.invalid',
        GIT_AUTHOR_DATE: '2026-01-01T00:00:00Z',
        GIT_COMMITTER_DATE: '2026-01-01T00:00:00Z',
      },
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trim()
  git(['init', '--object-format=sha1'])
  const tree = git(['mktree'])
  const lagging = git(['commit-tree', tree], 'lagging base')
  const base = git(['commit-tree', tree, '-p', lagging], 'advanced base')
  const head = git(['commit-tree', tree, '-p', lagging], 'pull request head')
  const candidate = git(
    ['commit-tree', tree, '-p', base, '-p', head],
    'tested merge',
  )
  const dangling = git(['commit-tree', tree, '-p', head], 'missing parent')
  const missingParent = git(
    ['commit-tree', tree, '-p', dangling],
    'orphaned candidate',
  )
  const blob = git(['hash-object', '-w', '--stdin'], 'not a commit')
  return {
    root,
    tree,
    blob,
    lagging,
    base,
    head,
    candidate,
    dangling,
    missingParent,
  }
}
