import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { resetMarkerProcessState } from '@devup-ui/plugin-utils'
import { afterEach, beforeEach, expect, it, mock } from 'bun:test'

import { createViteMarkers } from '../component-markers'

let root: string
beforeEach(() => {
  resetMarkerProcessState()
  root = mkdtempSync(join(tmpdir(), 'devup-vite-markers-'))
})
afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

const silent = { warn: mock() }

it('drops the markers no module selects from each chunk, with a map', () => {
  // Given
  const markers = createViteMarkers()
  markers.reset()
  markers.scanned(['a--b'], ['/src/A.tsx'])
  markers.record('/src/A.tsx', '', { givenMarkers: ['a--a', 'a--b', 'a--c'] })
  markers.record('C:\\src\\B.tsx', '', {})
  const chunk = {
    modules: {
      '/src/A.tsx': { code: 'unread' },
      'C:\\src\\B.tsx': { code: 'f("a--c")' },
      '/vendor.js': { code: 'f("a--c")' },
      '/empty.js': { code: null },
    },
  }
  // When
  const result = markers.renderChunk(
    'f("a--a x"),f("a--b"),f("a--c")',
    chunk,
    silent,
  )
  const untouched = markers.renderChunk('f("y")', chunk, silent)
  // Then
  expect(result?.code).toBe('f("x"),f("a--b"),f("a--c")')
  expect(result?.map.version).toBe(3)
  expect(untouched).toBeNull()
  expect(silent.warn).not.toHaveBeenCalled()
})

it('keeps every marker and warns once when the scan missed a selecting module', () => {
  // Given
  const markers = createViteMarkers()
  markers.scanned([], [])
  writeFileSync(join(root, 'Late.tsx'), 'on disk')
  markers.record(join(root, 'Late.tsx'), 'on disk', {
    givenMarkers: [],
    selectedMarkers: ['a--a'],
  })
  markers.record(join(root, 'Gone.tsx'), 'x', { selectedMarkers: ['a--b'] })
  const context = { warn: mock() }
  // When
  const first = markers.renderChunk('f("a--a")', { modules: {} }, context)
  const second = markers.renderChunk('f("a--a")', { modules: {} }, context)
  // Then
  expect([first, second]).toEqual([null, null])
  expect(context.warn).toHaveBeenCalledTimes(1)
  const [warning] = context.warn.mock.calls[0] as [string]
  expect(warning).toContain('Late.tsx [outside-scan]')
  expect(warning).toContain('Gone.tsx [virtual-module]')
})

it('keeps every marker quietly without a scan', () => {
  // Given
  const markers = createViteMarkers()
  markers.record('/a.tsx', '', { givenMarkers: ['a--a'] })
  const context = { warn: mock() }
  // When
  const result = markers.renderChunk('f("a--a")', { modules: {} }, context)
  // Then
  expect(result).toBeNull()
  expect(context.warn).not.toHaveBeenCalled()
})
