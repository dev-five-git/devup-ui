import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'
import {
  acceptPublication,
  auditLane,
  beginPublication,
  recordRoot,
  recordVite,
} from '../../../../test-harness/publication'

it('invalidates both stale lane outputs and the accepted marker before any lane starts', () => {
  const fixture = nativePublicationFixture()
  try {
    for (const lane of ['root', 'vite']) {
      mkdirSync(join(fixture.root, 'coverage', lane), { recursive: true })
      writeFileSync(join(fixture.root, 'coverage', lane, 'lcov.info'), 'stale')
    }
    writeFileSync(
      join(fixture.root, 'coverage/accepted.json'),
      'stale accepted',
    )
    beginPublication(fixture.root)
    for (const path of ['root/lcov.info', 'vite/lcov.info', 'accepted.json'])
      expect(existsSync(join(fixture.root, 'coverage', path))).toBe(false)
    expect(() =>
      acceptPublication(fixture.root, { root: [], vite: [] }),
    ).toThrow()
  } finally {
    fixture.dispose()
  }
})

it.each(['bunfig.toml', 'source.ts', 'package.json', 'tsconfig.json'])(
  'rejects current %s drift instead of accepting a previous native measurement',
  async (file) => {
    const fixture = nativePublicationFixture()
    try {
      const actual = file
      if (file === 'tsconfig.json')
        writeFileSync(
          join(fixture.root, actual),
          '{"compilerOptions":{"strict":true}}',
        )
      beginPublication(fixture.root)
      expect((await fixture.lane('root')).code).toBe(0)
      console.info(
        JSON.stringify({
          event: 'native-drift-control',
          file,
          raw: readFileSync(
            join(fixture.root, 'coverage/root/lcov.info'),
            'utf8',
          ),
        }),
      )
      writeFileSync(
        join(fixture.root, actual),
        readFileSync(join(fixture.root, actual), 'utf8') + '\n',
      )
      expect(() => recordRoot(fixture.root, ['source.ts'])).toThrow(
        'identity changed',
      )
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)

it.each(['root', 'vite'] as const)(
  'refuses missing source and a replaced %s report after native success',
  async (lane) => {
    const fixture = nativePublicationFixture()
    try {
      beginPublication(fixture.root)
      expect((await fixture.lane('root')).code).toBe(0)
      expect(() => auditLane(fixture.root, 'root', ['missing.ts'])).toThrow(
        'missing source missing.ts',
      )
      recordRoot(fixture.root, ['source.ts'])
      expect((await fixture.lane('vite')).code).toBe(0)
      recordVite(fixture.root, ['source.ts'])
      const report = join(fixture.root, 'coverage', lane, 'lcov.info')
      writeFileSync(report, readFileSync(report, 'utf8') + '\n')
      expect(() =>
        acceptPublication(fixture.root, {
          root: ['source.ts'],
          vite: ['source.ts'],
        }),
      ).toThrow(`${lane} report changed`)
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)

it.each([false, true])(
  'rejects divergent zero-hit universes in both native lane orders reversed=%s',
  async (reversed) => {
    const fixture = nativePublicationFixture()
    try {
      writeFileSync(
        join(fixture.root, 'source.ts'),
        'export function value(){\n return 42\n}\nexport function other(){\n return 2\n}\n',
      )
      for (const [file, first] of [
        ['source.test.ts', !reversed],
        ['vite.test.ts', reversed],
      ] as const) {
        writeFileSync(
          join(fixture.root, file),
          `import {it,expect} from 'bun:test';import {${first ? 'value' : 'other'}} from './source';it('partial',()=>expect(${first ? 'value' : 'other'}()).toBe(${first ? 42 : 2}))`,
        )
      }
      beginPublication(fixture.root)
      for (const lane of ['root', 'vite'] as const) {
        const result = await fixture.lane(lane)
        expect(result.code).toBe(1)
        expect(result.stderr).toMatch(/^\s*1 pass$/m)
        expect(result.stderr).toMatch(/^\s*0 fail$/m)
        expect(() => auditLane(fixture.root, lane, ['source.ts'])).toThrow(
          'below 100%',
        )
      }
      expect(() =>
        acceptPublication(fixture.root, {
          root: ['source.ts'],
          vite: ['source.ts'],
        }),
      ).toThrow('below 100%')
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)

it.each(['root', 'vite'] as const)(
  'refuses partial %s independently even beside a complete sibling with the same DA keys',
  async (partial) => {
    const fixture = nativePublicationFixture()
    try {
      writeFileSync(
        join(fixture.root, 'source.ts'),
        'export function value(){return 42};export function other(){return 2}\n',
      )
      writeFileSync(
        join(
          fixture.root,
          partial === 'root' ? 'vite.test.ts' : 'source.test.ts',
        ),
        "import {it,expect} from 'bun:test';import {value,other} from './source';it('both',()=>{expect(value()).toBe(42);expect(other()).toBe(2)})",
      )
      beginPublication(fixture.root)
      for (const lane of ['root', 'vite'] as const) {
        const result = await fixture.lane(lane)
        expect(result.code).toBe(lane === partial ? 1 : 0)
        expect(result.stderr).toMatch(/^\s*1 pass$/m)
        expect(result.stderr).toMatch(/^\s*0 fail$/m)
      }
      expect(() => auditLane(fixture.root, partial, ['source.ts'])).toThrow(
        'below 100%',
      )
      expect(() =>
        auditLane(fixture.root, partial === 'root' ? 'vite' : 'root', [
          'source.ts',
        ]),
      ).not.toThrow()
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
