import * as fs from 'node:fs'
import { request } from 'node:http'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import {
  type CoordinatorOptions,
  resetCoordinator,
  startCoordinator,
} from '../coordinator'

let codeExtractSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let exportSheetSpy: ReturnType<typeof spyOn>
let exportClassMapSpy: ReturnType<typeof spyOn>
let exportFileMapSpy: ReturnType<typeof spyOn>
let writeFileSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>

const tmpDir = join(process.cwd(), '.tmp-coordinator-test')

function makeOptions(
  overrides: Partial<CoordinatorOptions> = {},
): CoordinatorOptions {
  return {
    package: '@devup-ui/react',
    cssDir: join(tmpDir, 'css'),
    singleCss: false,
    sheetFile: join(tmpDir, 'sheet.json'),
    classMapFile: join(tmpDir, 'classMap.json'),
    fileMapFile: join(tmpDir, 'fileMap.json'),
    importAliases: {},
    coordinatorPortFile: join(tmpDir, 'coordinator.port'),
    ...overrides,
  }
}

function httpRequest(
  port: number,
  method: string,
  path: string,
  body?: string,
): Promise<{ status: number; body: string }> {
  return new Promise((resolve, reject) => {
    const req = request(
      {
        hostname: '127.0.0.1',
        port,
        path,
        method,
        headers: body ? { 'Content-Type': 'application/json' } : undefined,
      },
      (res) => {
        const chunks: Buffer[] = []
        res.on('data', (chunk: Buffer) => chunks.push(chunk))
        res.on('end', () => {
          resolve({
            status: res.statusCode ?? 0,
            body: Buffer.concat(chunks).toString('utf-8'),
          })
        })
      },
    )
    req.on('error', reject)
    if (body) req.write(body)
    req.end()
  })
}

beforeEach(() => {
  codeExtractSpy = spyOn(wasm, 'codeExtract')
  getCssSpy = spyOn(wasm, 'getCss')
  exportSheetSpy = spyOn(wasm, 'exportSheet')
  exportClassMapSpy = spyOn(wasm, 'exportClassMap')
  exportFileMapSpy = spyOn(wasm, 'exportFileMap')
  writeFileSpy = spyOn(fs, 'writeFile').mockImplementation(
    (_path: any, _data: any, _encOrCb: any, maybeCb?: any) => {
      const cb = typeof _encOrCb === 'function' ? _encOrCb : maybeCb
      if (cb) cb(null)
    },
  )
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
})

afterEach(() => {
  resetCoordinator()
  codeExtractSpy.mockRestore()
  getCssSpy.mockRestore()
  exportSheetSpy.mockRestore()
  exportClassMapSpy.mockRestore()
  exportFileMapSpy.mockRestore()
  writeFileSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
})

describe('coordinator', () => {
  it('should start and respond to /health', async () => {
    const options = makeOptions()
    const coordinator = startCoordinator(options)

    // Wait for server to start and write port file
    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(port, 'GET', '/health')
    expect(res.status).toBe(200)
    expect(res.body).toBe('ok')

    coordinator.close()
  })

  it('should handle /extract endpoint', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'transformed code',
      map: '{"version":3}',
      cssFile: 'devup-ui-1.css',
      updatedBaseStyle: true,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockImplementation(
      (fileNum: number | null, _importMainCss: boolean) => {
        if (fileNum === null) return 'base-css'
        return `file-css-${fileNum}`
      },
    )
    exportSheetSpy.mockReturnValue('sheet-json')
    exportClassMapSpy.mockReturnValue('classmap-json')
    exportFileMapSpy.mockReturnValue('filemap-json')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    expect(res.status).toBe(200)
    const data = JSON.parse(res.body)
    expect(data.code).toBe('transformed code')
    expect(data.map).toBe('{"version":3}')
    expect(data.cssFile).toBe('devup-ui-1.css')
    expect(data.updatedBaseStyle).toBe(true)

    // Verify WASM was called
    expect(codeExtractSpy).toHaveBeenCalledTimes(1)

    // Verify files were written (base CSS + per-file CSS + sheet + classmap + filemap)
    expect(writeFileSpy).toHaveBeenCalledTimes(5)

    coordinator.close()
  })

  it('should rewrite per-file CSS imports when singleCss=false', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'import "./../../df/devup-ui/devup-ui-79.css";\nimport "./../../df/devup-ui/devup-ui-3.css";\nconst x = 1;',
      map: '{"version":3}',
      cssFile: 'devup-ui-79.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('file-css')
    exportSheetSpy.mockReturnValue('{}')
    exportClassMapSpy.mockReturnValue('{}')
    exportFileMapSpy.mockReturnValue('{}')

    const options = makeOptions({ singleCss: false })
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    expect(res.status).toBe(200)
    const data = JSON.parse(res.body)
    // Verify imports were rewritten from devup-ui-N.css to devup-ui.css?fileNum=N
    expect(data.code).toContain('devup-ui.css?fileNum=79')
    expect(data.code).toContain('devup-ui.css?fileNum=3')
    expect(data.code).not.toContain('devup-ui-79.css')
    expect(data.code).not.toContain('devup-ui-3.css')

    coordinator.close()
  })

  it('should NOT rewrite CSS imports when singleCss=true', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'import "./../../df/devup-ui/devup-ui.css";\nconst x = 1;',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('all-styles')
    exportSheetSpy.mockReturnValue('{}')
    exportClassMapSpy.mockReturnValue('{}')
    exportFileMapSpy.mockReturnValue('{}')

    const options = makeOptions({ singleCss: true })
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    expect(res.status).toBe(200)
    const data = JSON.parse(res.body)
    // singleCss=true: no rewriting should happen, devup-ui.css stays as-is
    expect(data.code).toContain('devup-ui.css')
    expect(data.code).not.toContain('?fileNum=')

    coordinator.close()
  })

  it('should handle /extract in singleCss mode (cssFile is devup-ui.css)', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'single css code',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('all-styles')
    exportSheetSpy.mockReturnValue('sheet-json')
    exportClassMapSpy.mockReturnValue('classmap-json')
    exportFileMapSpy.mockReturnValue('filemap-json')

    const options = makeOptions({ singleCss: true })
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    expect(res.status).toBe(200)
    const data = JSON.parse(res.body)
    expect(data.code).toBe('single css code')
    expect(data.cssFile).toBe('devup-ui.css')

    // Verify getCss was called with (null, true) for the CSS file write
    expect(getCssSpy).toHaveBeenCalledWith(null, true)

    // Verify files were written (CSS + sheet + classmap + filemap, no base style update)
    expect(writeFileSpy).toHaveBeenCalledTimes(4)

    coordinator.close()
  })

  it('should handle /extract when no CSS file produced', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'no style code',
      map: undefined,
      cssFile: undefined,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/plain.ts',
        code: 'const x = 1',
        resourcePath: join(process.cwd(), 'src', 'plain.ts'),
      }),
    )

    expect(res.status).toBe(200)
    const data = JSON.parse(res.body)
    expect(data.code).toBe('no style code')
    expect(data.cssFile).toBeUndefined()

    // No file writes expected (no CSS, no updatedBaseStyle)
    expect(writeFileSpy).not.toHaveBeenCalled()

    coordinator.close()
  })

  it('should handle /extract errors', async () => {
    codeExtractSpy.mockImplementation(() => {
      throw new Error('extraction failed')
    })

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/bad.tsx',
        code: 'invalid',
        resourcePath: join(process.cwd(), 'src', 'bad.tsx'),
      }),
    )

    expect(res.status).toBe(500)
    const data = JSON.parse(res.body)
    expect(data.error).toBe('extraction failed')

    coordinator.close()
  })

  it('should handle /css endpoint', async () => {
    getCssSpy.mockReturnValue('css-content')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(
      port,
      'GET',
      '/css?fileNum=3&importMainCss=true',
    )

    expect(res.status).toBe(200)
    expect(res.body).toBe('css-content')
    expect(getCssSpy).toHaveBeenCalledWith(3, true)

    coordinator.close()
  })

  it('should handle /css endpoint without fileNum', async () => {
    getCssSpy.mockReturnValue('base-css')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(port, 'GET', '/css?importMainCss=false')

    expect(res.status).toBe(200)
    expect(res.body).toBe('base-css')
    expect(getCssSpy).toHaveBeenCalledWith(null, false)

    coordinator.close()
  })

  it('should handle /css with waitForIdle after extractions complete', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'code',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('complete-css')
    exportSheetSpy.mockReturnValue('{}')
    exportClassMapSpy.mockReturnValue('{}')
    exportFileMapSpy.mockReturnValue('{}')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    // Do an extraction first so totalExtractions > 0
    await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/A.tsx',
        code: 'code',
        resourcePath: join(process.cwd(), 'src', 'A.tsx'),
      }),
    )

    // Now request CSS with waitForIdle — should resolve after idle threshold
    const res = await httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    )

    expect(res.status).toBe(200)
    expect(res.body).toBe('complete-css')

    coordinator.close()
  })

  it('should handle /css with waitForIdle timeout when no extractions happen', async () => {
    getCssSpy.mockReturnValue('timeout-css')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    // Mock Date.now to simulate time passing beyond MAX_WAIT_MS (30s)
    let callCount = 0
    const dateNowSpy = spyOn(Date, 'now').mockImplementation(() => {
      callCount++
      // First call is `const start = Date.now()` → return 0
      // Subsequent calls return past MAX_WAIT_MS threshold
      if (callCount <= 1) return 0
      return 31_000
    })

    // Request CSS with waitForIdle=true but no extractions ever happen
    // (totalExtractions === 0), so it should timeout and return CSS anyway
    const res = await httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    )

    expect(res.status).toBe(200)
    expect(res.body).toBe('timeout-css')

    dateNowSpy.mockRestore()
    coordinator.close()
  })

  it('should return 404 for unknown routes', async () => {
    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    const res = await httpRequest(port, 'GET', '/unknown')

    expect(res.status).toBe(404)
    expect(res.body).toBe('Not Found')

    coordinator.close()
  })

  it('should write port file on startup', async () => {
    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    expect(writeFileSyncSpy).toHaveBeenCalledWith(
      options.coordinatorPortFile,
      expect.any(String),
      'utf-8',
    )

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)
    expect(port).toBeGreaterThan(0)

    coordinator.close()
  })

  it('should close cleanly', async () => {
    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    coordinator.close()

    // Server should be closed - double close should be safe
    coordinator.close()
  })

  it('should be reset via resetCoordinator while server is active', async () => {
    const options = makeOptions()
    startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    // resetCoordinator should close the active server
    resetCoordinator()

    // Calling again should be safe (server is already null)
    resetCoordinator()
  })
})
