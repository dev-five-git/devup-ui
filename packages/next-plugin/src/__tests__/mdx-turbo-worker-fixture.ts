import { execFile } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { promisify } from 'node:util'

const execute = promisify(execFile)

export async function turboWorkerFixture(
  observer = join(import.meta.dir, 'mdx-turbo-worker-observer.ts'),
) {
  const workspace = resolve(import.meta.dir, '../../../..')
  const landing = join(workspace, 'apps/landing')
  const installed = createRequire(join(landing, 'package.json'))
  const root = mkdtempSync(join(workspace, '.devup-mdx-turbo-'))
  const dispose = () => rmSync(root, { recursive: true, force: true })
  try {
    symlinkSync(
      join(landing, 'node_modules'),
      join(root, 'node_modules'),
      'junction',
    )
    mkdirSync(join(root, 'app'))
    writeFileSync(join(root, 'package.json'), '{}')
    writeFileSync(
      join(root, 'app/layout.js'),
      'export default function Layout({children}) { return <html><body>{children}</body></html> }',
    )
    writeFileSync(join(root, 'app/page.mdx'), '# receiving worker')
    writeFileSync(
      join(root, 'mdx-components.js'),
      'export function useMDXComponents() { return {} }',
    )
    const plugin = join(root, 'remark.cjs')
    const alternate = join(root, 'alternate.cjs')
    writeFileSync(
      plugin,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value = options.text } }',
    )
    writeFileSync(
      alternate,
      'module.exports = function() { return function(tree) { tree.children[0].children[0].value = "private-only" } }',
    )
    await execute(
      process.execPath,
      [
        'build',
        '--target',
        'node',
        '--packages',
        'external',
        '--format',
        'cjs',
        observer,
        '--outfile',
        join(root, 'observer.cjs'),
      ],
      { cwd: workspace, timeout: 30_000 },
    )
    return {
      root,
      plugin,
      dispose,
      async run(text: string, privateFirst?: boolean) {
        const tap = {
          loader: join(root, 'observer.cjs'),
          options:
            privateFirst === undefined ? {} : { alternate, privateFirst },
        }
        writeFileSync(
          join(root, 'next.config.js'),
          `const withMDX = require('@next/mdx')({options:{format:'mdx',remarkPlugins:[[${JSON.stringify(plugin)},{text:${JSON.stringify(text)}}]]}}); const config = withMDX({pageExtensions:['js','mdx'],turbopack:{root:${JSON.stringify(workspace)}}}); config.turbopack.rules['{*,next-mdx-rule}'][0].loaders.unshift(${JSON.stringify(tap)}); module.exports = config;`,
        )
        writeFileSync(join(root, 'observations.jsonl'), '')
        await execute(
          'node',
          [
            installed.resolve('next/dist/bin/next'),
            'build',
            '--turbopack',
            '--experimental-build-mode',
            'compile',
          ],
          {
            cwd: root,
            timeout: 120_000,
            maxBuffer: 4 * 1024 * 1024,
            env: {
              ...process.env,
              NODE_ENV: 'production',
              NEXT_TELEMETRY_DISABLED: '1',
            },
          },
        )
        return readFileSync(join(root, 'observations.jsonl'), 'utf8')
          .trim()
          .split('\n')
          .map((line): unknown => JSON.parse(line))
      },
    }
  } catch (error) {
    dispose()
    throw error
  }
}
