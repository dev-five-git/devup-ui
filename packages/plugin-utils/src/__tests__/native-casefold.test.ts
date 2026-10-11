import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { discoverTests } from '../../../../test-harness/groups'

it('preserves original paths when native discovery casefolds suffixes, extensions and directory boundaries', () => {
  // Given: independently named upper/mixed witnesses for every native suffix and extension.
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-native-casefold-')),
  )
  const receipt = join(root, 'executed.txt')
  const extensions = ['js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs', 'mts', 'cts']
  const suffixes = ['.test', '_test', '.spec', '_spec']
  const expected = [
    ...extensions.flatMap((extension) =>
      suffixes.flatMap((suffix) => [
        `packages/UPPER/case${suffix.toUpperCase()}.${extension.toUpperCase()}`,
        `packages/Mixed/case${suffix.replace('t', 'T').replace('s', 'S')}.${extension[0]?.toUpperCase()}${extension.slice(1)}`,
      ]),
    ),
    'packages/x_TEST.ts',
    'packages/Dist/generated.SpEc.Ts',
    'packages/Pkg/generated_TeSt.Js',
    'packages/Node_Modules-extra/boundary_TEST.ts',
  ].sort()
  const ignored = [
    'packages/Node_Modules/dependency_TEST.TS',
    'packages/NODE_MODULES/dependency_SPEC.JS',
    'packages/Mixed/Node_Modules/nested.TeSt.Ts',
    'packages/.Hidden/hidden_TEST.TS',
    'packages/Mixed/not_TEST.TSXx',
    'packages/Mixed/not_SpEc.CTsX',
    'packages/Mixed/not_TEST_extra.TS',
  ]
  try {
    writeFileSync(join(root, 'bunfig.toml'), '[test]\nroot="packages"\n')
    for (const file of [...expected, ...ignored]) {
      const path = join(root, file)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(
        path,
        `import {it,expect} from 'bun:test';import {appendFileSync} from 'node:fs';it(${JSON.stringify(file)},()=>{appendFileSync(${JSON.stringify(receipt)},${JSON.stringify(`${file}\n`)});expect(1).toBe(1)})`,
      )
    }
    // When: native automatic discovery executes the corpus without file arguments.
    const native = spawnSync(
      process.execPath,
      [`--config=${join(root, 'bunfig.toml')}`, 'test'],
      { cwd: root, encoding: 'utf8' },
    )
    if (native.error) throw native.error
    const executed = readFileSync(receipt, 'utf8').trim().split(/\r?\n/).sort()
    console.info(
      JSON.stringify({
        case: 'native-casefold-parity',
        native: native.stdout + native.stderr,
        executed,
      }),
    )
    // Then: actual native witnesses and the discoverer agree, retaining original spelling.
    expect(native.status).toBe(0)
    expect(executed).toEqual(expected)
    expect(discoverTests(root)).toEqual(executed)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
