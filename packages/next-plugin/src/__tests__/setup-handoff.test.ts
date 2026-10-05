import * as fs from 'node:fs'
import { join } from 'node:path'
import { serialize } from 'node:v8'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import type { SessionOwner } from '../lifecycle'
import { type AppContext, createAppContext } from '../session'
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

const originalEnv = { ...process.env }
let sequence = 0
let owners: SessionOwner[] = []
let once: ReturnType<typeof spyOn>

beforeEach(() => {
  sequence += 1
  once = spyOn(process, 'once').mockImplementation(() => process)
  process.env = { ...process.env, NODE_ENV: 'production' }
  resetSetupHandoffsForTesting()
})
afterEach(() => {
  for (const owner of owners) owner.close()
  owners = []
  once.mockRestore()
  resetSetupHandoffsForTesting()
  process.env = { ...originalEnv }
})

function app(options = {}) {
  return createAppContext({}, options)
}

function handoff(key = 'key'): SetupHandoff {
  return {
    key,
    defaultTheme: 'dark',
    rules: {
      '*.ts': { loaders: [], condition: { not: { path: /node_modules/ } } },
    },
    prewarmedFiles: 3,
    sessionToken: `session-token-${sequence}`,
  }
}

function store(context: AppContext, setup: SetupHandoff): void {
  owners.push(makeSetupOwner(context, setup))
  storeSetupHandoff(context, setup)
}

describe('setup handoff', () => {
  it('hands a setup to one other module instance, once', () => {
    process.chdir(makeProject())
    const context = app()
    const original = handoff()
    store(context, original)
    expect(
      fs.existsSync(join(context.appDir, `handoff-${process.pid}.bin`)),
    ).toBe(true)

    reloadSetupModuleForTesting()
    const taken = consumeSetupHandoff(context, 'key')

    expect(taken).toMatchObject(handoff())
    expect(taken?.rules).toBe(original.rules)
    expect(taken?.rules['*.ts']).toMatchObject({
      condition: { not: { path: /node_modules/ } },
    })
    expect(
      fs.existsSync(join(context.appDir, `handoff-${process.pid}.bin`)),
    ).toBe(false)
    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
  })

  it('never lets a module instance take its own handoff', () => {
    process.chdir(makeProject())
    const context = app()
    store(context, handoff())

    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
    reloadSetupModuleForTesting()
    expect(consumeSetupHandoff(context, 'key')).toBeDefined()
  })

  it('takes nothing for other options, config contents or another app', () => {
    process.chdir(makeProject())
    const context = app()
    const other = app({ singleCss: true })
    store(context, handoff())
    reloadSetupModuleForTesting()

    expect(consumeSetupHandoff(context, 'another key')).toBeUndefined()
    expect(consumeSetupHandoff(other, 'key')).toBeUndefined()
    expect(consumeSetupHandoff(context, 'key')).toBeDefined()
  })

  it('keeps the handoffs of two apps apart', () => {
    process.chdir(makeProject())
    const first = app()
    const second = app({ prefix: 'two-' })
    store(first, { ...handoff(), sessionToken: 'first' })
    store(second, { ...handoff(), sessionToken: 'second' })
    reloadSetupModuleForTesting()

    expect(consumeSetupHandoff(second, 'key')?.sessionToken).toBe('second')
    expect(consumeSetupHandoff(first, 'key')?.sessionToken).toBe('first')
  })

  it('rejects a token that is not the one that was handed over', () => {
    process.chdir(makeProject())
    const context = app()
    store(context, handoff())
    reloadSetupModuleForTesting()
    const name = `DEVUP_UI_SETUP_TOKEN_${context.appKey}`
    const token = process.env[name]
    process.env[name] = `${token}-forged`

    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()

    process.env[name] = token
    expect(consumeSetupHandoff(context, 'key')).toBeDefined()
  })

  it('takes nothing without a token or with an unreadable file', () => {
    process.chdir(makeProject())
    const context = app()
    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()

    store(context, handoff())
    reloadSetupModuleForTesting()
    const file = join(context.appDir, `handoff-${process.pid}.bin`)
    fs.writeFileSync(file, 'not v8 data')
    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
  })

  it('takes nothing when the stored value has no handoff shape', () => {
    process.chdir(makeProject())
    const context = app()
    store(context, handoff())
    reloadSetupModuleForTesting()
    const file = join(context.appDir, `handoff-${process.pid}.bin`)
    for (const value of [
      null,
      1,
      { key: 'key' },
      { key: 'key', owner: 'o', token: 't' },
    ]) {
      fs.writeFileSync(file, serialize(value))
      expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
    }
  })

  it('stays one-use when the file cannot be deleted and says so', () => {
    process.chdir(makeProject())
    const context = app()
    store(context, handoff())
    reloadSetupModuleForTesting()
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const remove = spyOn(fs, 'rmSync').mockImplementation(() => {
      throw new Error('EPERM')
    })

    try {
      expect(consumeSetupHandoff(context, 'key')).toBeDefined()
      expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        `handoff-${process.pid}.bin:1:1: devup-ui cannot use \`the setup handoff file\``,
      )
    } finally {
      remove.mockRestore()
      warn.mockRestore()
    }
  })

  it('leaves no token behind when the handoff cannot be written', () => {
    // Given
    const root = makeProject({
      'df/.devup': 'a file where a directory belongs',
    })
    process.chdir(root)
    const context = app()
    process.env[`DEVUP_UI_SETUP_TOKEN_${context.appKey}`] = 'stale-token'

    // When: writing a handoff needs no live owner; consuming a valid one does.
    storeSetupHandoff(context, handoff())
    reloadSetupModuleForTesting()

    // Then
    expect(
      process.env[`DEVUP_UI_SETUP_TOKEN_${context.appKey}`],
    ).toBeUndefined()
    expect(
      fs.existsSync(join(context.appDir, `handoff-${process.pid}.bin`)),
    ).toBe(false)
    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
  })

  it('forgets every handoff of the process on reset', () => {
    process.chdir(makeProject())
    const context = app()
    store(context, handoff())

    resetSetupHandoffsForTesting()

    expect(consumeSetupHandoff(context, 'key')).toBeUndefined()
    expect(
      Object.keys(process.env).filter((name) =>
        name.startsWith('DEVUP_UI_SETUP_TOKEN_'),
      ),
    ).toEqual([])
  })
})
