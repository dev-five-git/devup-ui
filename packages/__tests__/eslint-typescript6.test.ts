import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const root = resolve(import.meta.dir, '../..')
const script = `
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const path = require('node:path');
const root = process.argv[1];
const directory = process.argv[2];
const fromRoot = createRequire(path.join(root, 'package.json'));
const fromPlugin = createRequire(path.join(root, 'packages/eslint-plugin/package.json'));
async function main() {
  for (const suffix of ['', '/lib/tsserverlibrary', '/lib/typescript.js', '/package.json']) {
    const actual = fromRoot.resolve('typescript' + suffix);
    const expected = fromRoot.resolve('@typescript/typescript6' + suffix);
    assert.equal(actual, expected);
    assert.ok(fromRoot('typescript' + suffix));
  }
  assert.equal(fromRoot.resolve('node:path'), 'node:path');
  assert.equal(fromRoot('node:path'), path);
  const server = await import('typescript/lib/tsserverlibrary');
  assert.equal(typeof server.default.createProgram, 'function');
  const { Linter } = fromRoot('eslint');
  const tseslint = fromPlugin('typescript-eslint');
  const code = 'async function run() { await 123; }';
  const messages = new Linter({ cwd: directory }).verify(code, [{
    files: ['**/*.ts'],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: { projectService: true, tsconfigRootDir: directory },
    },
    plugins: { '@typescript-eslint': tseslint.plugin },
    rules: { '@typescript-eslint/await-thenable': 'error' },
  }], { filename: path.join(directory, 'probe.ts') });
  assert.deepEqual(messages.map(({ ruleId, messageId, fatal }) => ({ ruleId, messageId, fatal })), [
    { ruleId: '@typescript-eslint/await-thenable', messageId: 'await', fatal: undefined },
  ]);
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
`

it('resolves compatibility subpaths and runs a typed project-service rule through the Node shim', () => {
  const directory = mkdtempSync(join(tmpdir(), 'devup-eslint-typescript6-'))
  try {
    writeFileSync(
      join(directory, 'tsconfig.json'),
      JSON.stringify({
        compilerOptions: {
          strict: true,
          target: 'ESNext',
          lib: ['ESNext'],
          types: [],
        },
        include: ['probe.ts'],
      }),
    )
    writeFileSync(
      join(directory, 'probe.ts'),
      'async function run() { await 123; }',
    )
    const run = Bun.spawnSync(
      [
        'node',
        '--require',
        join(root, 'eslint-typescript6.cjs'),
        '-e',
        script,
        root,
        directory,
      ],
      { cwd: root, timeout: 60000 },
    )
    expect(run.stderr.toString()).toBe('')
    expect(run.exitCode).toBe(0)
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
}, 65000)
