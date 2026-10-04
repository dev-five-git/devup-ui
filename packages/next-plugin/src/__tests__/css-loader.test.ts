import { randomUUID } from 'node:crypto'
import * as fs from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
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

import { formatPortFile } from '../coordinator-port'
import type { DevupUICssLoaderOptions } from '../css-loader'
import loader, { resetInit, setWasmForTesting } from '../css-loader'

const defaults = {
  watch: false,
  sheetFile: 'sheet',
  classMapFile: 'classes',
  fileMapFile: 'files',
  themeFile: 'devup.json',
  defaultSheet: {},
  defaultClassMap: {},
  defaultFileMap: {},
}
function invoke(
  options: Partial<DevupUICssLoaderOptions>,
  resourcePath = 'devup-ui.css',
  resourceQuery?: string,
) {
  const addDependency = mock(),
    callback = mock()
  const result = new Promise<unknown>((resolve, reject) => {
    callback.mockImplementation((error: Error | null, content: unknown) => {
      if (error) reject(error)
      else resolve(content)
    })
    Reflect.apply(
      loader,
      {
        getOptions: () => ({ ...defaults, ...options }),
        resourcePath,
        resourceQuery,
        addDependency,
        callback,
        async: () => callback,
      },
      [Buffer.from('disk'), 'map', 'meta'],
    )
  })
  return { result, addDependency, callback }
}

describe('local CSS loading', () => {
  let spies: ReturnType<typeof spyOn>[]
  let exists: ReturnType<typeof spyOn>, read: ReturnType<typeof spyOn>
  beforeEach(() => {
    resetInit()
    setWasmForTesting(wasm)
    exists = spyOn(fs, 'existsSync').mockReturnValue(false)
    read = spyOn(fs, 'readFileSync').mockReturnValue('{}')
    spies = [exists, read, spyOn(wasm, 'getCss').mockReturnValue('live')]
    for (const key of [
      'importSheet',
      'importClassMap',
      'importFileMap',
      'registerTheme',
    ] as const)
      spies.push(spyOn(wasm, key).mockImplementation(() => {}))
  })
  afterEach(() => {
    for (const spy of spies) spy.mockRestore()
    setWasmForTesting(undefined)
  })
  it('returns build source and initializes defaults once', async () => {
    const run = invoke({})
    expect(await run.result).toEqual(Buffer.from('disk'))
    expect(run.callback).toHaveBeenCalledWith(
      null,
      Buffer.from('disk'),
      'map',
      'meta',
    )
    await invoke({}).result
    expect(wasm.importSheet).toHaveBeenCalledTimes(1)
  })
  it('reads live CSS with missing watch snapshots', async () => {
    expect(await invoke({ watch: true }).result).toBe('live')
    expect(wasm.importSheet).not.toHaveBeenCalled()
  })
  it.each(['{}', '{"theme":{"color":"red"}}'])(
    'restores parsed watch snapshots and theme',
    async (content) => {
      exists.mockReturnValue(true)
      read.mockReturnValue(content)
      expect(await invoke({ watch: true }).result).toBe('live')
      expect(wasm.registerTheme).toHaveBeenCalledWith(
        JSON.parse(content).theme ?? {},
      )
    },
  )
  it('does not restore a partial snapshot after parse failure', async () => {
    exists.mockReturnValue(true)
    read.mockImplementation((file: unknown) =>
      file === 'files' ? 'broken' : '{}',
    )
    await expect(invoke({ watch: true }).result).rejects.toThrow()
    expect(wasm.importSheet).not.toHaveBeenCalled()
  })
})

describe('coordinator CSS loading', () => {
  let dir: string, portFile: string
  let server: ReturnType<typeof createServer>
  let status = 200,
    content = 'live',
    query = ''
  beforeEach(async () => {
    dir = fs.mkdtempSync(join(tmpdir(), 'devup-css-loader-'))
    portFile = join(dir, 'endpoint')
    status = 200
    content = 'live'
    query = ''
    const identity = { project: dir, token: randomUUID() }
    server = createServer((req, res) => {
      if (req.url === '/health')
        res.end(formatPortFile(port, process.pid, identity))
      else {
        query = req.url ?? ''
        res.writeHead(status)
        res.end(content)
      }
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const address = server.address()
    if (!address || typeof address === 'string')
      throw new Error('Expected TCP server')
    const port = address.port
    fs.writeFileSync(portFile, formatPortFile(port, process.pid, identity))
    fs.writeFileSync(
      join(dir, 'devup.json'),
      '{"extends":["base.json","missing.json"]}',
    )
    fs.writeFileSync(join(dir, 'base.json'), '{}')
  })
  afterEach(async () => {
    server.closeAllConnections()
    await new Promise<void>((resolve) => server.close(() => resolve()))
    fs.rmSync(dir, { recursive: true, force: true })
  })
  function run(
    extra: Partial<DevupUICssLoaderOptions> = {},
    path?: string,
    resourceQuery?: string,
  ) {
    return invoke(
      {
        coordinatorPortFile: portFile,
        projectRoot: dir,
        revisionFile: 'revision',
        requestTimeoutMs: 1000,
        ...extra,
      },
      path,
      resourceQuery,
    )
  }
  it('watches endpoint, revision and full config chain, including missing files', async () => {
    const request = run()
    expect(await request.result).toBe('live')
    for (const file of [
      'endpoint',
      'revision',
      'devup.json',
      'base.json',
      'missing.json',
    ])
      expect(request.addDependency).toHaveBeenCalledWith(join(dir, file))
    expect(
      new URL(query, 'http://localhost').searchParams.get('waitForIdle'),
    ).toBe('true')
  })
  it.each([
    ['devup-ui.css?fileNum=79', undefined, '79'],
    ['devup-ui.css', '?fileNum=3', '3'],
  ])(
    'preserves per-file query information %s',
    async (path, resourceQuery, expected) => {
      expect(
        await run(
          {
            watch: true,
            themeFiles: ['explicit.json'],
            revisionFile: undefined,
          },
          path,
          resourceQuery,
        ).result,
      ).toBe('live')
      const params = new URL(query, 'http://localhost').searchParams
      expect(params.get('fileNum')).toBe(expected)
      expect(params.get('importMainCss')).toBe('true')
      expect(params.has('waitForIdle')).toBe(false)
    },
  )
  it('propagates missing filename details from coordinator errors once', async () => {
    status = 503
    content = '{"error":"Missing app/page.tsx"}'
    const request = run()
    await expect(request.result).rejects.toThrow('Missing app/page.tsx')
    expect(request.callback).toHaveBeenCalledTimes(1)
  })
  it('returns located errors for invalid config before HTTP dispatch', async () => {
    fs.writeFileSync(join(dir, 'devup.json'), 'broken')
    await expect(run().result).rejects.toThrow('devup-ui.css:1:1:')
    expect(query).toBe('')
  })
})
