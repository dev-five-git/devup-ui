import { randomUUID } from 'node:crypto'
import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

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

import { formatPortFile } from '../coordinator-port'
import type { DevupUILoaderOptions } from '../loader'
import loader, { resetInit, setWasmForTesting } from '../loader'

const defaults = {
  package: 'package',
  cssDir: 'cssDir',
  sheetFile: 'sheetFile',
  classMapFile: 'classMapFile',
  fileMapFile: 'fileMapFile',
  themeFile: 'themeFile',
  watch: false,
  singleCss: true,
  defaultSheet: {},
  defaultClassMap: {},
  defaultFileMap: {},
}
function invoke(
  options: Partial<DevupUILoaderOptions> = {},
  resourcePath = resolve('App.tsx'),
) {
  const addDependency = mock()
  const callback = mock()
  const result = new Promise<{ code?: string; map?: string | null }>(
    (resolve, reject) => {
      callback.mockImplementation(
        (error: Error | null, code?: string, map?: string | null) => {
          if (error) reject(error)
          else resolve({ code, map })
        },
      )
      Reflect.apply(
        loader,
        {
          getOptions: () => ({ ...defaults, ...options }),
          resourcePath,
          addDependency,
          async: () => callback,
        },
        [Buffer.from('source')],
      )
    },
  )
  return { result, callback, addDependency }
}

describe('local source extraction', () => {
  let spies: ReturnType<typeof spyOn>[] = []
  let extract: ReturnType<typeof spyOn>,
    exists: ReturnType<typeof spyOn>,
    read: ReturnType<typeof spyOn>,
    write: ReturnType<typeof spyOn>
  beforeEach(() => {
    resetInit()
    setWasmForTesting(wasm)
    exists = spyOn(fs, 'existsSync').mockReturnValue(false)
    read = spyOn(fs, 'readFileSync').mockReturnValue('{}')
    write = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
    extract = spyOn(wasm, 'codeExtract').mockReturnValue({
      code: 'compiled',
      css: undefined,
      cssFile: undefined,
      map: undefined,
      updatedBaseStyle: false,
      dependencies: ['tokens.ts'],
      free: mock(),
      [Symbol.dispose]: mock(),
    })
    spies = [exists, read, write, extract]
    for (const key of [
      'importSheet',
      'importClassMap',
      'importFileMap',
      'registerTheme',
    ] as const)
      spies.push(spyOn(wasm, key).mockImplementation(() => {}))
    for (const key of [
      'exportSheet',
      'exportClassMap',
      'exportFileMap',
      'getCss',
    ] as const)
      spies.push(spyOn(wasm, key).mockReturnValue('state'))
  })
  afterEach(() => {
    for (const spy of spies) spy.mockRestore()
    setWasmForTesting(undefined)
  })
  it('extracts with project-relative ids and dependency paths in build mode', async () => {
    const projectRoot = resolve('project')
    const run = invoke({ projectRoot }, join(projectRoot, 'App.tsx'))
    expect(await run.result).toEqual({ code: 'compiled', map: null })
    expect(extract.mock.calls[0]?.[0]).toBe('App.tsx')
    expect(run.addDependency).toHaveBeenCalledWith(
      join(projectRoot, 'tokens.ts'),
    )
    expect(write).not.toHaveBeenCalled()
  })
  it('initializes defaults once across local operations', async () => {
    await invoke().result
    await invoke().result
    expect(wasm.importSheet).toHaveBeenCalledTimes(1)
    expect(wasm.registerTheme).toHaveBeenCalledWith(undefined)
  })
  it('writes updated sheet and per-file state in watch mode', async () => {
    exists.mockReturnValue(true)
    read.mockReturnValue('{"theme":{"colors":{}}}')
    extract.mockReturnValue({
      code: 'compiled',
      map: '{}',
      cssFile: 'devup-ui-1.css',
      updatedBaseStyle: true,
    })
    const run = invoke({ watch: true })
    expect(await run.result).toEqual({ code: 'compiled', map: '{}' })
    expect(write).toHaveBeenCalledWith(
      join('cssDir', 'devup-ui.css'),
      'state',
      'utf-8',
    )
    expect(write).toHaveBeenCalledWith('sheetFile', 'state')
    expect(run.addDependency).toHaveBeenCalledWith('themeFile')
    expect(wasm.registerTheme).toHaveBeenCalledWith({ colors: {} })
  })
  it('registers an empty theme when config has no theme', async () => {
    exists.mockReturnValue(true)
    await invoke({ watch: true, cssDir: resolve('.') }).result
    expect(wasm.registerTheme).toHaveBeenCalledWith({})
  })
  it('does not import partial snapshots when a later snapshot cannot parse', async () => {
    exists.mockReturnValue(true)
    read.mockImplementation((file: unknown) =>
      file === 'fileMapFile' ? 'broken' : '{}',
    )
    await expect(invoke({ watch: true }).result).rejects.toThrow()
    expect(wasm.importSheet).not.toHaveBeenCalled()
    expect(wasm.importClassMap).not.toHaveBeenCalled()
  })
  it('extracts with missing snapshot files in watch mode', async () => {
    expect((await invoke({ watch: true }).result).code).toBe('compiled')
    expect(wasm.importSheet).not.toHaveBeenCalled()
  })
  it.each([new Error('extraction failed'), 'extraction failed'])(
    'propagates extraction failure',
    async (error) => {
      extract.mockImplementation(() => {
        throw error
      })
      await expect(invoke().result).rejects.toThrow('extraction failed')
    },
  )
  it('propagates write failure to the callback exactly once', async () => {
    extract.mockReturnValue({
      code: 'compiled',
      cssFile: 'file.css',
      updatedBaseStyle: false,
    })
    write.mockRejectedValue(new Error('write failed'))
    const run = invoke({ watch: true })
    await expect(run.result).rejects.toThrow('write failed')
    expect(run.callback).toHaveBeenCalledTimes(1)
  })
  it('rejects malformed source maps rather than returning success', async () => {
    extract.mockReturnValue({ code: 'compiled', map: 'broken' })
    await expect(invoke().result).rejects.toThrow()
  })
})

describe('coordinator source extraction', () => {
  let dir: string
  let server: ReturnType<typeof createServer>
  let response = '{}'
  let status = 200
  let payload = ''
  beforeEach(async () => {
    dir = fs.mkdtempSync(join(tmpdir(), 'devup-loader-'))
    response = '{}'
    status = 200
    payload = ''
    const identity = { project: dir, token: randomUUID() }
    server = createServer((req, res) => {
      if (req.url === '/health')
        res.end(formatPortFile(port, process.pid, identity))
      else {
        req.on('data', (chunk: Buffer) => {
          payload += chunk.toString()
        })
        req.on('end', () => {
          res.writeHead(status)
          res.end(response)
        })
      }
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const address = server.address()
    if (!address || typeof address === 'string')
      throw new Error('Expected TCP server')
    const port = address.port
    fs.writeFileSync(
      join(dir, 'endpoint'),
      formatPortFile(port, process.pid, identity),
    )
    fs.writeFileSync(
      join(dir, 'themeFile'),
      JSON.stringify({ extends: ['base.json', 'missing.json'] }),
    )
    fs.writeFileSync(join(dir, 'base.json'), '{}')
  })
  afterEach(async () => {
    server.closeAllConnections()
    await new Promise<void>((resolve) => server.close(() => resolve()))
    fs.rmSync(dir, { recursive: true, force: true })
  })
  function run(extra: Partial<DevupUILoaderOptions> = {}) {
    return invoke(
      {
        coordinatorPortFile: join(dir, 'endpoint'),
        projectRoot: dir,
        requestTimeoutMs: 100,
        revisionFile: 'revision',
        ...extra,
      },
      join(dir, 'src', 'App.tsx'),
    )
  }
  it('returns code/map and watches config inheritance, missing config, revision and imported tokens', async () => {
    response = JSON.stringify({
      code: 'compiled',
      map: '{"version":3}',
      dependencies: ['tokens.ts', 1],
    })
    const request = run()
    expect(await request.result).toEqual({
      code: 'compiled',
      map: '{"version":3}',
    })
    expect(JSON.parse(payload).filename).toBe('src/App.tsx')
    for (const file of [
      'endpoint',
      'revision',
      'themeFile',
      'base.json',
      'missing.json',
      'tokens.ts',
    ])
      expect(request.addDependency).toHaveBeenCalledWith(join(dir, file))
  })
  it('uses explicit theme dependency files when supplied', async () => {
    response = '{"code":"compiled"}'
    const request = run({
      themeFiles: ['explicit.json'],
      revisionFile: undefined,
    })
    expect(await request.result).toEqual({ code: 'compiled', map: null })
    expect(request.addDependency).toHaveBeenCalledWith(
      join(dir, 'explicit.json'),
    )
  })
  it.each([
    'null',
    '{}',
    '{"code":1}',
    '{',
    '{"code":"compiled","map":"broken"}',
  ])('rejects malformed extraction data %s with location', async (body) => {
    response = body
    const request = run()
    await expect(request.result).rejects.toThrow(
      `${join(dir, 'src', 'App.tsx')}:1:1:`,
    )
    expect(request.callback).toHaveBeenCalledTimes(1)
  })
  it('propagates server build-time error details', async () => {
    status = 500
    response = '{"error":"tokens.ts:3:5: failed"}'
    await expect(run().result).rejects.toThrow('tokens.ts:3:5: failed')
  })
  it('reports invalid config with a located coordinator error before dispatch', async () => {
    fs.writeFileSync(join(dir, 'themeFile'), 'broken')
    await expect(run().result).rejects.toThrow(`${join(dir, 'themeFile')}:1:1:`)
    expect(payload).toBe('')
  })
})
