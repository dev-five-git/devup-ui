import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { readJsxImportSource } from '../jsx-import-source'

describe('readJsxImportSource', () => {
  let cwd: string

  function write(path: string, content: string) {
    mkdirSync(dirname(join(cwd, path)), { recursive: true })
    writeFileSync(join(cwd, path), content)
  }

  beforeEach(() => {
    cwd = mkdtempSync(join(tmpdir(), 'devup-ui-jsx-import-source-'))
  })

  afterEach(() => {
    rmSync(cwd, { recursive: true, force: true })
  })

  it('reads nothing without a config', () => {
    expect(readJsxImportSource(cwd)).toBeUndefined()
  })

  it('reads the default directory when none is given', () => {
    expect(readJsxImportSource()).toBe(readJsxImportSource(process.cwd()))
  })

  it('reads tsconfig, with the comments and trailing commas it allows', () => {
    write(
      'tsconfig.json',
      `{
  // the JSX runtime
  "compilerOptions": {
    /* Emotion's */ "jsxImportSource": "@emotion/react",
    "paths": { "a\\"b": ["./src/*"], },
  },
}`,
    )
    write(
      'jsconfig.json',
      '{ "compilerOptions": { "jsxImportSource": "preact" } }',
    )
    expect(readJsxImportSource(cwd)).toBe('@emotion/react')
  })

  it('reads jsconfig when there is no tsconfig', () => {
    write(
      'jsconfig.json',
      '{ "compilerOptions": { "jsxImportSource": "preact" } }',
    )
    expect(readJsxImportSource(cwd)).toBe('preact')
  })

  it('follows extends, a later entry overriding an earlier one', () => {
    write(
      'tsconfig.json',
      '{ "extends": ["./base/first", "@acme/config", "./base/second.json"], "compilerOptions": { "jsxImportSource": 1 } }',
    )
    write(
      'base/first.json',
      '{ "compilerOptions": { "jsxImportSource": "first" } }',
    )
    write('base/second.json', '{ "compilerOptions": {} }')
    write(
      'node_modules/@acme/config/tsconfig.json',
      '{ "compilerOptions": { "jsxImportSource": "@emotion/react" } }',
    )
    expect(readJsxImportSource(cwd)).toBe('@emotion/react')
  })

  it("follows a package's main config", () => {
    write('tsconfig.json', '{ "extends": "@acme/main" }')
    write(
      'node_modules/@acme/main/package.json',
      '{ "name": "@acme/main", "main": "base.json" }',
    )
    write(
      'node_modules/@acme/main/base.json',
      '{ "compilerOptions": { "jsxImportSource": "@emotion/react" } }',
    )
    expect(readJsxImportSource(cwd)).toBe('@emotion/react')
  })

  it('follows project references to a file or a directory', () => {
    write(
      'tsconfig.json',
      '{ "files": [], "extends": ["@acme/missing", "./missing"], "references": [1, { "path": 2 }, { "path": "./gone" }, { "path": "./tsconfig.node.json" }, { "path": "./app" }] }',
    )
    write('tsconfig.node.json', '{ "compilerOptions": {} }')
    write(
      'app/tsconfig.json',
      '{ "compilerOptions": { "jsxImportSource": "@emotion/react" } }',
    )
    expect(readJsxImportSource(cwd)).toBe('@emotion/react')
  })

  it('stops at configs extending each other', () => {
    write('tsconfig.json', '{ "extends": "./other.json" }')
    write('other.json', '{ "extends": "./tsconfig.json" }')
    expect(readJsxImportSource(cwd)).toBeUndefined()
  })

  it('reads nothing from a config it cannot parse', () => {
    write('tsconfig.json', '{ "compilerOptions": /* unterminated')
    expect(readJsxImportSource(cwd)).toBeUndefined()
    write('tsconfig.json', '{ "extends": "./base" } // last line')
    write('base', '{ "compilerOptions": { "jsxImportSource": "base" } }')
    expect(readJsxImportSource(cwd)).toBe('base')
    write('tsconfig.json', '{ "compilerOptions": [1, 2,] }')
    expect(readJsxImportSource(cwd)).toBeUndefined()
    write('tsconfig.json', '{ "compilerOptions": {} },')
    expect(readJsxImportSource(cwd)).toBeUndefined()
  })
})
