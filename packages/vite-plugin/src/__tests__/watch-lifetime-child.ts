import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

type LifetimeRecord = {
  readonly action: string
  readonly id: number | undefined
  readonly complete: number | undefined
  readonly hold: number | undefined
  readonly owner: string | undefined
  readonly afterFinal: boolean | undefined
  readonly pending: readonly unknown[] | undefined
}

function parseRecord(line: string): LifetimeRecord {
  const row: unknown = JSON.parse(line)
  assert.ok(typeof row === 'object' && row !== null)
  assert.ok('action' in row && typeof row.action === 'string')
  return {
    action: row.action,
    id: 'id' in row && typeof row.id === 'number' ? row.id : undefined,
    complete:
      'complete' in row && typeof row.complete === 'number'
        ? row.complete
        : undefined,
    hold: 'hold' in row && typeof row.hold === 'number' ? row.hold : undefined,
    owner:
      'owner' in row && typeof row.owner === 'string' ? row.owner : undefined,
    afterFinal:
      'afterFinal' in row && typeof row.afterFinal === 'boolean'
        ? row.afterFinal
        : undefined,
    pending:
      'pending' in row && Array.isArray(row.pending) ? row.pending : undefined,
  }
}

// This child proves lifetime behavior only; it does not supply parent coverage.
export function runWatchLifetime() {
  const root = resolve(import.meta.dir, '../../../..')
  const owners = [
    'resolution-watch.test.ts',
    'resolution-watch-environments.test.ts',
  ].map((file) => join(import.meta.dir, file).replaceAll('\\', '/'))
  const fixture = mkdtempSync(join(tmpdir(), 'vite-watch-lifetime-'))
  const trace = join(fixture, 'lifetime.jsonl')
  const observer = join(fixture, 'observer.mjs')
  const config = join(fixture, 'bunfig.toml')
  try {
    writeFileSync(trace, '')
    writeFileSync(
      observer,
      `import { appendFileSync } from 'node:fs'
import { afterAll, afterEach } from 'bun:test'
const owners = ${JSON.stringify(owners)}
const emit = record => appendFileSync(${JSON.stringify(trace)}, JSON.stringify(record) + '\\n')
const pending = new Map()
const outputs = new Set()
const NativeRegistry = globalThis.FinalizationRegistry
const timeout = globalThis.setTimeout
const clear = globalThis.clearTimeout
const restoreRegistries = []
let outputSequence = 0
let sequence = 0
let complete = 0
let final = false
let restored = false
const restore = () => {
  if (restored) return
  restored = true
  globalThis.setTimeout = timeout
  globalThis.clearTimeout = clear
  globalThis.FinalizationRegistry = NativeRegistry
  for (const restoreRegistry of restoreRegistries) restoreRegistry()
  emit({ action: 'restored' })
}
globalThis.FinalizationRegistry = new Proxy(NativeRegistry, {
  construct(target, args, newTarget) {
    const tokens = new WeakMap()
    const pointers = new Map()
    const callback = args[0]
    const registry = Reflect.construct(target, [function (...values) {
      const id = pointers.get(values[0])
      if (id !== undefined) {
        pointers.delete(values[0])
        emit({ action: 'output-finalize', id, afterFinal: final })
      }
      return Reflect.apply(callback, this, values)
    }], newTarget)
    const register = registry.register
    const unregister = registry.unregister
    restoreRegistries.push(() => {
      registry.register = register
      registry.unregister = unregister
    })
    registry.register = new Proxy(register, {
      apply(method, receiver, values) {
        const result = Reflect.apply(method, receiver, values)
        const stack = new Error().stack
        if (values[0].constructor.name === 'Output' && /bindings[\\\\/]devup-ui-wasm[\\\\/]pkg[\\\\/]index.js/.test(stack)) {
          const id = ++outputSequence
          pointers.set(values[1], id)
          tokens.set(values[2], { id, pointer: values[1] })
          outputs.add(id)
          emit({ action: 'output-register', id, stack })
        }
        return result
      },
    })
    registry.unregister = new Proxy(unregister, {
      apply(method, receiver, values) {
        const result = Reflect.apply(method, receiver, values)
        const record = tokens.get(values[0])
        if (result && record) {
          pointers.delete(record.pointer)
          tokens.delete(values[0])
          outputs.delete(record.id)
          emit({ action: 'output-unregister', id: record.id, afterFinal: final })
        }
        return result
      },
    })
    return registry
  },
})
globalThis.setTimeout = new Proxy(timeout, {
  apply(target, receiver, args) {
    const [callback, duration, ...rest] = args
    const stack = new Error().stack.replaceAll('\\\\', '/')
    const owner = owners.find(file => stack.includes(file + ':'))
    if (!owner || typeof callback !== 'function') return Reflect.apply(target, receiver, args)
    const record = { id: ++sequence, owner, duration }
    let timer
    timer = Reflect.apply(target, receiver, [function (...values) {
      pending.delete(timer)
      emit({ action: 'fire', afterFinal: final, ...record })
      return Reflect.apply(callback, this, values)
    }, duration, ...rest])
    pending.set(timer, record)
    emit({ action: 'register', ...record })
    return timer
  },
})
globalThis.clearTimeout = new Proxy(clear, {
  apply(target, receiver, args) {
    const record = pending.get(args[0])
    if (record) { pending.delete(args[0]); emit({ action: 'clear', ...record }) }
    return Reflect.apply(target, receiver, args)
  },
})
afterEach(async () => {
  emit({ action: 'body-complete', complete: ++complete })
  if (complete !== 3) return
  try {
    final = true
    emit({ action: 'final', pending: [...pending.values()] })
    emit({ action: 'output-final', pending: [...outputs] })
    const started = performance.now()
    await new Promise(done => timeout(done, 11000))
    emit({ action: 'held', hold: performance.now() - started })
  } finally {
    restore()
  }
}, 120000)
afterAll(restore, 120000)
process.once('exit', restore)
`,
    )
    writeFileSync(
      join(fixture, 'original-watch.test.mjs'),
      owners.map((file) => `await import(${JSON.stringify(file)})`).join('\n'),
    )
    writeFileSync(
      config,
      `[test]\nroot = ${JSON.stringify(fixture.replaceAll('\\', '/'))}\npreload = ${JSON.stringify([observer.replaceAll('\\', '/'), join(root, 'bun.matrix.ts').replaceAll('\\', '/')])}\n`,
    )
    // No --isolate: native isolation could conceal leaked timers or Outputs.
    const child = spawnSync(process.execPath, [`--config=${config}`, 'test'], {
      cwd: fixture,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 120000,
    })
    if (child.error) throw child.error
    const records = readFileSync(trace, 'utf8')
      .trim()
      .split('\n')
      .map(parseRecord)
    const output = child.stdout + child.stderr
    console.info(
      'Native watch lifetime evidence',
      JSON.stringify({ status: child.status, output, records }),
    )
    return { status: child.status, output, records, owners }
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}
