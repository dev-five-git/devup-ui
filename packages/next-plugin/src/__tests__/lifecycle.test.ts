import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'
import type { NextConfig } from 'next'

import type { CoordinatorHandle } from '../coordinator-options'
import { installAfterCompileDrain, retainSession } from '../lifecycle'
import { createAppContext, createSession } from '../session'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

type Handler = () => unknown

let handlers: Record<string, Handler>
let onceSpy: ReturnType<typeof spyOn>
let errorSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  handlers = {}
  onceSpy = spyOn(process, 'once').mockImplementation(((
    event: string,
    handler: Handler,
  ) => {
    handlers[event] = handler
    return process
  }) as never)
  errorSpy = spyOn(console, 'error').mockImplementation(() => {})
})
afterEach(() => {
  onceSpy.mockRestore()
  errorSpy.mockRestore()
})

function setup() {
  process.chdir(makeProject())
  const session = createSession(createAppContext({}, {}))
  mkdirSync(session.sessionDir, { recursive: true })
  writeFileSync(session.revisionFile, '0')
  const order: string[] = []
  const handle: {
    -readonly [K in keyof CoordinatorHandle]: CoordinatorHandle[K]
  } = {
    ready: Promise.resolve(),
    close: mock(() => void order.push('close')),
    drain: mock(async () => {
      await Promise.resolve()
      order.push('drain')
    }),
  }
  return { session, handle, order }
}

describe('retainSession', () => {
  it('drains before closing when the event loop is empty', async () => {
    const { session, handle, order } = setup()
    retainSession({ session, coordinator: handle })
    expect(Object.keys(handlers).sort()).toEqual(['beforeExit', 'exit'])
    expect(handle.close).not.toHaveBeenCalled()

    await handlers.beforeExit!()

    expect(order).toEqual(['drain', 'close'])
    expect(existsSync(session.sessionDir)).toBe(false)
  })

  it('closes synchronously on exit and never twice', async () => {
    const { session, handle } = setup()
    retainSession({ session, coordinator: handle })

    handlers.exit!()
    handlers.exit!()
    await handlers.beforeExit!()

    expect(handle.close).toHaveBeenCalledTimes(1)
    expect(handle.drain).toHaveBeenCalledTimes(1)
    expect(existsSync(session.sessionDir)).toBe(false)
  })

  it('does not wait for anything on exit', () => {
    const { session, handle } = setup()
    retainSession({ session, coordinator: handle })

    const returned = handlers.exit!()

    expect(returned).toBeUndefined()
    expect(handle.drain).not.toHaveBeenCalled()
  })

  it('reports a failed drain with its location and still closes', async () => {
    const { session, handle } = setup()
    handle.drain = mock(async () => {
      throw new Error('disk full')
    })
    retainSession({ session, coordinator: handle })

    await handlers.beforeExit!()

    expect(errorSpy).toHaveBeenCalledWith(
      `${session.sessionDir}:1:1: devup-ui cannot use \`the final state write\` at build time: disk full; needs a writable distDir.`,
    )
    expect(handle.close).toHaveBeenCalledTimes(1)
  })

  it('reports a coordinator that never started listening', async () => {
    const { session, handle } = setup()
    handle.ready = Promise.reject(new Error('EADDRINUSE'))
    retainSession({ session, coordinator: handle })

    await handle.ready.catch(() => undefined)
    await Promise.resolve()

    expect(errorSpy).toHaveBeenCalledWith(
      `${session.endpointFile}:1:1: devup-ui coordinator cannot use \`a loopback listener\` at build time: EADDRINUSE; needs permission to listen on 127.0.0.1 and to write its endpoint file.`,
    )
  })

  it('describes a failure that is not an Error', async () => {
    const { session, handle } = setup()
    handle.drain = mock(() => Promise.reject('plain'))
    retainSession({ session, coordinator: handle })

    await handlers.beforeExit!()

    expect(String(errorSpy.mock.calls[0]?.[0])).toContain(
      'at build time: plain;',
    )
  })

  it('does not touch the directory of another session', async () => {
    const first = setup()
    const second = setup()
    retainSession({ session: first.session, coordinator: first.handle })
    const firstHandlers = { ...handlers }
    retainSession({ session: second.session, coordinator: second.handle })

    firstHandlers.exit!()

    expect(first.handle.close).toHaveBeenCalledTimes(1)
    expect(second.handle.close).not.toHaveBeenCalled()
    expect(existsSync(second.session.sessionDir)).toBe(true)
    expect(existsSync(join(second.session.sessionDir, 'revision'))).toBe(true)
  })
})

describe('installAfterCompileDrain', () => {
  it('drains the coordinator of the session before the user hook runs', async () => {
    const { session, handle, order } = setup()
    retainSession({ session, coordinator: handle })
    const metadata = { projectDir: '/p', distDir: '.next' }
    const user = mock(async () => {
      order.push('user')
    })
    const config: NextConfig = {
      compiler: { removeConsole: true, runAfterProductionCompile: user },
    }

    installAfterCompileDrain(config, session.token)
    await config.compiler?.runAfterProductionCompile?.(metadata)

    expect(order).toEqual(['drain', 'user'])
    expect(user).toHaveBeenCalledWith(metadata)
    expect(config.compiler?.removeConsole).toBe(true)
    expect(handle.close).not.toHaveBeenCalled()
  })

  it('works without a user hook or compiler options', async () => {
    const { session, handle } = setup()
    retainSession({ session, coordinator: handle })
    const config: NextConfig = {}

    installAfterCompileDrain(config, session.token)
    await config.compiler?.runAfterProductionCompile?.({
      projectDir: '/p',
      distDir: '.next',
    })

    expect(handle.drain).toHaveBeenCalledTimes(1)
  })

  it('does nothing for a session that is not in this process or is over', async () => {
    const { session, handle } = setup()
    retainSession({ session, coordinator: handle })
    handlers.exit!()
    const user = mock(async () => {})
    const config: NextConfig = { compiler: { runAfterProductionCompile: user } }

    installAfterCompileDrain(config, session.token)
    await config.compiler?.runAfterProductionCompile?.({
      projectDir: '/p',
      distDir: '.next',
    })
    installAfterCompileDrain(config, 'unknown-session')
    await config.compiler?.runAfterProductionCompile?.({
      projectDir: '/p',
      distDir: '.next',
    })

    expect(handle.drain).not.toHaveBeenCalled()
    expect(user).toHaveBeenCalledTimes(2)
  })

  it('lets a failed drain fail the build', async () => {
    const { session, handle } = setup()
    handle.drain = mock(async () => {
      throw new Error('write failed')
    })
    retainSession({ session, coordinator: handle })
    const config: NextConfig = {}

    installAfterCompileDrain(config, session.token)

    await expect(
      config.compiler?.runAfterProductionCompile?.({
        projectDir: '/p',
        distDir: '.next',
      }),
    ).rejects.toThrow('write failed')
  })
})
