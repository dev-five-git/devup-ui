import * as modules from 'node:module'

import { expect, it, spyOn } from 'bun:test'

import { compileMdx } from '../mdx'
import * as implementation from '../plugin'
import * as publicEntry from '../register'

it('forwards the actual public registration and build bindings without wrappers', () => {
  expect(publicEntry.register).toBe(implementation.register)
  expect(publicEntry.DevupUI).toBe(implementation.DevupUI)
  expect(publicEntry.DevupUI({ debug: true }).name).toBe('devup-ui')
})

it('rethrows the original registration rejection from the native CJS entry', async () => {
  const cause = new Error('registration failed at native boundary')
  const rejected = Promise.reject<void>(cause)
  void rejected.then(undefined, () => undefined)
  let observe: (error: unknown) => void = () => undefined
  const observed = new Promise<unknown>((resolve) => {
    observe = resolve
  })
  function observeCatch<T = never>(
    reject?: ((reason: unknown) => T | PromiseLike<T>) | null,
  ): Promise<void | T> {
    const result = rejected.then(undefined, reject)
    void result.then(undefined, observe)
    return result
  }
  const catchSpy = spyOn(rejected, 'catch').mockImplementation(observeCatch)
  const register = spyOn(implementation, 'register').mockReturnValue(rejected)
  try {
    const entry = await import('../index.cjs')
    expect(await observed).toBe(cause)
    expect(entry.plugin).toBe(implementation.plugin)
    expect(register).toHaveBeenCalledWith()
  } finally {
    register.mockRestore()
    catchSpy.mockRestore()
  }
})

it('preserves non-missing compiler resolution failures at the native require boundary', async () => {
  const cause = Object.assign(new Error('compiler resolution denied'), {
    code: 'EACCES',
  })
  const createRequire = modules.createRequire
  const resolveSpies: { mockRestore(): void }[] = []
  const requireSpy = spyOn(modules, 'createRequire').mockImplementation(
    (path) => {
      const require = createRequire(path)
      resolveSpies.push(
        spyOn(require, 'resolve').mockImplementation(
          Object.assign(
            () => {
              throw cause
            },
            { paths: require.resolve.paths },
          ),
        ),
      )
      return require
    },
  )
  try {
    await expect(
      compileMdx(process.cwd(), '/page.mdx', '# title'),
    ).rejects.toBe(cause)
  } finally {
    requireSpy.mockRestore()
    for (const spy of resolveSpies) spy.mockRestore()
  }
})
