import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const entry = resolve(import.meta.dir, '../src/register.ts').replaceAll(
  '\\',
  '/',
)
const scratch = join(tmpdir(), 'opencode', 'workers', 'w20-plugins-core', 'bun')
mkdirSync(scratch, { recursive: true })

function run(
  script: string,
  files: Readonly<Record<string, string>>,
  compiler = false,
) {
  const root = realpathSync.native(mkdtempSync(join(scratch, 'modern-')))
  try {
    writeFileSync(join(root, 'bunfig.toml'), '')
    if (compiler) {
      const host = createRequire(
        resolve(import.meta.dir, '../../../apps/landing/package.json'),
      )
      const compilerPath = createRequire(
        host.resolve('@mdx-js/loader'),
      ).resolve('@mdx-js/mdx')
      mkdirSync(join(root, 'node_modules/@mdx-js'), { recursive: true })
      symlinkSync(
        dirname(compilerPath),
        join(root, 'node_modules/@mdx-js/mdx'),
        'junction',
      )
    }
    for (const [name, contents] of Object.entries(files)) {
      mkdirSync(resolve(root, name, '..'), { recursive: true })
      writeFileSync(join(root, name), contents)
    }
    writeFileSync(
      join(root, 'check.ts'),
      `import { expect } from 'bun:test'; import { DevupUI, register } from ${JSON.stringify(entry)};\n${script}`,
    )
    const result = Bun.spawnSync([process.execPath, 'check.ts'], {
      cwd: root,
      stdout: 'pipe',
      stderr: 'pipe',
    })
    expect(
      result.exitCode,
      result.stdout.toString() + result.stderr.toString(),
    ).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it.each(['ts', 'tsx', 'mts', 'cts', 'js', 'jsx', 'mjs', 'cjs'])(
  'extracts real runtime and bundled source when its extension is %s',
  (extension) => {
    run(
      `
    await register();
    const { cls } = await import('./local.${extension}');
    expect(cls).toBeTruthy();
    const disk = await Bun.file('df/devup-ui/devup-ui.css').text();
    expect(disk).toContain('width:719px');
    expect(cls).not.toContain('width');
    const result = await Bun.build({ entrypoints: ['./local.${extension}'], plugins: [DevupUI()] });
    expect(result.success, String(result.logs)).toBe(true);
    const css = result.outputs.find(output => output.path.endsWith('.css'));
    expect(await css.text()).toContain('719px');
  `,
      {
        [`local.${extension}`]:
          "import { css } from '@devup-ui/react'; export const cls = css({ width: '719px' })",
      },
    )
  },
)

it('uses project paths when invoked from a parent directory', () => {
  run(
    `
    const result = await Bun.build({ root: './project', entrypoints: ['./project/app/page.ts'], plugins: [DevupUI({ distDir: 'generated' })] });
    expect(result.success, String(result.logs)).toBe(true);
    expect(await Bun.file('project/generated/theme.d.ts').exists()).toBe(true);
    expect(await Bun.file('generated/theme.d.ts').exists()).toBe(false);
    const css = result.outputs.find(output => output.path.endsWith('.css'));
    expect(await css.text()).toContain('#abcdef');
  `,
    {
      'project/devup.json':
        '{"theme":{"colors":{"default":{"primary":"#abcdef"}}}}',
      'project/app/page.ts':
        "import { css } from '@devup-ui/react'; export const cls = css({ width: '731px' })",
    },
  )
})

it.each(['{', '{"extends":["./devup.json"]}'])(
  'fails initialization with a located diagnostic when configuration is invalid: %s',
  (config) => {
    run(
      `
    let failure;
    try { await Bun.build({ entrypoints: ['./entry.ts'], plugins: [DevupUI()] }); } catch (cause) { failure = String(cause); }
    expect(failure).toContain('devup.json');
  `,
      { 'devup.json': config, 'entry.ts': 'export const value = 1' },
    )
  },
)

it('reports styling MDX without a compiler instead of extracting raw syntax', () => {
  run(
    `
    let failure;
    try { await Bun.build({ entrypoints: ['./page.mdx'], plugins: [DevupUI()] }); } catch (cause) { failure = cause instanceof AggregateError ? cause.errors.map(String).join('\\n') : String(cause); }
    expect(failure).toContain('page.mdx:1:1');
    expect(failure).toContain('@mdx-js/mdx');
  `,
    {
      'node_modules/@mdx-js/mdx/package.json': '{"exports":"./missing.js"}',
      'page.mdx':
        "import { Box } from '@devup-ui/react'\n\n# heading\n\n<Box bg='red' />",
    },
  )
})

it('compiles real MDX before extraction using the project compiler', () => {
  run(
    `
    const result = await Bun.build({ entrypoints: ['./page.mdx'], external: ['react', 'react/jsx-runtime', 'react/jsx-dev-runtime'], plugins: [DevupUI()] });
    expect(result.success, String(result.logs)).toBe(true);
    const css = result.outputs.find(output => output.path.endsWith('.css'));
    expect(await css.text()).toContain('743px');
    const js = result.outputs.find(output => output.path.endsWith('.js'));
    const code = await js.text();
    expect(code).toContain('heading');
    expect(code).not.toContain('@devup-ui/react');
  `,
    {
      'page.mdx':
        "import { Box } from '@devup-ui/react'\n\n# heading\n\n<Box w='743px' />",
    },
    true,
  )
})

it('labels compiled MDX locations when extraction fails without a compiler source map', () => {
  run(
    `
    let failure;
    try { await Bun.build({ entrypoints: ['./page.mdx'], external: ['react', 'react/jsx-runtime', 'react/jsx-dev-runtime'], plugins: [DevupUI()] }); }
    catch (cause) { failure = cause instanceof AggregateError ? cause.errors.map(String).join('\\n') : String(cause); }
    expect(failure).toContain('page.mdx:');
    expect(failure).toContain('(in compiled MDX)');
  `,
    {
      'page.mdx':
        "import { css } from '@devup-ui/react'\n\nexport const cls = css({ width: unknownWidth })\n\n# heading",
    },
    true,
  )
})

it('restores explicitly configured readable names when another engine user changes debug', () => {
  const wasm = resolve(
    import.meta.dir,
    '../../../bindings/devup-ui-wasm/pkg/index.js',
  ).replaceAll('\\', '/')
  run(
    `
    await register({ debug: true });
    const { setDebug } = await import(${JSON.stringify(wasm)});
    setDebug(false);
    const { cls } = await import('./local.ts');
    expect(cls).toContain('width');
  `,
    {
      'local.ts':
        "import { css } from '@devup-ui/react'; export const cls = css({ width: '761px' })",
    },
  )
})

it('transforms an included uncompiled runtime library', () => {
  run(
    `
    await register({ include: ['library'] });
    const { cls } = await import('library');
    expect(cls).toBeTruthy();
    expect(await Bun.file('df/devup-ui/devup-ui.css').text()).toContain('width:769px');
  `,
    {
      'node_modules/library/package.json':
        '{"type":"module","exports":"./index.mjs"}',
      'node_modules/library/index.mjs':
        "import { css } from '@devup-ui/react'; export const cls = css({ width: '769px' })",
    },
  )
})
