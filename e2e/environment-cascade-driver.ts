import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const repo = fileURLToPath(new URL('../', import.meta.url))

export const cascadeModes = [
  { name: 'ordinary', atomHoist: undefined, singleCss: false },
  { name: 'singleCss', atomHoist: undefined, singleCss: true },
  { name: 'atomHoist', atomHoist: 2, singleCss: false },
] as const

export const cascadeOrders = [
  ['A', 'B'],
  ['B', 'A'],
] as const

type CompiledFile = {
  readonly source: string
  readonly code: string
  readonly cssFile: string
  readonly css: string
  readonly markup: string
}

export type CascadePair = {
  readonly A: CompiledFile
  readonly B: CompiledFile
  readonly shared: string
}

// Authored once, unchanged for both extraction orders and delivery orders.
const sources = {
  A: `export function View() { return <Box data-testid="A" flexDir={['column', 'row']} display={['none', null, null, 'flex']} />; }`,
  B: `export function View() { return <Box data-testid="B" flexDir="column" display="none" />; }`,
} as const

function parseFile(value: unknown): CompiledFile {
  if (
    typeof value !== 'object' ||
    value === null ||
    !('source' in value) ||
    typeof value.source !== 'string' ||
    !('code' in value) ||
    typeof value.code !== 'string' ||
    !('cssFile' in value) ||
    typeof value.cssFile !== 'string' ||
    !('css' in value) ||
    typeof value.css !== 'string' ||
    !('markup' in value) ||
    typeof value.markup !== 'string'
  )
    throw new TypeError('Invalid public-WASM cascade file receipt')
  return {
    source: value.source,
    code: value.code,
    cssFile: value.cssFile,
    css: value.css,
    markup: value.markup,
  }
}

export function compileCascadePair(
  mode: (typeof cascadeModes)[number],
  order: (typeof cascadeOrders)[number],
  seeded: boolean,
): CascadePair {
  const root = mkdtempSync(join(tmpdir(), 'devup-cascade-pair-'))
  try {
    const result: unknown = JSON.parse(
      execFileSync(
        'bun',
        [
          '--eval',
          String.raw`
import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
const require = createRequire(${JSON.stringify(join(repo, 'apps/landing/package.json'))});
const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');
const wasm = (await import(${JSON.stringify(pathToFileURL(resolve(repo, 'bindings/devup-ui-wasm/pkg/index.js')).href)})).default;
const filenames = { A: 'a.tsx', B: 'b.tsx' };
const authored = ${JSON.stringify(sources)};
wasm.resetBuildState();
wasm.setDebug(false);
wasm.setPrefix(undefined);
if (${seeded}) wasm.seedFileMap(Object.values(filenames));
wasm.setAtomHoist(${JSON.stringify(mode.atomHoist) ?? 'undefined'});
wasm.importFileRoutes({ 'a.tsx': [0, 1], 'b.tsx': [0, 1] });
const outputs = {};
try {
  for (const id of ${JSON.stringify(order)}) {
    const source = 'import React from ' + JSON.stringify(pathToFileURL(require.resolve('react')).href) + '; import { Box } from "@devup-ui/react"; ' + authored[id];
    const output = wasm.codeExtract(filenames[id], source, '@devup-ui/react', 'df/devup-ui', ${mode.singleCss}, false, false, {});
    outputs[id] = { output, source };
  }
  const transpiler = new Bun.Transpiler({ loader: 'tsx', tsconfig: { compilerOptions: { jsx: 'react', jsxFactory: 'React.createElement' } } });
  const files = {};
  for (const id of ['A', 'B']) {
    const { output, source } = outputs[id];
    // Preserve raw compiled JS in the receipt; remove only bundler CSS imports for SSR.
    const executable = output.code.replace(/import\s*["'][^"']+\.css(?:\?[^"']*)?["'];?/g, '');
    const modulePath = join(${JSON.stringify(root)}, id + '.mjs');
    writeFileSync(modulePath, transpiler.transformSync(executable));
    const { View } = await import(pathToFileURL(modulePath).href);
    const number = output.cssFile?.match(/\/devup-ui-(\d+)\.css$/);
    if (!${mode.singleCss} && !number) throw new TypeError('Missing public per-file CSS number');
    files[id] = { source, code: output.code, cssFile: output.cssFile ?? '',
      css: ${mode.singleCss} ? '' : wasm.getCss(Number(number[1]), false),
      markup: renderToStaticMarkup(React.createElement(View)) };
  }
  process.stdout.write(JSON.stringify({ ...files, shared: wasm.getCss(null, false) }));
} finally {
  for (const { output } of Object.values(outputs)) output.free();
}
`,
        ],
        { cwd: root, encoding: 'utf8', timeout: 30_000 },
      ),
    )
    if (
      typeof result !== 'object' ||
      result === null ||
      !('A' in result) ||
      !('B' in result) ||
      !('shared' in result) ||
      typeof result.shared !== 'string'
    )
      throw new TypeError('Invalid public-WASM cascade pair receipt')
    return {
      A: parseFile(result.A),
      B: parseFile(result.B),
      shared: result.shared,
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
