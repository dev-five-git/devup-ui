import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

export function builtPluginEntry(packageName: string, entry = 'index.mjs') {
  return JSON.stringify(
    pathToFileURL(
      resolve(import.meta.dir, '../../../', packageName, 'dist', entry),
    ).href,
  )
}

// A fresh process keeps bun:test's package mocks and WASM globals out of the
// integration. The plugin, not the harness, configures route reach and hoisting.
export function runAtomHoistIntegration(setup: string) {
  const root = mkdtempSync(join(tmpdir(), 'devup-atom-hoist-'))
  const run = (reverse: boolean) =>
    execFileSync(
      process.execPath,
      [
        '--eval',
        String.raw`
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
const root = ${JSON.stringify(root)};
const original = process.cwd();
const sources = {
  'src/shared.tsx': 'import { Box } from "@devup-ui/react"; export const Shared = () => <Box color="red" p={4} />;',
  'src/app/a/page.tsx': 'import { Box } from "@devup-ui/react"; import { Shared } from "../../shared"; export default function A() { return <><Shared /><Box opacity={0.25} /></>; }',
  'src/app/b/page.tsx': 'import { Box } from "@devup-ui/react"; import { Shared } from "../../shared"; export default function B() { return <><Shared /><Box opacity={0.75} /></>; }',
};
const files = Object.keys(sources);
const cssDir = join(root, 'df/devup-ui');
rmSync(join(root, 'df'), { recursive: true, force: true });
for (const [file, code] of Object.entries(sources)) {
  mkdirSync(dirname(join(root, file)), { recursive: true });
  writeFileSync(join(root, file), code);
}
mkdirSync(cssDir, { recursive: true });
writeFileSync(join(root, 'devup.json'), '{}');
writeFileSync(join(root, 'tsconfig.json'), '{}');
writeFileSync(join(root, 'package.json'), '{"name":"atom-hoist-fixture","private":true}');
process.chdir(root);
try {
${setup}
  const outputs = {};
  for (const file of ${reverse} ? [...files].reverse() : files) {
    outputs[file] = await transform(file, sources[file]);
    if (file === 'src/shared.tsx') {
      const writtenBase = readFileSync(join(cssDir, 'devup-ui.css'), 'utf8');
      assert.match(writtenBase, /color:red(?:;|})/, 'shared CSS dirty signal must reach disk');
      assert.match(writtenBase, /padding:16px(?:;|})/);
    }
  }
  const base = await loadCss('devup-ui.css');
  const sheets = {};
  const classes = code => [...code.matchAll(/className\s*=\s*(?:\{\s*)?["']([^"']+)["']/g)]
    .flatMap(match => match[1].trim().split(/\s+/));
  const declarations = css => [...css.matchAll(/\.([^\s.{:]+)\{([^{}]+)\}/g)]
    .map(match => [match[1], match[2].replace(/;$/, '')]);
  const sharedClasses = classes(outputs['src/shared.tsx']);
  assert.equal(sharedClasses.length, 2, 'shared JSX must really be transformed');
  for (const name of sharedClasses) {
    assert.equal(declarations(base).filter(([key]) => key === name).length, 1,
      'shared atom must be defined exactly once');
  }
  assert.deepEqual(declarations(base).map(([, value]) => value).sort(), ['color:red', 'padding:16px']);
  for (const file of files) {
    const code = outputs[file];
    const imports = [...code.matchAll(/["']([^"']*devup-ui(?:-\d+)?\.css(?:\?fileNum=\d+)?)["']/g)]
      .map(match => match[1].split('/').pop());
    assert.ok(imports.length > 0, 'transformed JSX must import its CSS');
    const loaded = [];
    for (const name of imports) {
      sheets[name] = await loadCss(name);
      if (name !== 'devup-ui.css') {
        assert.ok(declarations(sheets[name]).every(([, value]) =>
          value !== 'color:red' && value !== 'padding:16px'),
          'hoisted declarations must not be duplicated in local sheets');
      }
      loaded.push(sheets[name]);
    }
    const local = loaded.join('\n');
    const expected = file === 'src/shared.tsx' ? ['color:red', 'padding:16px']
      : [file.includes('/a/') ? 'opacity:.25' : 'opacity:.75'];
    const generated = classes(code);
    assert.equal(generated.length, expected.length);
    const values = generated.flatMap(name => {
      const matches = declarations(base + '\n' + local).filter(([key]) => key === name);
      assert.ok(matches.length > 0, 'every generated JSX class must exist in loaded CSS: ' + name);
      return [...new Set(matches.map(([, value]) => value))];
    });
    assert.deepEqual(values.sort(), expected.sort());
    if (file !== 'src/shared.tsx') {
      assert.ok(generated.every(name => !sharedClasses.includes(name)));
      assert.ok(declarations(local).some(([, value]) => value === expected[0]), 'private styles stay local');
    }
  }
  const clean = css => css.replace(/\/\*[\s\S]*?\*\//g, '').trim();
  console.log(JSON.stringify({
    classes: files.map(file => classes(outputs[file])),
    base: clean(base),
    sheets: Object.fromEntries(Object.entries(sheets).sort().map(([name, css]) => [name, clean(css)])),
  }));
  await finish();
} finally {
  process.chdir(original);
}
process.exit(0);
`,
      ],
      { encoding: 'utf8', timeout: 30_000, env: process.env },
    ).trim()
  try {
    return [run(false), run(true), run(false)] as const
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
