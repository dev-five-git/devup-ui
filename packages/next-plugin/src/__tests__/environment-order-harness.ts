import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

export const environmentMatrix = [
  { atomHoist: undefined, singleCss: false },
  { atomHoist: undefined, singleCss: true },
  { atomHoist: 2, singleCss: false },
  { atomHoist: 2, singleCss: true },
] as const

// Keep byte capture separate from the older hoisting harness, which deliberately
// compares only classes and strips headers. Each run here owns a fresh Node/WASM.
export function runEnvironmentOrder(
  setup: string,
  environments: readonly string[],
) {
  const root = mkdtempSync(join(tmpdir(), 'devup-environment-order-'))
  const wasmEntry = pathToFileURL(
    resolve(import.meta.dir, '../../../../bindings/devup-ui-wasm/pkg/index.js'),
  ).href
  const run = (order: readonly string[]) =>
    execFileSync(
      'node',
      [
        '--input-type=module',
        '--eval',
        String.raw`
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync, readFileSync, rmSync, existsSync, symlinkSync } from 'node:fs';
import { dirname, join, resolve, basename } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { runInNewContext } from 'node:vm';
const root = ${JSON.stringify(root)};
const wasm = (await import(${JSON.stringify(wasmEntry)})).default;
const cssDir = join(root, 'df/devup-ui');
rmSync(join(root, 'df'), { recursive: true, force: true });
const source = 'import { Box, keyframes } from "@devup-ui/react"; import { tone } from "conditional-theme"; import { shades } from "./bridge"; const spin = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } }); export const Shared = ({ width }) => <Box color={tone} bg={shades} p={4} w={width} animation={spin + " 1s linear"} />;';
const sources = {
  'src/shared.tsx': 'import { Box } from "@devup-ui/react"; export const Shared = () => null;',
  'src/bridge.ts': 'export { shades } from "conditional-theme";',
  'src/app/a/page.tsx': 'import { Shared } from "../../shared"; export default Shared;',
  'src/app/b/page.tsx': 'import { Shared } from "../../shared"; export default Shared;',
};
for (const [file, code] of Object.entries(sources)) {
  mkdirSync(dirname(join(root, file)), { recursive: true });
  writeFileSync(join(root, file), code);
}
mkdirSync(cssDir, { recursive: true });
mkdirSync(join(root, 'node_modules/conditional-theme'), { recursive: true });
writeFileSync(join(root, 'devup.json'), '{}');
writeFileSync(join(root, 'tsconfig.json'), '{}');
writeFileSync(join(root, 'package.json'), '{"name":"environment-order-fixture","private":true}');
process.chdir(root);
const file = 'src/shared.tsx';
const files = Object.keys(sources);
const variants = {
  client: { tone: 'red', shades: ['blue'] },
  web: { tone: 'red', shades: ['blue'] },
  server: { tone: 'green', shades: ['yellow', 'purple'] },
  node: { tone: 'green', shades: ['yellow', 'purple'] },
  rsc: { tone: 'orange', shades: ['pink', 'cyan', 'navy'] },
};
const selectEnvironment = name => {
  const variant = variants[name];
  const code = 'export const tone = ' + JSON.stringify(variant.tone) + '; export const shades = ' + JSON.stringify(variant.shades) + ';';
  // The webpack loader installs its own resolver. Its real package reader sees
  // the selected conditional entry; other adapters use the real WASM callback.
  writeFileSync(join(root, 'node_modules/conditional-theme', name + '.js'), code);
  writeFileSync(join(root, 'node_modules/conditional-theme/package.json'), JSON.stringify({ name: 'conditional-theme', exports: { '.': { import: './' + name + '.js', default: './' + name + '.js' } } }));
  const reads = [];
  wasm.setModuleResolver((specifier, importer) => {
    reads.push([specifier, importer]);
    if (specifier === 'conditional-theme') return { path: 'node_modules/conditional-theme/' + name + '.js', code };
    if (specifier === './bridge') return { path: 'src/bridge.ts', code: sources['src/bridge.ts'] };
  });
  return reads;
};
const invoke = (loader, options, resourcePath, code, resourceQuery = '') => new Promise((resolve, reject) => {
  const callback = (error, code, map) => error ? reject(error) : resolve({ code, map: map ?? null });
  loader.call({ resourcePath, resourceQuery, getOptions: () => options, addDependency() {}, cacheable() {}, callback, async: () => callback }, code);
});
${setup}
const outputs = {};
for (const name of ${JSON.stringify(order)}) {
  const reads = selectEnvironment(name);
  outputs[name] = await transform(name, source);
  assert.equal(typeof outputs[name].code, 'string');
  if (usesCallback) {
    assert.ok(reads.some(([specifier]) => specifier === './bridge'), 'relative re-export must be evaluated');
    assert.ok(reads.filter(([specifier]) => specifier === 'conditional-theme').length >= 2, 'bare and transitive constants must both resolve');
  }
}
const names = new Set(['devup-ui.css']);
for (const { code } of Object.values(outputs)) {
  for (const match of code.matchAll(/["']([^"']*devup-ui(?:-\d+)?\.css(?:\?fileNum=\d+)?)["']/g)) names.add(match[1].split('/').pop());
}
const sheets = {};
for (const name of [...names].sort()) sheets[name] = await loadCss(name);
const css = Object.values(sheets).join('\n');
const declarations = [...css.matchAll(/\.([^\s.{:]+)\{([^{}]+)\}/g)].map(match => [match[1], match[2]]);
const React = createRequire(new URL('../../../apps/landing/package.json', ${JSON.stringify(wasmEntry)}))('react');
const renderedProps = code => {
  const program = JSON.parse(execFileSync('bun', ['--eval',
    'const code=JSON.parse(await Bun.stdin.text());const transpiler=new Bun.Transpiler({loader:"tsx",tsconfig:{compilerOptions:{jsx:"react",jsxFactory:"React.createElement"}}});process.stdout.write(JSON.stringify(transpiler.transformSync(code)));'
  ], { encoding: 'utf8', timeout: 30000, input: JSON.stringify(code
    .replace(/import\s*(?:[^;]*?\s+from\s*)?["'][^"']+["'];?/g, '')
    .replace(/\bexport\s+(?=(?:const|let|var|function|class)\b)/g, '')) }));
  const sandbox = { React };
  runInNewContext(program + '\nglobalThis.element=Shared({width:"13px"});', sandbox, { timeout: 1000 });
  assert.ok(React.isValidElement(sandbox.element), 'compiled component must render a real React element');
  return sandbox.element.props;
};
for (const [name, output] of Object.entries(outputs)) {
  const props = renderedProps(output.code);
  const classes = String(props.className ?? '').trim().split(/\s+/).filter(Boolean);
  assert.ok(classes.length >= 5, 'JSX must really be extracted');
  const values = classes.flatMap(key => {
    const rules = declarations.filter(([candidate]) => candidate === key);
    assert.ok(rules.length > 0, 'missing class definition: ' + key);
    return rules.map(([, value]) => value);
  }).join('\n');
  assert.ok(values.includes('color:' + variants[name].tone), 'wrong conditional bare constant: ' + name);
  for (const shade of variants[name].shades) assert.ok(values.includes('background:' + shade), 'wrong transitive constant: ' + name + '/' + shade);
  assert.match(values, /padding:16px/);
  assert.match(values, /width:var\(--[^)]+\)/);
  assert.match(output.code, /width/);
  assert.match(values, /animation:var\(--[^)]+\)/);
  const keyframe = css.match(/@keyframes ([^\s{]+)\{/);
  assert.ok(keyframe, 'missing keyframes definition');
  assert.ok(output.code.includes('"' + keyframe[1] + '"'), 'JS keyframes name must match the CSS definition');
  for (const [, property, variable] of values.matchAll(/(width|animation):var\((--[^)]+)\)/g)) {
    assert.equal(props.style[variable], property === 'width' ? '13px' : keyframe[1] + ' 1s linear');
  }
  if (output.map !== null && output.map !== undefined) {
    const map = typeof output.map === 'string' ? JSON.parse(output.map) : output.map;
    assert.ok(map.sourcesContent.includes(source), 'source map must retain original source');
  }
}
await finish();
console.log(JSON.stringify({ outputs: Object.fromEntries(Object.entries(outputs).sort()), sheets }));
process.exit(0);
`,
      ],
      { encoding: 'utf8', timeout: 30_000, env: process.env },
    ).trim()
  try {
    return [
      run(environments),
      run([...environments].reverse()),
      run(environments),
    ] as const
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
