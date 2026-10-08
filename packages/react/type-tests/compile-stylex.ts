import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

export async function compileStylex(fixture: string) {
  const directory = await mkdtemp(join(tmpdir(), 'devup-stylex-types-'))
  const project = join(directory, 'tsconfig.json')
  try {
    await writeFile(
      project,
      JSON.stringify({
        extends: fileURLToPath(
          new URL('./stylex-transitions.json', import.meta.url),
        ),
        files: [
          fileURLToPath(new URL('../src/compat/stylex.d.ts', import.meta.url)),
          fileURLToPath(new URL(fixture, import.meta.url)),
        ],
      }),
      'utf8',
    )
    const result = Bun.spawnSync([
      process.execPath,
      fileURLToPath(
        new URL(
          '../../../node_modules/@typescript/native/bin/tsc',
          import.meta.url,
        ),
      ),
      '--project',
      project,
      '--pretty',
      'false',
    ])
    return {
      exitCode: result.exitCode,
      diagnostics: result.stdout.toString() + result.stderr.toString(),
    }
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}
