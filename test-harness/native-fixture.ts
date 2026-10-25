import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { CoverageError, parseLcov } from './lcov'
import { nativeProcess, nativeReport } from './native-process'

export function nativeFixture(
  inputs: Readonly<Record<string, string>>,
  preload: readonly string[] = [],
) {
  const root = mkdtempSync(join(tmpdir(), 'devup-native-coverage-'))
  const config = join(root, 'bunfig.toml')
  const coverage = join(root, 'coverage')
  writeFileSync(
    config,
    `[test]\npreload=${JSON.stringify(preload)}\ncoverageReporter=["text","lcov"]\ncoverageThreshold={lines=1.0,functions=1.0,statements=1.0}\ncoverageSkipTestFiles=true\ncoverageDir="coverage"\n`,
  )
  for (const [file, code] of Object.entries(inputs))
    writeFileSync(join(root, file), code)
  return {
    root,
    config,
    coverage,
    dispose() {
      rmSync(root, { recursive: true, force: true })
    },
    async run(files: readonly string[] = ['source.test.ts']) {
      if (files.length === 0) throw new CoverageError('empty native lane')
      if (nativeReport(coverage)) throw new CoverageError('stale native report')
      const args = [
        `--config=${config}`,
        'test',
        '--isolate',
        '--coverage',
        ...files.map((file) => `./${file}`),
      ]
      const result = await nativeProcess({
        root,
        executable: process.execPath,
        args,
        timeout: 120000,
      })
      const lcov = nativeReport(coverage)
      return {
        ...result,
        lcov,
        sources: lcov ? parseLcov(lcov) : [],
        identity: {
          executable: process.execPath,
          bun: Bun.version,
          revision: Bun.revision,
          config,
          args,
        },
      }
    },
  }
}
