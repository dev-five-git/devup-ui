import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

type Sheets = Record<string, string>

function build(shells: 'on' | 'off', layout: 'root' | 'none' = 'root'): Sheets {
  const fixture = mkdtempSync(join(tmpdir(), 'devup-route-shells-'))
  try {
    const child = Bun.spawnSync(
      [
        'node',
        join(import.meta.dir, 'route-shells-build.cjs'),
        fixture,
        shells,
        layout,
      ],
      { stdout: 'pipe', stderr: 'pipe', timeout: 45000 },
    )
    assert.equal(
      child.exitCode,
      0,
      child.stdout.toString() + child.stderr.toString(),
    )
    return JSON.parse(child.stdout.toString()) as Sheets
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}

/** The sheet whose own rules (after the shared import) hold `needle` */
function sheetWith(sheets: Sheets, needle: string): string | undefined {
  return Object.keys(sheets).find((name) => sheets[name]!.includes(needle))
}

it('hoists what a layout imports into the shared sheet, in cascade order, when shells count with their routes', () => {
  // Given: a root layout entry importing a header, and three page entries
  // When: real webpack builds with and without route shells
  const off = build('off')
  const on = build('on')
  const again = build('on')

  // Then: as its own route, the layout's header kept per-file names
  expect(sheetWith(off, 'color:orange')).not.toBe('devup-ui.css')
  expect(off['devup-ui.css']).not.toContain('color:white')
  // ...counted with the pages, it joins the shared sheet with short names
  const shared = on['devup-ui.css']!
  expect(shared).toMatch(/\.[a-z]+\{color:white\}/)
  expect(shared).toMatch(/\.[a-z]+:hover\{color:orange\}/)
  // Base rules, then the breakpoint, then the selector
  const base = shared.indexOf('{color:white}')
  const media = shared.indexOf('@media(min-width:768px){')
  const hover = shared.indexOf(':hover{color:orange}')
  expect(base).toBeGreaterThan(-1)
  expect(media).toBeGreaterThan(base)
  expect(shared.indexOf('{font-size:72px}')).toBeGreaterThan(media)
  expect(hover).toBeGreaterThan(media)
  // The header's own sheet keeps only the shared import, ahead of nothing
  const headerSheet = sheetWith(off, 'color:orange')!
  expect(on[headerSheet]).toMatch(/@import "\.\/devup-ui\.css";$/)
  // Every other per-file sheet is unchanged and still imports the shared one first
  for (const name of Object.keys(off))
    if (name !== headerSheet && name !== 'devup-ui.css') {
      expect(on[name]).toBe(off[name])
      expect(on[name]).toContain('@import "./devup-ui.css";')
    }
  // Deterministic
  expect(again).toEqual(on)
}, 120000)

it('leaves a build without route shells byte-identical', () => {
  expect(build('on', 'none')).toEqual(build('off', 'none'))
}, 120000)
