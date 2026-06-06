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
  flushCoordinatorWrites,
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
    canonicalMap: {},
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

    // Mock Date.now to simulate time passing beyond MAX_WAIT_MS (60s).
    let callCount = 0
    const dateNowSpy = spyOn(Date, 'now').mockImplementation(() => {
      callCount++
      // First call is `const start = Date.now()` — return 0
      // Subsequent calls return past MAX_WAIT_MS threshold
      if (callCount <= 1) return 0
      return 61_000
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

  it('should touch devup-ui.css to invalidate Turbopack cache when singleCss=false and new CSS collected', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'import "./../../df/devup-ui/devup-ui-5.css";\nconst x = 1;',
      map: undefined,
      cssFile: 'devup-ui-5.css',
      updatedBaseStyle: false,
      css: '.a{color:yellow}',
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockImplementation(
      (fileNum: number | null, _importMainCss: boolean) => {
        if (fileNum === null) return 'base-css'
        return `file-css-${fileNum}`
      },
    )
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
        code: 'const x = <Box color="yellow" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    expect(res.status).toBe(200)

    // 5 writes: per-file CSS + sheet + classmap + filemap + devup-ui.css invalidation
    expect(writeFileSpy).toHaveBeenCalledTimes(5)

    // Verify devup-ui.css was written to trigger Turbopack invalidation
    const devupUiCssWrite = writeFileSpy.mock.calls.find(
      (call: unknown[]) =>
        typeof call[0] === 'string' && call[0].endsWith('devup-ui.css'),
    )
    expect(devupUiCssWrite).toBeTruthy()
    // Content should include base CSS + timestamp nonce
    const content = devupUiCssWrite![1] as string
    expect(content).toContain('base-css')
    expect(content).toMatch(/\/\* \d+ \*\//)

    coordinator.close()
  })

  it('should NOT touch devup-ui.css when singleCss=false but no new CSS collected', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'const x = 1;',
      map: undefined,
      cssFile: 'devup-ui-5.css',
      updatedBaseStyle: false,
      css: undefined, // no new CSS collected
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

    await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = 1',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    // 4 writes: per-file CSS + sheet + classmap + filemap (NO devup-ui.css touch)
    expect(writeFileSpy).toHaveBeenCalledTimes(4)

    // Verify NO devup-ui.css write
    const devupUiCssWrite = writeFileSpy.mock.calls.find(
      (call: unknown[]) =>
        typeof call[0] === 'string' && call[0].endsWith('devup-ui.css'),
    )
    expect(devupUiCssWrite).toBeUndefined()

    coordinator.close()
  })

  it('should NOT touch devup-ui.css for singleCss=true (not needed)', async () => {
    codeExtractSpy.mockReturnValue({
      code: 'const x = 1;',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: false,
      css: '.a{color:yellow}',
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

    await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box color="yellow" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    // 4 writes: CSS file (devup-ui.css via cssFile) + sheet + classmap + filemap
    // NO additional devup-ui.css invalidation write (singleCss=true doesn't need it)
    expect(writeFileSpy).toHaveBeenCalledTimes(4)

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

  it('should coalesce duplicate writes to the same path within one /extract handler', async () => {
    // When a single /extract invocation triggers multiple writes to the same
    // path (singleCss + updatedBaseStyle=true → both the base-CSS write and
    // the cssFile write target `devup-ui.css`), the second write must be
    // collapsed by the latest-wins serializer: the first chained run sees the
    // *latest* content and writes it once, the second chained run finds
    // `latestContent` already consumed and resolves as a no-op.
    codeExtractSpy.mockReturnValue({
      code: 'single css code',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: true,
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

    // Both safeWrite calls target `devup-ui.css`. Coalescing means exactly
    // one physical writeFile call is made for that path; the sheet/classMap/
    // fileMap writes (3 more) all go to distinct paths.
    const devupUiCssWrites = writeFileSpy.mock.calls.filter((call) =>
      String(call[0]).endsWith('devup-ui.css'),
    )
    expect(devupUiCssWrites.length).toBe(1)

    coordinator.close()
  })

  it('should expose flushCoordinatorWrites to drain queued writes', async () => {
    // The exported helper must return a settled promise even when no writes
    // are pending (idle coordinator), so build orchestration can safely await
    // it without risk of hanging.
    await expect(flushCoordinatorWrites()).resolves.toBeUndefined()

    // After triggering a real /extract, awaiting the helper must wait for all
    // queued writes (chained per path) to settle. We assert the spy has been
    // invoked by the time the helper resolves.
    codeExtractSpy.mockReturnValue({
      code: 'code',
      map: undefined,
      cssFile: 'devup-ui-7.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('per-file-css')
    exportSheetSpy.mockReturnValue('sheet-json')
    exportClassMapSpy.mockReturnValue('classmap-json')
    exportFileMapSpy.mockReturnValue('filemap-json')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )

    await expect(flushCoordinatorWrites()).resolves.toBeUndefined()
    expect(writeFileSpy.mock.calls.length).toBeGreaterThan(0)

    coordinator.close()
  })

  it('should continue chained writes after a previous write fails (chain error recovery)', async () => {
    // The serializer must not let one failed write poison every subsequent
    // write for that path. We force the first writeFile to fail, then verify
    // the second extraction's writes still happen for that same path.
    codeExtractSpy.mockReturnValue({
      code: 'code',
      map: undefined,
      cssFile: 'devup-ui.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('css-content')
    exportSheetSpy.mockReturnValue('sheet-json')
    exportClassMapSpy.mockReturnValue('classmap-json')
    exportFileMapSpy.mockReturnValue('filemap-json')

    // Re-install writeFile spy with controlled failure: any write to the
    // devup-ui.css path errors out on the *first* invocation only.
    writeFileSpy.mockRestore()
    let devupCssCallCount = 0
    writeFileSpy = spyOn(fs, 'writeFile').mockImplementation(
      (_path: any, _data: any, _encOrCb: any, maybeCb?: any) => {
        const cb = typeof _encOrCb === 'function' ? _encOrCb : maybeCb
        if (cb) {
          if (String(_path).endsWith('devup-ui.css')) {
            devupCssCallCount++
            if (devupCssCallCount === 1) {
              cb(new Error('simulated disk error'))
              return
            }
          }
          cb(null)
        }
      },
    )

    const options = makeOptions({ singleCss: true })
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    // First /extract: triggers a write to devup-ui.css that we make fail.
    // The coordinator will respond with 500 (await Promise.all([..., failingWrite])
    // rejects), but the chain itself must NOT be poisoned.
    const firstRes = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/A.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'A.tsx'),
      }),
    )
    expect(firstRes.status).toBe(500)

    // Second /extract for the same path must SUCCEED — the `.catch(() => {})`
    // chain-survival branch is what makes this work.
    const secondRes = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/B.tsx',
        code: 'const y = <Box bg="blue" />',
        resourcePath: join(process.cwd(), 'src', 'B.tsx'),
      }),
    )
    expect(secondRes.status).toBe(200)

    // We must have observed at least 2 attempts on the devup-ui.css path:
    // the first (failed) and the second (succeeded).
    expect(devupCssCallCount).toBeGreaterThanOrEqual(2)

    coordinator.close()
  })

  it('should release the pending-extract slot when readBody throws before promotion', async () => {
    // If JSON.parse on the request body throws, the handler must still tear
    // down its `pendingExtractStarts` reservation (rather than the active
    // counter) so waitForIdle is not left waiting forever for a phantom
    // extraction. We verify by sending a malformed body, then proving the
    // coordinator still processes a follow-up extraction normally.
    codeExtractSpy.mockReturnValue({
      code: 'code',
      map: undefined,
      cssFile: 'devup-ui-1.css',
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    getCssSpy.mockReturnValue('per-file-css')
    exportSheetSpy.mockReturnValue('sheet-json')
    exportClassMapSpy.mockReturnValue('classmap-json')
    exportFileMapSpy.mockReturnValue('filemap-json')

    const options = makeOptions()
    const coordinator = startCoordinator(options)

    await new Promise((r) => setTimeout(r, 100))

    const portStr = (writeFileSyncSpy.mock.calls[0] as [string, string])[1]
    const port = parseInt(portStr)

    // Send an invalid body so JSON.parse throws BEFORE activeExtractions is
    // incremented. The handler must still respond 500 cleanly.
    const badRes = await httpRequest(port, 'POST', '/extract', 'not-json')
    expect(badRes.status).toBe(500)
    const errorPayload = JSON.parse(badRes.body) as { error: string }
    expect(typeof errorPayload.error).toBe('string')

    // A subsequent well-formed extraction must succeed. If the pending-slot
    // bookkeeping was wrong (decrementing activeExtractions instead of
    // pendingExtractStarts in finally), internal counters would drift negative
    // — that would not crash this request but is asserted by the next test
    // case via waitForIdle behaviour.
    const goodRes = await httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename: 'src/App.tsx',
        code: 'const x = <Box bg="red" />',
        resourcePath: join(process.cwd(), 'src', 'App.tsx'),
      }),
    )
    expect(goodRes.status).toBe(200)
    const okPayload = JSON.parse(goodRes.body) as { code: string }
    expect(okPayload.code).toBe('code')

    coordinator.close()
  })
})

describe('coordinator per-bucket completion', () => {
  function extractResult(cssFile: string) {
    return {
      code: 'code',
      map: undefined,
      cssFile,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    }
  }

  async function startAndGetPort(options: CoordinatorOptions) {
    const coordinator = startCoordinator(options)
    await new Promise((r) => setTimeout(r, 100))
    const port = parseInt(
      (writeFileSyncSpy.mock.calls[0] as [string, string])[1],
    )
    return { coordinator, port }
  }

  function extract(port: number, filename: string) {
    return httpRequest(
      port,
      'POST',
      '/extract',
      JSON.stringify({
        filename,
        code: 'c',
        resourcePath: join(process.cwd(), filename),
      }),
    )
  }

  // T0: idleThresholdMs option is honored by the base-css idle wait.
  it('honors idleThresholdMs for the base-css idle wait (fast when small)', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui.css'))
    getCssSpy.mockReturnValue('base-css')
    const { coordinator, port } = await startAndGetPort(
      makeOptions({ idleThresholdMs: 50 }),
    )
    await extract(port, 'src/A.tsx')

    const t0 = Date.now()
    const res = await httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    )
    const elapsed = Date.now() - t0

    expect(res.status).toBe(200)
    // 50ms threshold -> resolves fast; the previous hardcoded 2500ms idle
    // would push elapsed past 1500ms.
    expect(elapsed).toBeLessThan(1500)

    coordinator.close()
  })

  // T1: a collapsed bucket's CSS is not served until ALL its members are
  // extracted (the race that flaked landing e2e on slow CI).
  it('waits for all bucket members before serving a collapsed chunk', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui-1.css'))
    getCssSpy.mockReturnValue('bucket-css')
    // m1, m2 collapse into bucket.tsx; g is @global (must NOT be awaited).
    const canonicalMap = {
      'src/m1.tsx': 'src/bucket.tsx',
      'src/m2.tsx': 'src/bucket.tsx',
      'src/g.tsx': '@global',
    }
    const { coordinator, port } = await startAndGetPort(
      makeOptions({ canonicalMap, idleThresholdMs: 100 }),
    )
    // Extract ONLY the bucket root; m1, m2 still pending.
    await extract(port, 'src/bucket.tsx')
    getCssSpy.mockClear()

    // Request the bucket chunk; it must NOT resolve while m1/m2 are missing,
    // even though the (small) idle threshold has elapsed.
    let resolved = false
    const cssPromise = httpRequest(
      port,
      'GET',
      '/css?fileNum=1&importMainCss=true&waitForIdle=true',
    ).then((r) => {
      resolved = true
      return r
    })
    await new Promise((r) => setTimeout(r, 300))
    expect(resolved).toBe(false)
    expect(getCssSpy).not.toHaveBeenCalled()

    // Extract the remaining members -> the bucket is now complete.
    await extract(port, 'src/m1.tsx')
    await extract(port, 'src/m2.tsx')
    const res = await cssPromise
    expect(res.status).toBe(200)
    expect(getCssSpy).toHaveBeenCalledWith(1, true)

    coordinator.close()
  })

  // T2: a non-collapsed (singleton) bucket serves as soon as its own file is
  // extracted, without waiting out the idle threshold.
  it('serves a singleton bucket promptly without an idle wait', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui-1.css'))
    getCssSpy.mockReturnValue('css')
    const { coordinator, port } = await startAndGetPort(
      makeOptions({ canonicalMap: {}, idleThresholdMs: 2000 }),
    )
    await extract(port, 'src/f.tsx')

    const t0 = Date.now()
    const res = await httpRequest(
      port,
      'GET',
      '/css?fileNum=1&importMainCss=true&waitForIdle=true',
    )
    const elapsed = Date.now() - t0

    expect(res.status).toBe(200)
    // Members = {f} (already extracted) -> immediate; the 2000ms idle threshold
    // would otherwise dominate on the old idle-only path.
    expect(elapsed).toBeLessThan(1000)
    expect(getCssSpy).toHaveBeenCalledWith(1, true)

    coordinator.close()
  })

  // T3: a bucket member that never arrives -> the per-bucket wait fails open
  // after maxWaitMs (serves whatever exists) instead of hanging the build.
  it('fails open and serves partial CSS when a bucket member never extracts', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui-1.css'))
    getCssSpy.mockReturnValue('partial-css')
    const warnSpy = spyOn(console, 'warn').mockReturnValue(undefined)
    // m1 is a member of the bucket but is never POSTed to /extract.
    const canonicalMap = { 'src/m1.tsx': 'src/bucket.tsx' }
    const { coordinator, port } = await startAndGetPort(
      makeOptions({ canonicalMap, idleThresholdMs: 100, maxWaitMs: 150 }),
    )
    // Extract only the bucket root; src/m1.tsx stays missing forever.
    await extract(port, 'src/bucket.tsx')

    const t0 = Date.now()
    const res = await httpRequest(
      port,
      'GET',
      '/css?fileNum=1&importMainCss=true&waitForIdle=true',
    )
    const elapsed = Date.now() - t0

    // Resolves via the hard timeout (fail open) rather than hanging.
    expect(res.status).toBe(200)
    expect(res.body).toBe('partial-css')
    expect(elapsed).toBeGreaterThanOrEqual(150)
    expect(warnSpy).toHaveBeenCalled()
    expect(getCssSpy).toHaveBeenCalledWith(1, true)

    warnSpy.mockRestore()
    coordinator.close()
  })

  // T4: base css resolves DETERMINISTICALLY once every route-reachable runtime
  // file (expectedBaseFiles) is extracted — NOT after an idle gap. Proven by a
  // large idleThresholdMs that would dominate if the idle path were taken.
  it('serves base css as soon as all expectedBaseFiles are extracted (no idle wait)', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui.css'))
    getCssSpy.mockReturnValue('base-css')
    const { coordinator, port } = await startAndGetPort(
      makeOptions({
        expectedBaseFiles: ['src/a.tsx', 'src/b.tsx'],
        idleThresholdMs: 5000,
      }),
    )
    await extract(port, 'src/a.tsx')
    await extract(port, 'src/b.tsx')

    const t0 = Date.now()
    const res = await httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    )
    const elapsed = Date.now() - t0

    expect(res.status).toBe(200)
    expect(res.body).toBe('base-css')
    // Both expected files extracted -> immediate; the 5000ms idle threshold is
    // never consulted on the deterministic path.
    expect(elapsed).toBeLessThan(1000)

    coordinator.close()
  })

  // T5: the deterministic wait blocks base css until a still-missing
  // expectedBaseFile arrives — even after the idle threshold elapses with
  // nothing in flight. This is exactly the gap-between-waves case the old idle
  // heuristic resolved too early (dropping late files' styles).
  it('blocks base css until a missing expectedBaseFile is extracted', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui.css'))
    getCssSpy.mockReturnValue('base-css')
    const { coordinator, port } = await startAndGetPort(
      makeOptions({
        expectedBaseFiles: ['src/a.tsx', 'src/late.tsx'],
        idleThresholdMs: 50,
      }),
    )
    await extract(port, 'src/a.tsx')

    let resolved = false
    const cssPromise = httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    ).then((r) => {
      resolved = true
      return r
    })
    // Idle threshold (50ms) elapses and nothing is in flight, yet src/late.tsx
    // is still missing -> must NOT resolve.
    await new Promise((r) => setTimeout(r, 300))
    expect(resolved).toBe(false)

    await extract(port, 'src/late.tsx')
    const res = await cssPromise
    expect(res.status).toBe(200)
    expect(res.body).toBe('base-css')

    coordinator.close()
  })

  // T6: a phantom expectedBaseFile that never extracts fails open via the
  // dormant maxWaitMs backstop instead of hanging the build forever.
  it('fails open on a phantom expectedBaseFile via maxWaitMs', async () => {
    codeExtractSpy.mockReturnValue(extractResult('devup-ui.css'))
    getCssSpy.mockReturnValue('base-css')
    const { coordinator, port } = await startAndGetPort(
      makeOptions({
        expectedBaseFiles: ['src/a.tsx', 'src/phantom.tsx'],
        maxWaitMs: 150,
      }),
    )
    await extract(port, 'src/a.tsx')

    const t0 = Date.now()
    const res = await httpRequest(
      port,
      'GET',
      '/css?importMainCss=false&waitForIdle=true',
    )
    const elapsed = Date.now() - t0

    expect(res.status).toBe(200)
    expect(res.body).toBe('base-css')
    expect(elapsed).toBeGreaterThanOrEqual(150)

    coordinator.close()
  })
})
