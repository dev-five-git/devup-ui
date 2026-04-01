import * as fs from 'node:fs'
import * as http from 'node:http'

import * as wasm from '@devup-ui/wasm'
import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import devupUICssLoader, { resetInit } from '../css-loader'

let getCssSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>
let importSheetSpy: ReturnType<typeof spyOn>
let importClassMapSpy: ReturnType<typeof spyOn>
let importFileMapSpy: ReturnType<typeof spyOn>
let existsSyncSpy: ReturnType<typeof spyOn>
let readFileSyncSpy: ReturnType<typeof spyOn>

const defaultOptions = {
  watch: false,
  sheetFile: 'sheet.json',
  classMapFile: 'classMap.json',
  fileMapFile: 'fileMap.json',
  themeFile: 'devup.json',
  theme: {},
  defaultSheet: {},
  defaultClassMap: {},
  defaultFileMap: {},
}

beforeAll(() => {
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('get css')
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
  importSheetSpy = spyOn(wasm, 'importSheet').mockReturnValue(undefined)
  importClassMapSpy = spyOn(wasm, 'importClassMap').mockReturnValue(undefined)
  importFileMapSpy = spyOn(wasm, 'importFileMap').mockReturnValue(undefined)
  existsSyncSpy = spyOn(fs, 'existsSync').mockReturnValue(false)
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('{}')
})

afterEach(() => {
  resetInit()
  getCssSpy.mockClear()
  registerThemeSpy.mockClear()
  importSheetSpy.mockClear()
  importClassMapSpy.mockClear()
  importFileMapSpy.mockClear()
  existsSyncSpy.mockClear()
  readFileSyncSpy.mockClear()
})

afterAll(() => {
  getCssSpy.mockRestore()
  registerThemeSpy.mockRestore()
  importSheetSpy.mockRestore()
  importClassMapSpy.mockRestore()
  importFileMapSpy.mockRestore()
  existsSyncSpy.mockRestore()
  readFileSyncSpy.mockRestore()
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ ...defaultOptions, watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledWith(
      null,
      Buffer.from('data'),
      '',
      undefined,
    )
    // Should initialize on first call
    expect(importFileMapSpy).toHaveBeenCalledTimes(1)
    expect(importClassMapSpy).toHaveBeenCalledTimes(1)
    expect(importSheetSpy).toHaveBeenCalledTimes(1)
    expect(registerThemeSpy).toHaveBeenCalledTimes(1)
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledTimes(1)
    expect(getCssSpy).toHaveBeenCalledTimes(1)
    getCssSpy.mockClear()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)

    getCssSpy.mockClear()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)
  })

  it('should read files from disk in watch mode when files exist', () => {
    existsSyncSpy.mockReturnValue(true)
    readFileSyncSpy.mockReturnValue(JSON.stringify({ theme: { color: 'red' } }))

    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    // Should read files from disk
    expect(existsSyncSpy).toHaveBeenCalledTimes(4)
    expect(readFileSyncSpy).toHaveBeenCalledTimes(4)
    expect(importSheetSpy).toHaveBeenCalledTimes(1)
    expect(importClassMapSpy).toHaveBeenCalledTimes(1)
    expect(importFileMapSpy).toHaveBeenCalledTimes(1)
    expect(registerThemeSpy).toHaveBeenCalledWith({ color: 'red' })
  })

  it('should handle missing theme in devup.json', () => {
    existsSyncSpy.mockReturnValue(true)
    readFileSyncSpy.mockReturnValue(JSON.stringify({}))

    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ ...defaultOptions, watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    // Should call registerTheme with empty object when theme is missing
    expect(registerThemeSpy).toHaveBeenCalledWith({})
  })

  it('should fetch CSS from coordinator in coordinator mode', async () => {
    // Start a mock coordinator server
    const server = http.createServer((_req, res) => {
      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end('.a{color:red}')
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = (server.address() as { port: number }).port
    const portFile = `test-coordinator-${port}.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? String(port) : '{}',
    )

    const result = await new Promise<string>((resolve, reject) => {
      const asyncCallback = mock((err: Error | null, content?: string) => {
        if (err) reject(err)
        else resolve(content!)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          watch: true,
          coordinatorPortFile: portFile,
        }),
        resourcePath: 'devup-ui-1.css',
      } as any)(Buffer.from('stale content'), 'existing-map', 'meta')
    })

    expect(result).toBe('.a{color:red}')

    // NO WASM functions should be called
    expect(getCssSpy).not.toHaveBeenCalled()
    expect(importSheetSpy).not.toHaveBeenCalled()
    expect(importClassMapSpy).not.toHaveBeenCalled()
    expect(importFileMapSpy).not.toHaveBeenCalled()
    expect(registerThemeSpy).not.toHaveBeenCalled()

    server.close()
  })

  it('should fetch CSS from coordinator for devup-ui.css (singleCss)', async () => {
    const server = http.createServer((req, res) => {
      const url = new URL(req.url ?? '/', `http://${req.headers.host}`)
      expect(url.searchParams.get('importMainCss')).toBe('false')
      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end('.full{display:flex}')
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = (server.address() as { port: number }).port
    const portFile = `test-coordinator-${port}.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? String(port) : '{}',
    )

    const result = await new Promise<string>((resolve, reject) => {
      const asyncCallback = mock((err: Error | null, content?: string) => {
        if (err) reject(err)
        else resolve(content!)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          watch: false,
          coordinatorPortFile: portFile,
        }),
        resourcePath: 'devup-ui.css',
      } as any)(Buffer.from('stale'), '', '')
    })

    expect(result).toBe('.full{display:flex}')
    expect(getCssSpy).not.toHaveBeenCalled()
    expect(importSheetSpy).not.toHaveBeenCalled()

    server.close()
  })

  it('should fetch CSS from coordinator with ?fileNum query in resourcePath', async () => {
    const server = http.createServer((req, res) => {
      const url = new URL(req.url ?? '/', `http://${req.headers.host}`)
      // Should receive fileNum=79 and importMainCss=true
      expect(url.searchParams.get('fileNum')).toBe('79')
      expect(url.searchParams.get('importMainCss')).toBe('true')
      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end('.file79{color:blue}')
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = (server.address() as { port: number }).port
    const portFile = `test-coordinator-query-${port}.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? String(port) : '{}',
    )

    const result = await new Promise<string>((resolve, reject) => {
      const asyncCallback = mock((err: Error | null, content?: string) => {
        if (err) reject(err)
        else resolve(content!)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          watch: true,
          coordinatorPortFile: portFile,
        }),
        // Turbopack embeds query in resourcePath
        resourcePath: '/path/to/df/devup-ui/devup-ui.css?fileNum=79',
      } as any)(Buffer.from('stale content'), 'existing-map', 'meta')
    })

    expect(result).toBe('.file79{color:blue}')
    expect(getCssSpy).not.toHaveBeenCalled()

    server.close()
  })

  it('should fetch CSS from coordinator with ?fileNum in resourceQuery', async () => {
    const server = http.createServer((req, res) => {
      const url = new URL(req.url ?? '/', `http://${req.headers.host}`)
      expect(url.searchParams.get('fileNum')).toBe('3')
      expect(url.searchParams.get('importMainCss')).toBe('true')
      res.writeHead(200, { 'Content-Type': 'text/css' })
      res.end('.file3{color:green}')
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = (server.address() as { port: number }).port
    const portFile = `test-coordinator-rq-${port}.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? String(port) : '{}',
    )

    const result = await new Promise<string>((resolve, reject) => {
      const asyncCallback = mock((err: Error | null, content?: string) => {
        if (err) reject(err)
        else resolve(content!)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          watch: false,
          coordinatorPortFile: portFile,
        }),
        // resourcePath without query, query in separate property
        resourcePath: '/path/to/df/devup-ui/devup-ui.css',
        resourceQuery: '?fileNum=3',
      } as any)(Buffer.from('stale'), '', '')
    })

    expect(result).toBe('.file3{color:green}')
    expect(getCssSpy).not.toHaveBeenCalled()

    server.close()
  })

  it('should error when coordinator port file never appears', async () => {
    existsSyncSpy.mockReturnValue(false)

    const error = await new Promise<Error>((resolve) => {
      const asyncCallback = mock((err: Error | null) => {
        if (err) resolve(err)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          coordinatorPortFile: 'nonexistent.port',
        }),
        resourcePath: 'devup-ui.css',
      } as any)(Buffer.from(''), '', '')
    })

    expect(error.message).toBe('Coordinator port file not found')
  })

  it('should error when coordinator returns non-200 status', async () => {
    const server = http.createServer((_req, res) => {
      res.writeHead(500, { 'Content-Type': 'text/plain' })
      res.end('Internal Server Error')
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = (server.address() as { port: number }).port
    const portFile = `test-coordinator-err-${port}.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? String(port) : '{}',
    )

    const error = await new Promise<Error>((resolve) => {
      const asyncCallback = mock((err: Error | null) => {
        if (err) resolve(err)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          coordinatorPortFile: portFile,
        }),
        resourcePath: 'devup-ui.css',
      } as any)(Buffer.from(''), '', '')
    })

    expect(error.message).toBe('Coordinator CSS error: 500')
    server.close()
  })

  it('should error when readCoordinatorPort throws', async () => {
    const portFile = `test-coordinator-throw.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    readFileSyncSpy.mockImplementation((p: unknown) => {
      if (p === portFile) throw new Error('EACCES: permission denied')
      return '{}'
    })

    const error = await new Promise<Error>((resolve) => {
      const asyncCallback = mock((err: Error | null) => {
        if (err) resolve(err)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          coordinatorPortFile: portFile,
        }),
        resourcePath: 'devup-ui.css',
      } as any)(Buffer.from(''), '', '')
    })

    expect(error.message).toBe('EACCES: permission denied')
  })

  it('should error when coordinator connection fails', async () => {
    const portFile = `test-coordinator-conn-err.port`

    existsSyncSpy.mockImplementation((p: unknown) => p === portFile)
    // Use a port that nothing listens on
    readFileSyncSpy.mockImplementation((p: unknown) =>
      p === portFile ? '19999' : '{}',
    )

    const error = await new Promise<Error>((resolve) => {
      const asyncCallback = mock((err: Error | null) => {
        if (err) resolve(err)
      })
      devupUICssLoader.bind({
        async: () => asyncCallback,
        addContextDependency: mock(),
        getOptions: () => ({
          ...defaultOptions,
          coordinatorPortFile: portFile,
        }),
        resourcePath: 'devup-ui.css',
      } as any)(Buffer.from(''), '', '')
    })

    expect(error).toBeInstanceOf(Error)
  })
})
