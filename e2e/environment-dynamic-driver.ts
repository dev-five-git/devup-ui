import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const repo = fileURLToPath(new URL('../', import.meta.url))

export const dynamicModes = [
  { atomHoist: undefined, singleCss: false },
  { atomHoist: undefined, singleCss: true },
  { atomHoist: 2, singleCss: false },
  { atomHoist: 2, singleCss: true },
] as const

export function renderDynamicSite(
  source: string,
  render: string,
  mode: (typeof dynamicModes)[number],
): string {
  const root = mkdtempSync(join(tmpdir(), 'devup-dynamic-sites-'))
  try {
    return execFileSync(
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
const filename = 'src/view.tsx';
const source = 'import React from ' + JSON.stringify(pathToFileURL(require.resolve('react')).href) + '; import { Box } from "@devup-ui/react"; ' + ${JSON.stringify(source)};
wasm.setDebug(false);
wasm.seedFileMap([filename]);
wasm.setAtomHoist(${JSON.stringify(mode.atomHoist) ?? 'undefined'});
wasm.importFileRoutes({ [filename]: [0, 1] });
const output = wasm.codeExtract(filename, source, '@devup-ui/react', 'df/devup-ui', ${mode.singleCss}, false, false, {});
try {
  const css = wasm.getCss(null, false) + (${mode.singleCss} ? '' : '\n' + (output.css ?? ''));
  // Only bundler-consumed CSS imports are removed; generated JSX executes intact.
  const code = output.code.replace(/import\s*["'][^"']+\.css(?:\?[^"']*)?["'];?/g, '');
  const transpiler = new Bun.Transpiler({ loader: 'tsx', tsconfig: { compilerOptions: { jsx: 'react', jsxFactory: 'React.createElement' } } });
  const modulePath = join(${JSON.stringify(root)}, 'view.mjs');
  writeFileSync(modulePath, transpiler.transformSync(code));
  const { View } = await import(pathToFileURL(modulePath).href);
  const markup = renderToStaticMarkup(${render});
  process.stdout.write('<!doctype html><html><head><style>' + css + '</style></head><body>' + markup + '</body></html>');
} finally {
  output.free();
}
`,
      ],
      { cwd: root, encoding: 'utf8', timeout: 30_000 },
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
