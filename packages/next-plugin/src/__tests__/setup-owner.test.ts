import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { deserialize, serialize } from 'node:v8'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { findSessionOwner, type SessionOwner } from '../lifecycle'
import { createAppContext, createSession } from '../session'
import {
  consumeSetupHandoff,
  reloadSetupModuleForTesting,
  resetSetupHandoffsForTesting,
  type SetupHandoff,
  storeSetupHandoff,
} from '../setup-handoff'
import { installProjectHooks, makeProject } from './project'
import { makeSetupOwner } from './setup-owner-fixture'

installProjectHooks()
let owners: SessionOwner[] = []
let once: ReturnType<typeof spyOn>

beforeEach(() => {
  once = spyOn(process, 'once').mockImplementation(() => process)
  resetSetupHandoffsForTesting()
})
afterEach(() => {
  for (const owner of owners) owner.close()
  owners = []
  once.mockRestore()
  resetSetupHandoffsForTesting()
})

function setup() {
  process.chdir(makeProject())
  const context = createAppContext({}, {})
  const result: SetupHandoff = {
    key: 'key',
    defaultTheme: undefined,
    rules: {},
    prewarmedFiles: 0,
    sessionToken: createSession(context).token,
  }
  const owner = makeSetupOwner(context, result)
  owners.push(owner)
  storeSetupHandoff(context, result)
  reloadSetupModuleForTesting()
  return {
    context,
    result,
    owner,
    file: join(context.appDir, `handoff-${process.pid}.bin`),
  }
}

describe('live setup ownership', () => {
  it('keeps the engine and pending handle in the live owner when a descriptor is consumed', () => {
    // Given: the owned engine and coordinator are not serializable transport values.
    const { context, result, owner, file } = setup()
    const stored: unknown = deserialize(readFileSync(file))
    const original = findSessionOwner(result.sessionToken)
    // When: a reloaded module takes the descriptor.
    const taken = consumeSetupHandoff(context, result.key)
    // Then: the original live values survive, while the file contains only scalar descriptors.
    expect(taken?.rules).toBe(result.rules)
    expect(findSessionOwner(result.sessionToken)).toBe(original)
    expect(original?.setup?.engine).toBe(owner.setup?.engine)
    expect(original?.coordinator).toBe(owner.coordinator)
    if (typeof stored !== 'object' || stored === null) {
      throw new TypeError('handoff descriptor must be an object')
    }
    expect(
      Object.values(stored).every((value) => typeof value === 'string'),
    ).toBe(true)
  })

  it('rejects a descriptor after its owner is released', () => {
    // Given: a valid descriptor whose owner has closed.
    const { context, result, owner } = setup()
    owner.close()
    // When: another config evaluation consumes it.
    const taken = consumeSetupHandoff(context, result.key)
    // Then: it cannot point loaders at a closed coordinator.
    expect(taken).toBeUndefined()
  })

  it('rejects a descriptor after a production drain without waiting for process exit', async () => {
    // Given: the owner has completed its production drain.
    const { context, result, owner } = setup()
    await owner.drain()
    // When: the config is reloaded before its old exit callback runs.
    const taken = consumeSetupHandoff(context, result.key)
    // Then: transport ownership cannot be resurrected by a remaining descriptor.
    expect(taken).toBeUndefined()
  })

  it('rejects a descriptor for a missing owner', () => {
    // Given: a valid descriptor with no matching live token.
    const { context, result, file } = setup()
    const stored: unknown = deserialize(readFileSync(file))
    if (typeof stored !== 'object' || stored === null) {
      throw new TypeError('handoff descriptor must be an object')
    }
    writeFileSync(file, serialize({ ...stored, sessionToken: 'missing' }))
    // When: the nonce-authenticated descriptor is consumed.
    const taken = consumeSetupHandoff(context, result.key)
    // Then: serialized data alone cannot create an owner or engine.
    expect(taken).toBeUndefined()
  })

  it('rejects an owner from a different captured app even with matching serialized fields', () => {
    // Given: a descriptor points at another live app with different options.
    const { context, result, file } = setup()
    const other = createAppContext({}, { prefix: 'other-' })
    const otherResult = { ...result, sessionToken: createSession(other).token }
    owners.push(makeSetupOwner(other, otherResult))
    const stored: unknown = deserialize(readFileSync(file))
    if (typeof stored !== 'object' || stored === null) {
      throw new TypeError('handoff descriptor must be an object')
    }
    writeFileSync(
      file,
      serialize({ ...stored, sessionToken: otherResult.sessionToken }),
    )
    // When: the original app consumes its descriptor.
    const taken = consumeSetupHandoff(context, result.key)
    // Then: live ownership, not only the file's projection, decides reuse.
    expect(taken).toBeUndefined()
  })

  it.each(['root', 'phase', 'appKey'])(
    'rejects a changed %s in the descriptor',
    (field) => {
      // Given: valid ownership but a corrupted captured-app field.
      const { context, result, file } = setup()
      const stored: unknown = deserialize(readFileSync(file))
      if (typeof stored !== 'object' || stored === null) {
        throw new TypeError('handoff descriptor must be an object')
      }
      writeFileSync(file, serialize({ ...stored, [field]: 'changed' }))
      // When: the descriptor is consumed.
      const taken = consumeSetupHandoff(context, result.key)
      // Then: it cannot reuse the captured app through stale serialized settings.
      expect(taken).toBeUndefined()
    },
  )
})
