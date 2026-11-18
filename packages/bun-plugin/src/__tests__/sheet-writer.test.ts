import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { createSheetWriter } from '../sheet-writer'

let directory: string

beforeEach(() => {
  directory = fs.mkdtempSync(join(tmpdir(), 'devup-bun-sheet-'))
})

afterEach(() => {
  fs.rmSync(directory, { recursive: true, force: true })
})

describe('createSheetWriter', () => {
  it('writes the sheets scheduled in one synchronous stretch once, before an awaiting caller resumes', async () => {
    const path = join(directory, 'devup-ui.css')
    let sheet = ''
    const writeSync = spyOn(fs, 'writeFileSync')
    try {
      const schedule = createSheetWriter(
        () => path,
        () => sheet,
      )
      for (let module = 0; module < 100; module++) {
        sheet += `.m${module}{color:red}`
        schedule()
      }
      // Nothing is written while the modules load
      expect(writeSync).not.toHaveBeenCalled()

      await Promise.resolve()
      expect(writeSync).toHaveBeenCalledTimes(1)
      expect(fs.readFileSync(path, 'utf-8')).toBe(sheet)

      // A later stretch writes again
      sheet += '.later{color:blue}'
      schedule()
      await Promise.resolve()
      expect(writeSync).toHaveBeenCalledTimes(2)
      expect(fs.readFileSync(path, 'utf-8')).toBe(sheet)
    } finally {
      writeSync.mockRestore()
    }
  })

  it('reports a failed write and writes again when a module next adds styles', async () => {
    const path = join(directory, 'missing', 'devup-ui.css')
    const logged = spyOn(console, 'error').mockReturnValue(undefined)
    try {
      const schedule = createSheetWriter(
        () => path,
        () => '.a{color:red}',
      )
      schedule()
      await Promise.resolve()
      expect(logged).toHaveBeenCalledTimes(1)

      fs.mkdirSync(join(directory, 'missing'))
      schedule()
      await Promise.resolve()
      expect(fs.readFileSync(path, 'utf-8')).toBe('.a{color:red}')
    } finally {
      logged.mockRestore()
    }
  })
})
