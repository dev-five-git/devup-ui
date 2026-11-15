export function createOwnerRuntime(control) {
  const handles = new WeakMap()
  let site
  const callsite = (operation) => {
    const stack = new Error().stack ?? ''
    for (const row of stack.split('\n').slice(1)) {
      if (row.includes(__filename)) continue
      const found = row.match(/^\s*at (.+):(\d+):(\d+)(\)?)$/)
      if (found) {
        let file = found[1]
        if (found[4] === ')') {
          // Match the outer frame wrapper, not parentheses in its label or path.
          let depth = 1
          for (let index = file.length - 1; index >= 0; index--) {
            if (file[index] === ')') depth++
            else if (file[index] === '(') depth--
            if (depth === 0) {
              file = file.slice(index + 1)
              break
            }
          }
        }
        return {
          file,
          line: Number(found[2]),
          column: Number(found[3]),
          operation,
        }
      }
    }
    throw new Error('engine operation requires an actual source callsite')
  }
  const locate = (error, operation) => {
    const details = control._ownerFault(error)
    if (details === undefined) return error
    const [category, message, importer, cleanup, hasCause, cause] = details
    const origin =
      importer === undefined
        ? (site ?? callsite(operation))
        : { file: importer, line: 1, column: 1 }
    const reason =
      category === 'Reentry'
        ? 'finish the current synchronous engine operation before entering another'
        : message
    const located = new Error(
      `${origin.file}:${origin.line}:${origin.column}: build generation cannot use \`${operation}\` at build time: ${reason}`,
      { cause: hasCause === true ? cause : error },
    )
    located.name = 'ResidentOwnerError'
    if (cleanup !== undefined)
      Object.defineProperty(located, 'cleanupCause', {
        value: locate(cleanup, 'cleanup'),
        enumerable: true,
      })
    return located
  }
  const composite = (outcome, cleanup) => {
    const error = new Error(outcome.message ?? 'engine operation failed', {
      cause: outcome.error,
    })
    Object.defineProperty(error, 'cleanupCause', {
      value: cleanup,
      enumerable: true,
    })
    return error
  }
  const complete = (outcome, cleanup) => {
    try {
      cleanup()
    } catch (error) {
      const fault = locate(error, 'cleanup')
      if (outcome.failed) throw composite(outcome, fault)
      throw fault
    }
    if (outcome.failed) throw outcome.error
    return outcome.value
  }
  const invoke = (action, operation = site?.operation ?? 'engine') => {
    try {
      return action()
    } catch (error) {
      throw locate(error, operation)
    }
  }
  const capture = (action) => {
    try {
      return { failed: false, value: action() }
    } catch (error) {
      let message = 'engine operation failed'
      try {
        const diagnostic = error instanceof Error ? error.message : undefined
        if (typeof diagnostic === 'string') message = diagnostic
      } catch {
        message = 'engine operation failed (diagnostic formatting failed)'
      }
      return { failed: true, error, message }
    }
  }
  const id = (handle) => {
    const token = handles.get(handle)
    if (token === undefined) {
      try {
        control._ownerReject()
      } catch (error) {
        throw locate(error, 'foreign owner')
      }
      throw new Error('owner rejection unexpectedly returned')
    }
    return token
  }
  const ownership = Object.freeze({
    create(callback) {
      if (typeof callback !== 'function')
        throw new TypeError('capture must be a function')
      const token = complete(
        capture(() => invoke(() => control._ownerCreate(callback), 'create')),
        control._ownerDrain,
      )
      const handle = Object.freeze({
        get disposed() {
          const outcome = capture(() =>
            invoke(() => control._ownerDisposed(token), 'disposed'),
          )
          if (outcome.failed) throw outcome.error
          return outcome.value
        },
      })
      handles.set(handle, token)
      return handle
    },
    run(handle, request) {
      const token = id(handle)
      const previous = site
      try {
        control._ownerBegin(token)
      } catch (error) {
        return complete(
          { failed: true, error: locate(error, 'run') },
          control._ownerDrain,
        )
      }
      const outcome = capture(() => {
        const supplied = request.site
        const { file, line, column, operation } = supplied
        if (
          typeof file !== 'string' ||
          file.length === 0 ||
          !Number.isSafeInteger(line) ||
          line < 1 ||
          !Number.isSafeInteger(column) ||
          column < 1 ||
          typeof operation !== 'string'
        )
          throw new TypeError('run requires an actual site')
        site = { file, line, column, operation }
        request.configure()
        invoke(control._ownerConfigured)
        return request.action()
      })
      try {
        return complete(outcome, () =>
          complete(
            capture(() => invoke(control._ownerFinish)),
            control._ownerDrain,
          ),
        )
      } finally {
        site = previous
      }
    },
    dispose(handle) {
      const token = id(handle)
      return complete(
        capture(() => invoke(() => control._ownerDispose(token), 'dispose')),
        control._ownerDrain,
      )
    },
    clear(handle) {
      const token = id(handle)
      return complete(
        capture(() => invoke(() => control._ownerClear(token), 'clear')),
        control._ownerDrain,
      )
    },
  })
  return {
    ownership,
    wrap(original, opcode, operation) {
      return function (...args) {
        let ticket
        try {
          ticket = control._ownerEnterExport(opcode)
        } catch (error) {
          return complete(
            { failed: true, error: locate(error, operation) },
            control._ownerDrain,
          )
        }
        const outcome = capture(() =>
          invoke(() => original(...args), operation),
        )
        return complete(outcome, () =>
          complete(
            capture(() => invoke(() => control._ownerLeaveExport(ticket))),
            control._ownerDrain,
          ),
        )
      }
    },
  }
}
