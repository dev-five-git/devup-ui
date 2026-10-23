const assert = require('node:assert/strict')
const { Session } = require('node:inspector')
const { dirname, resolve } = require('node:path')

/** Only reads the exact closer's own listener, never a neighboring container listener.
 * @param {Function} closer
 */
function directoryScope(closer) {
  const session = new Session()
  session.connect()
  Reflect.set(globalThis, '__ProvenanceRegistrationCloser', closer)
  /** @param {string} objectId */
  function properties(objectId) {
    /** @type {import('node:inspector').Runtime.GetPropertiesReturnType | undefined} */
    let result
    session.post(
      'Runtime.getProperties',
      { objectId, ownProperties: true },
      (error, response) => {
        if (error) throw error
        result = response
      },
    )
    assert(result, 'local inspector must complete synchronously')
    return [...result.result, ...(result.internalProperties ?? [])]
  }
  /** @param {string} objectId @param {string} description */
  function scope(objectId, description) {
    const scopes = properties(objectId).find(
      (item) => item.name === '[[Scopes]]',
    )?.value?.objectId
    assert(scopes)
    return properties(scopes).find(
      (item) => item.value?.description === description,
    )?.value?.objectId
  }
  try {
    /** @type {string | undefined} */
    let id
    session.post(
      'Runtime.evaluate',
      {
        expression: 'globalThis.__ProvenanceRegistrationCloser',
        objectGroup: 'provenance-registration',
      },
      (error, response) => {
        if (error) throw error
        assert(!response.exceptionDetails)
        id = response.result.objectId
      },
    )
    assert(id)
    const registration = scope(id, 'Closure (setFsWatchListener)')
    if (!registration) return
    const listener = properties(registration).find(
      (item) => item.name === 'listener',
    )?.value?.objectId
    assert(listener)
    const directory = scope(listener, 'Closure (_handleDir)')
    if (!directory) return
    const values = properties(directory)
    const dir = values.find((item) => item.name === 'dir')?.value?.value
    const target = values.find((item) => item.name === 'target')?.value
    assert(typeof dir === 'string' && target)
    assert(target.type === 'undefined' || target.type === 'string')
    return {
      dir,
      target: target.type === 'undefined' ? undefined : String(target.value),
    }
  } finally {
    session.post('Runtime.releaseObjectGroup', {
      objectGroup: 'provenance-registration',
    })
    Reflect.deleteProperty(globalThis, '__ProvenanceRegistrationCloser')
    session.disconnect()
  }
}

/** Installed solely by the outside-native scenario; no watch requests are authored here.
 * @param {import('vite').ViteDevServer['watcher']} watcher
 * @param {string} directory
 */
function registrationBarrier(watcher, directory) {
  const original = Reflect.get(watcher, '_addPathCloser')
  assert(typeof original === 'function')
  /** @type {PromiseWithResolvers<void>} */
  const unrestricted = Promise.withResolvers()
  /** @type {PromiseWithResolvers<void>} */
  const targeted = Promise.withResolvers()
  /** @type {{sequence:number,returnedSequence:number,signalSequence:number,path:string,dir:string,target:string|undefined,accepted:boolean,unrestrictedPending:boolean}[]} */
  const records = []
  let sequence = 0
  let accepted = false
  const hadOwn = Object.hasOwn(watcher, '_addPathCloser')
  const observer = new Proxy(original, {
    apply(method, receiver, args) {
      const returned = Reflect.apply(method, receiver, args)
      const [path, closer] = args
      if (typeof path !== 'string' || typeof closer !== 'function')
        return returned
      if (
        resolve(path) !== resolve(directory) &&
        resolve(dirname(path)) !== resolve(directory)
      )
        return returned
      const returnedSequence = ++sequence
      // Queue only observer work; the backend caller receives its real return first.
      queueMicrotask(() => {
        try {
          const closers = Reflect.get(watcher, '_closers')
          assert(
            closers instanceof Map &&
              closers
                .get(require('node:path').normalize(path))
                ?.includes(closer),
          )
          const value = directoryScope(closer)
          if (!value || resolve(value.dir) !== resolve(directory)) return
          const qualifies = value.target === undefined
          const classifiedSequence = ++sequence
          const signalSequence = ++sequence
          records.push({
            sequence: classifiedSequence,
            returnedSequence,
            signalSequence,
            path,
            ...value,
            accepted: qualifies,
            unrestrictedPending: !accepted,
          })
          if (qualifies) {
            accepted = true
            unrestricted.resolve()
          } else {
            targeted.resolve()
          }
        } catch (error) {
          unrestricted.reject(error)
          targeted.reject(error)
        }
      })
      return returned
    },
  })
  Reflect.set(watcher, '_addPathCloser', observer)
  return {
    ready: Promise.all([unrestricted.promise, targeted.promise]),
    records,
    /** @param {string} operation */
    beforeMutation(operation) {
      console.info(
        'provenance-native mutation ' +
          JSON.stringify({
            sequence: ++sequence,
            operation,
            releaseCause:
              'actual-unrestricted-and-targeted-registration-signals',
            records,
          }),
      )
    },
    assertReady() {
      assert(
        accepted,
        'only actual unrestricted registration releases membership',
      )
      assert(
        records.some((item) => item.target !== undefined && !item.accepted),
        'actual targeted registration must be rejected',
      )
    },
    restore() {
      if (hadOwn) Reflect.set(watcher, '_addPathCloser', original)
      else Reflect.deleteProperty(watcher, '_addPathCloser')
    },
  }
}
module.exports = { registrationBarrier }
