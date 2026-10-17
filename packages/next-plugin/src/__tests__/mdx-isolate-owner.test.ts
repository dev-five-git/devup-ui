import { EventEmitter, once } from 'node:events'

import { expect, it } from 'bun:test'

import { createMdxIsolateOwner } from '../mdx-isolate-owner'
import { isMdxRecord } from '../mdx-pipeline'

const filename = '/dormant/resource.mdx'
const pipeline = {
  bundler: 'turbo' as const,
  ruleKey: '*.mdx',
  conditions: [],
  aliases: {},
  loaders: [{ loader: '@next/mdx/mdx-js-loader' }],
}
const demand = { run: 'one', filename, pipeline, timeoutMs: 1000 }
const canonical = {
  filename,
  source: 'compiled',
  map: undefined,
  dependencies: [],
  contextDependencies: [],
  missingDependencies: [],
  buildDependencies: [],
  dependencyReports: [],
}
const prepared = { kind: 'prepared', output: canonical, inputs: [] }
const binding = { run: 'one', filename, canonical }

class Channel extends EventEmitter {
  terminated = 0
  readonly sent: unknown[] = []
  constructor(
    readonly respond: (channel: Channel) => void = (channel) =>
      channel.emit('message', prepared),
  ) {
    super()
  }
  postMessage(value: unknown) {
    this.sent.push(value)
    if (isMdxRecord(value) && value.operation !== undefined) {
      queueMicrotask(() =>
        this.emit('message', { run: value.run, filename: value.filename }),
      )
      return
    }
    queueMicrotask(() => this.respond(this))
  }
  async terminate() {
    this.terminated++
    this.emit('exit', 0)
    return 0
  }
}

it('creates once on demand, retains once per resource/run, and awaits owned disposal', async () => {
  // Given
  let created = 0
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => {
    created++
    return channel
  })
  const signal = new AbortController().signal
  expect(created).toBe(0)
  try {
    // When
    await Promise.all([
      owner.prepare(demand, signal),
      owner.prepare(demand, signal),
    ])
    await owner.prepare({ ...demand, run: 'two' }, signal)
    // Then
    expect(await owner.bind(binding)).toEqual(canonical)
    expect(created).toBe(1)
    expect(channel.sent).toHaveLength(3)
  } finally {
    await owner.dispose()
  }
  expect(channel.terminated).toBe(1)
})

it.each([
  'invalid',
  'resource',
  'failed-resource',
  'error',
  'exit',
  'post',
  'startup',
  'timeout',
] as const)(
  'retains first transport failure and quarantines without respawn: %s',
  async (form) => {
    // Given
    const channel = new Channel((owned) => {
      switch (form) {
        case 'invalid':
          owned.emit('message', null)
          return
        case 'resource':
          owned.emit('message', {
            ...prepared,
            output: { ...canonical, filename: 'other' },
          })
          return
        case 'failed-resource':
          owned.emit('message', {
            kind: 'failed',
            error: {
              filename: 'other',
              message: 'wrong failure resource',
              line: 1,
              column: 1,
            },
          })
          return
        case 'error':
          owned.emit('error', new Error('first transport cause'))
          return
        case 'exit':
          owned.emit('exit', 1)
          return
        case 'post':
        case 'startup':
        case 'timeout':
          return
        default:
          return form satisfies never
      }
    })
    if (form === 'post')
      channel.postMessage = () => {
        throw new Error('post failed')
      }
    let attempts = 0
    const owner = createMdxIsolateOwner('/project', () => {
      attempts++
      if (form === 'startup') throw new Error('startup failed')
      return channel
    })
    const signal = new AbortController().signal
    try {
      // When
      await owner.prepare(
        { ...demand, timeoutMs: form === 'timeout' ? 1 : 1000 },
        signal,
      )
      await owner.prepare({ ...demand, run: 'later' }, signal)
      // Then
      await expect(owner.bind(binding)).rejects.toThrow()
      expect(attempts).toBe(1)
    } finally {
      await owner.dispose()
    }
  },
)

it('retains abort before demand without creating any worker', async () => {
  // Given
  const controller = new AbortController()
  controller.abort(new Error('original abort'))
  const owner = createMdxIsolateOwner('/project', () => {
    throw new Error('must stay dormant')
  })
  try {
    // When
    await owner.prepare(demand, controller.signal)
    // Then
    await expect(owner.bind(binding)).rejects.toThrow('original abort')
  } finally {
    await owner.dispose()
  }
})

it('refuses absent binding and disposed demand without worker creation', async () => {
  // Given
  const owner = createMdxIsolateOwner('/project', () => {
    throw new Error('must stay dormant')
  })
  await owner.dispose()
  // When
  await expect(owner.bind(binding)).rejects.toThrow('disposed')
  await owner.prepare(demand, new AbortController().signal)
  // Then
  await expect(owner.bind(binding)).rejects.toThrow('disposed')
})

it('refuses unprepared bindings both before demand and after quarantine without starting new work', async () => {
  // Given
  const channel = new Channel((owned) =>
    owned.emit('error', new Error('worker transport failed')),
  )
  let starts = 0
  const owner = createMdxIsolateOwner('/project', () => {
    starts++
    return channel
  })
  // When / Then
  await expect(owner.bind(binding)).rejects.toThrow('not prepared')
  expect(starts).toBe(0)
  await owner.prepare(demand, new AbortController().signal)
  await expect(
    owner.bind({ ...binding, filename: '/unprepared.mdx' }),
  ).rejects.toThrow('new build or dev-server restart')
  expect(starts).toBe(1)
  await owner.dispose()
})

it('retains wire-policy and same-run pipeline failures provisionally until binding', async () => {
  // Given
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => channel)
  const signal = new AbortController().signal
  await owner.prepare(demand, signal)
  try {
    // When
    await owner.prepare(
      { ...demand, pipeline: { ...pipeline, ruleKey: 'other' } },
      signal,
    )
    // Then
    await expect(owner.bind(binding)).rejects.toThrow('different pipeline')
  } finally {
    await owner.dispose()
  }
})

it('preserves the first located compile cause when termination also fails', async () => {
  // Given
  const channel = new Channel((owned) =>
    owned.emit('message', {
      kind: 'failed',
      error: {
        filename,
        line: 5,
        column: 2,
        message: 'original compile cause',
      },
    }),
  )
  channel.terminate = () => Promise.reject(new Error('cleanup cause'))
  const owner = createMdxIsolateOwner('/project', () => channel)
  // When
  await owner.prepare(demand, new AbortController().signal)
  // Then
  await expect(owner.bind(binding)).rejects.toThrow('original compile cause')
  await expect(owner.dispose()).rejects.toThrow('original compile cause')
})

it('retains a non-DATA refusal without creating a worker', async () => {
  // Given
  const owner = createMdxIsolateOwner('/project', () => {
    throw new Error('must remain dormant')
  })
  try {
    // When
    await owner.prepare(
      { ...demand, pipeline: { ...pipeline, aliases: { opaque: new Map() } } },
      new AbortController().signal,
    )
    // Then
    await expect(owner.bind(binding)).rejects.toThrow('plain DATA only')
  } finally {
    await owner.dispose()
  }
})

it('retains an in-flight abort and removes the job listener before cleanup', async () => {
  // Given
  const controller = new AbortController()
  const channel = new Channel(() =>
    controller.abort(new Error('in-flight abort')),
  )
  const owner = createMdxIsolateOwner('/project', () => channel)
  try {
    // When
    await owner.prepare(demand, controller.signal)
    // Then
    await expect(owner.bind(binding)).rejects.toThrow('in-flight abort')
    expect(channel.listenerCount('message')).toBe(0)
  } finally {
    await owner.dispose()
  }
})

it('awaits the exit event even when terminate resolves earlier', async () => {
  // Given
  const channel = new Channel()
  channel.terminate = () => Promise.resolve(0)
  const owner = createMdxIsolateOwner('/project', () => channel)
  await owner.prepare(demand, new AbortController().signal)
  let disposed = false
  // When
  const closing = owner.dispose().then(() => {
    disposed = true
  })
  await Promise.resolve()
  // Then
  expect(disposed).toBe(false)
  channel.emit('exit', 0)
  await closing
})

it.each(['dispose', 'retire'] as const)(
  'rejects an in-flight scope reply when %s closes binding admission first',
  async (form) => {
    // Given
    const channel = new Channel()
    const owner = createMdxIsolateOwner('/project', () => channel)
    await owner.prepare(demand, new AbortController().signal)
    const original = channel.postMessage.bind(channel)
    channel.postMessage = (value) => {
      if (isMdxRecord(value) && value.operation === 'check') {
        channel.emit('checking')
        return
      }
      original(value)
    }
    const checking = once(channel, 'checking')
    const pendingBinding = owner.bind(binding)
    const observed = pendingBinding.then(
      () => undefined,
      (error: unknown) => error,
    )
    await checking
    // When
    const closing = form === 'dispose' ? owner.dispose() : owner.retire('one')
    if (form === 'retire') channel.emit('message', { run: 'one', filename })
    // Then
    expect(await observed).toBeInstanceOf(Error)
    await closing
    await expect(owner.bind(binding)).rejects.toThrow(
      form === 'dispose' ? 'disposed' : 'retired',
    )
    await owner.dispose()
  },
)

it('does not erase a binding that completed before disposal', async () => {
  // Given
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => channel)
  await owner.prepare(demand, new AbortController().signal)
  const delivered = await owner.bind(binding)
  // When
  await owner.dispose()
  // Then
  expect(delivered).toEqual(canonical)
  await expect(owner.bind(binding)).rejects.toThrow('disposed')
})

it('binds only the resource-specific terminal success after cooperative quarantine', async () => {
  // Given
  const other = '/dormant/other.mdx'
  const channel = new Channel((owned) => {
    const request = owned.sent.at(-1)
    owned.emit(
      'message',
      isMdxRecord(request) && request.filename === other
        ? {
            kind: 'failed',
            error: {
              filename: other,
              line: 8,
              column: 2,
              message: 'other cause',
            },
            terminal: [{ run: 'one', filename }],
          }
        : prepared,
    )
  })
  const owner = createMdxIsolateOwner('/project', () => channel)
  const signal = new AbortController().signal
  await owner.prepare(demand, signal)
  // When
  await owner.prepare({ ...demand, filename: other }, signal)
  const output = await owner.bind(binding)
  // Then
  expect(output).toEqual(canonical)
  expect(channel.terminated).toBe(1)
  await expect(owner.bind({ ...binding, filename: other })).rejects.toThrow(
    'other cause',
  )
  await owner.dispose()
})

it('retains a failed scope check as its own fatal terminal verdict', async () => {
  // Given
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => channel)
  await owner.prepare(demand, new AbortController().signal)
  const failure = { filename, line: 1, column: 1, message: 'own scope changed' }
  channel.postMessage = () =>
    queueMicrotask(() =>
      channel.emit('message', {
        run: 'one',
        filename,
        error: failure,
        terminal: [{ run: 'one', filename, error: failure }],
      }),
    )
  // When
  const error = await owner.bind(binding).then(
    () => undefined,
    (cause: unknown) => cause,
  )
  // Then
  expect(error).toMatchObject({ failure })
  expect(channel.terminated).toBe(1)
  await expect(owner.bind(binding)).rejects.toThrow('own scope changed')
  await owner.dispose()
})

it('releases a retired run after a mismatched worker acknowledgement fails closed', async () => {
  // Given
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => channel)
  await owner.prepare(demand, new AbortController().signal)
  channel.postMessage = () =>
    queueMicrotask(() =>
      channel.emit('message', { run: 'other', filename: '/project' }),
    )
  // When
  const error = await owner.retire('one').then(
    () => undefined,
    (cause: unknown) => cause,
  )
  // Then
  expect(error).toBeInstanceOf(Error)
  await expect(owner.bind(binding)).rejects.toThrow('retired')
  expect(channel.terminated).toBe(1)
  await owner.dispose()
})

it.each([true, false])(
  'seals accepted native work after a parent DATA refusal, final response delivered=%s',
  async (delivered) => {
    // Given
    const channel = new Channel()
    const owner = createMdxIsolateOwner('/project', () => channel)
    const signal = new AbortController().signal
    await owner.prepare(demand, signal)
    channel.postMessage = () => {
      if (!delivered) throw new Error('final channel unavailable')
      queueMicrotask(() =>
        channel.emit('message', {
          run: 'one',
          filename: '/dormant/invalid.mdx',
          terminal: [{ run: 'one', filename }],
        }),
      )
    }
    // When
    await owner.prepare(
      {
        ...demand,
        filename: '/dormant/invalid.mdx',
        pipeline: { ...pipeline, aliases: { opaque: new Date() } },
      },
      signal,
    )
    // Then
    if (delivered) expect(await owner.bind(binding)).toEqual(canonical)
    else await expect(owner.bind(binding)).rejects.toThrow('no new publication')
    expect(channel.terminated).toBe(1)
    await owner.dispose()
  },
)

it('quarantines a fresh parent physical-source refusal rather than accepting a stale successful scope verdict', async () => {
  // Given
  const channel = new Channel((owned) =>
    owned.emit('message', {
      ...prepared,
      physical: [{ loader: 'unrecognized', module: 'old-module' }],
    }),
  )
  const owner = createMdxIsolateOwner('/project', () => channel)
  await owner.prepare(demand, new AbortController().signal)
  // When
  const error = await owner.bind(binding).then(
    () => undefined,
    (cause: unknown) => cause,
  )
  // Then
  expect(error).toMatchObject({
    failure: {
      filename,
      message: expect.stringContaining(
        'physical compiler source recognition changed',
      ),
    },
  })
  expect(channel.terminated).toBe(1)
  await owner.dispose()
})

it.each([
  { finalize: true, terminate: false },
  { finalize: false, terminate: true },
  { finalize: true, terminate: true },
])(
  'preserves physical-source first cause across secondary failures %j',
  async (fault) => {
    // Given
    const channel = new Channel((owned) =>
      owned.emit('message', {
        ...prepared,
        physical: [{ loader: 'unrecognized', module: 'old-module' }],
      }),
    )
    const owner = createMdxIsolateOwner('/project', () => channel)
    await owner.prepare(demand, new AbortController().signal)
    const sealFailure = new Error('secondary finalize cause')
    const cleanupFailure = new Error('secondary terminate cause')
    const operations: string[] = []
    const originalPost = channel.postMessage.bind(channel)
    const originalTerminate = channel.terminate.bind(channel)
    channel.postMessage = (value) => {
      if (isMdxRecord(value) && value.operation === 'finalize') {
        operations.push('finalize')
        if (fault.finalize) throw sealFailure
      }
      originalPost(value)
    }
    channel.terminate = async () => {
      operations.push('terminate')
      if (fault.terminate) {
        channel.emit('exit', 1)
        throw cleanupFailure
      }
      return originalTerminate()
    }
    let publications = 0
    // When
    const failure = await owner.bind(binding).then(
      () => {
        publications++
      },
      (cause: unknown) => cause,
    )
    // Then
    expect(publications).toBe(0)
    expect(operations).toEqual(['finalize', 'terminate'])
    expect(failure).toMatchObject({
      failure: {
        filename,
        line: 1,
        column: 1,
        message: expect.stringContaining(
          'physical compiler source recognition changed',
        ),
      },
    })
    expect(failure instanceof Error && failure.cause).toMatchObject({
      errors: [
        ...(fault.finalize ? [sealFailure] : []),
        ...(fault.terminate ? [cleanupFailure] : []),
      ],
    })
    const closing = await owner.dispose().then(
      () => undefined,
      (cause: unknown) => cause,
    )
    if (fault.terminate)
      expect(closing).toMatchObject({ failure: { filename } })
    else expect(closing).toBeUndefined()
  },
)

it('keeps quarantine when retirement closes a negative scope check', async () => {
  // Given
  const channel = new Channel()
  const owner = createMdxIsolateOwner('/project', () => channel)
  const signal = new AbortController().signal
  await owner.prepare(demand, signal)
  await owner.prepare({ ...demand, run: 'two' }, signal)
  const original = channel.postMessage.bind(channel)
  channel.postMessage = (value) => {
    if (
      isMdxRecord(value) &&
      value.operation === 'check' &&
      value.run === 'one'
    ) {
      channel.emit('checking')
      return
    }
    original(value)
  }
  const checking = once(channel, 'checking')
  const pending = owner.bind(binding).then(
    () => undefined,
    (cause: unknown) => cause,
  )
  await checking
  try {
    // When
    const retiring = owner.retire('one')
    const error = {
      filename,
      line: 1,
      column: 1,
      message: 'scope changed while retiring',
    }
    channel.emit('message', {
      run: 'one',
      filename,
      error,
      terminal: [
        { run: 'one', filename, error },
        { run: 'two', filename },
      ],
    })
    await retiring
    // Then
    expect(await pending).toBeInstanceOf(Error)
    expect(channel.terminated).toBe(1)
    expect(await owner.bind({ ...binding, run: 'two' })).toEqual(canonical)
  } finally {
    await owner.dispose()
  }
})

it.each(['check', 'scope', 'parent'] as const)(
  'preserves the first %s refusal when cleanup also fails',
  async (form) => {
    // Given
    const channel = new Channel()
    const owner = createMdxIsolateOwner('/project', () => channel)
    const signal = new AbortController().signal
    await owner.prepare(demand, signal)
    const failure = {
      filename,
      line: 1,
      column: 1,
      message: 'first scope refusal',
    }
    channel.terminate = () =>
      Promise.reject(new Error('secondary cleanup cause'))
    channel.postMessage = (value) => {
      if (!isMdxRecord(value)) throw new TypeError('expected control')
      queueMicrotask(() => {
        switch (form) {
          case 'check':
            channel.emit('message', { run: 'wrong', filename })
            return
          case 'scope':
            channel.emit('message', { run: 'one', filename, error: failure })
            return
          case 'parent':
            channel.emit('message', {
              run: value.run,
              filename: value.filename,
            })
            return
          default:
            return form satisfies never
        }
      })
    }
    // When
    let observed: unknown
    switch (form) {
      case 'parent':
        await owner.prepare(
          {
            ...demand,
            filename: '/invalid.mdx',
            pipeline: { ...pipeline, aliases: { opaque: new Date() } },
          },
          signal,
        )
        observed = await owner
          .bind({ ...binding, filename: '/invalid.mdx' })
          .then(
            () => undefined,
            (cause: unknown) => cause,
          )
        break
      case 'check':
      case 'scope':
        observed = await owner.bind(binding).then(
          () => undefined,
          (cause: unknown) => cause,
        )
        break
      default:
        return form satisfies never
    }
    // Then
    expect(observed).toBeInstanceOf(Error)
    expect(observed instanceof Error && observed.message).not.toContain(
      'secondary cleanup cause',
    )
    await expect(owner.dispose()).rejects.toThrow()
  },
)

it.each(['compile', 'check', 'finalize'] as const)(
  'never binds a terminal verdict for an unregistered resource in %s',
  async (form) => {
    // Given
    const failure = {
      filename,
      line: 1,
      column: 1,
      message: 'fatal compiler refusal',
    }
    const terminal = [{ run: 'unregistered', filename }]
    const channel = new Channel((owned) =>
      owned.emit(
        'message',
        form === 'compile'
          ? { kind: 'failed', error: failure, terminal }
          : prepared,
      ),
    )
    const owner = createMdxIsolateOwner('/project', () => channel)
    const signal = new AbortController().signal
    await owner.prepare(demand, signal)
    channel.postMessage = (value) => {
      if (!isMdxRecord(value)) throw new TypeError('expected control')
      queueMicrotask(() =>
        channel.emit('message', {
          run: value.run,
          filename: value.filename,
          error: form === 'check' ? failure : undefined,
          terminal,
        }),
      )
    }
    try {
      // When
      if (form === 'finalize')
        await owner.prepare(
          {
            ...demand,
            filename: '/invalid.mdx',
            pipeline: { ...pipeline, aliases: { opaque: new Date() } },
          },
          signal,
        )
      let publications = 0
      const error = await owner.bind(binding).then(
        () => {
          publications++
        },
        (cause: unknown) => cause,
      )
      // Then
      expect(error).toBeInstanceOf(Error)
      expect(publications).toBe(0)
      expect(channel.terminated).toBe(1)
    } finally {
      await owner.dispose()
    }
  },
)
