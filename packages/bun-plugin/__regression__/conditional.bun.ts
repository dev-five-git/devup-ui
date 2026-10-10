import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const entry = resolve(import.meta.dir, '../src/register.ts').replaceAll(
  '\\',
  '/',
)
const wasm = resolve(
  import.meta.dir,
  '../../../bindings/devup-ui-wasm/pkg/index.js',
).replaceAll('\\', '/')
const scratch = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'bun-conditions',
)
mkdirSync(scratch, { recursive: true })

function run(script: string) {
  const root = realpathSync.native(mkdtempSync(join(scratch, 'conditional-')))
  try {
    // Given: declaration order makes accidentally enabling require observable.
    const branches = {
      require: '811px',
      custom: '821px',
      browser: '823px',
      bun: '827px',
      node: '829px',
      import: '839px',
      module: '853px',
      default: '857px',
    }
    const exports = Object.fromEntries(
      Object.keys(branches).map((name) => [name, `./${name}.js`]),
    )
    const files = {
      'bunfig.toml': '',
      'package.json': '{"type":"module"}',
      'wrong.js': "export const width = '863px'",
      'node_modules/conditional/package.json': JSON.stringify({
        type: 'module',
        exports: {
          '.': exports,
          './node-first': {
            node: './node.js',
            bun: './bun.js',
            default: './default.js',
          },
          './import-only': {
            require: './require.js',
            import: './import.js',
            default: './default.js',
          },
          './module-first': {
            module: './module.js',
            import: './import.js',
            default: './default.js',
          },
          './fallback': { default: './default.js' },
        },
      }),
      ...Object.fromEntries(
        Object.entries(branches).map(([name, width]) => [
          `node_modules/conditional/${name}.js`,
          `export const width = '${width}'`,
        ]),
      ),
    }
    for (const [name, contents] of Object.entries(files)) {
      mkdirSync(resolve(root, name, '..'), { recursive: true })
      writeFileSync(join(root, name), contents)
    }
    writeFileSync(
      join(root, 'check.ts'),
      `import { expect } from 'bun:test'; import { DevupUI, register } from ${JSON.stringify(entry)}; import { setModuleResolver } from ${JSON.stringify(wasm)};\n${script}`,
    )
    const result = Bun.spawnSync([process.execPath, 'check.ts'], {
      cwd: root,
      stdout: 'pipe',
      stderr: 'pipe',
    })
    expect({
      exitCode: result.exitCode,
      output: result.stdout.toString() + result.stderr.toString(),
    }).toEqual({ exitCode: 0, output: '' })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

const builds = [
  { config: {}, specifier: 'conditional', width: '823px' },
  { config: { target: 'browser' }, specifier: 'conditional', width: '823px' },
  { config: { target: 'bun' }, specifier: 'conditional', width: '827px' },
  { config: { target: 'node' }, specifier: 'conditional', width: '829px' },
  {
    config: { conditions: ['custom'] },
    specifier: 'conditional',
    width: '821px',
  },
  {
    config: { target: 'node', conditions: 'custom' },
    specifier: 'conditional',
    width: '821px',
  },
  {
    config: { target: 'bun' },
    specifier: 'conditional/node-first',
    width: '829px',
  },
  { config: {}, specifier: 'conditional/import-only', width: '839px' },
  { config: {}, specifier: 'conditional/module-first', width: '853px' },
  {
    config: { conditions: [] },
    specifier: 'conditional/module-first',
    width: '853px',
  },
  {
    config: { conditions: ['custom'] },
    specifier: 'conditional/module-first',
    width: '853px',
  },
  { config: {}, specifier: 'conditional/fallback', width: '857px' },
] as const

it.each([...builds])(
  'matches real Bun.build export selection for $specifier with $config',
  ({ config, specifier, width }) => {
    run(`
    const config = ${JSON.stringify(config)};
    await Bun.write('oracle.ts', "export { width } from '${specifier}'");
    const oracle = await Bun.build({ ...config, entrypoints: ['./oracle.ts'] });
    expect(oracle.success, String(oracle.logs)).toBe(true);
    const js = oracle.outputs.find(output => output.path.endsWith('.js'));
    const actual = await import('data:text/javascript;base64,' + Buffer.from(await js.text()).toString('base64'));
    expect(actual.width).toBe('${width}');
    // Vanilla Extract evaluates imports through its CommonJS evaluator, but
    // these requests originated as ESM imports, not source require() calls.
    await Bun.write('styled.ts', "import { style } from '@vanilla-extract/css'; import { width } from '${specifier}'; export const cls = style({ width });");
    // When: a later engine user replaces the shared resolver after setup.
    const result = await Bun.build({ ...config, entrypoints: ['./styled.ts'], plugins: [DevupUI(), { name: 'other-engine', setup() { setModuleResolver(() => ({ path: process.cwd() + '/wrong.js', code: "export const width = '863px'" })); } }] });
    expect(result.success, String(result.logs)).toBe(true);
    // Then: extracted compile-time CSS agrees with the actual bundler branch.
    const css = result.outputs.find(output => output.path.endsWith('.css'));
    expect(await css.text()).toContain(actual.width);
    expect(await css.text()).not.toContain('863px');
  `)
  },
)

it.each([
  'conditional',
  'conditional/node-first',
  'conditional/import-only',
  'conditional/module-first',
])(
  'matches native runtime imports after a foreign resolver override for %s',
  (specifier) => {
    run(`
    // Given: Bun runtime has bun/node conditions; the fixture also has browser.
    const actual = await import('${specifier}');
    expect(require('conditional/import-only').width).toBe('811px');
    await Bun.write('styled.ts', "import { css } from '@devup-ui/react'; import { width } from '${specifier}'; export const cls = css({ width });");
    await register();
    setModuleResolver(() => ({ path: process.cwd() + '/wrong.js', code: "export const width = '863px'" }));
    // When: the styling module is imported through the real runtime plugin.
    const { cls } = await import('./styled.ts');
    // Then: synchronous disk CSS agrees with native resolution, not the override.
    expect(cls).toBeTruthy();
    const css = await Bun.file('df/devup-ui/devup-ui.css').text();
    expect(css).toContain(actual.width);
    expect(css).not.toContain('863px');
  `)
  },
)
