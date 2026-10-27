const assert = require('node:assert/strict')

/**
 * @param {import('vite').Plugin} plugin
 * @param {(fact: {phase:'watchChange',file:string,type:'create'|'update'|'delete'} | {phase:'hotUpdate',context:import('vite').HotUpdateOptions}, environment: string, error: unknown) => void} observe
 */
function observeRefreshFailure(plugin, observe) {
  const watch = plugin.watchChange
  assert.equal(typeof watch, 'function')
  if (typeof watch !== 'function')
    throw new TypeError('Expected watchChange hook')
  plugin.watchChange = async function (file, change) {
    try {
      return await watch.call(this, file, change)
    } catch (error) {
      observe(
        { phase: 'watchChange', file, type: change.event },
        this.environment.name,
        error,
      )
      throw error
    }
  }
  const hook = plugin.hotUpdate
  assert.equal(typeof hook, 'function')
  if (typeof hook !== 'function') throw new TypeError('Expected hotUpdate hook')
  plugin.hotUpdate = async function (context) {
    try {
      return await hook.call(this, context)
    } catch (error) {
      observe({ phase: 'hotUpdate', context }, this.environment.name, error)
      throw error
    }
  }
}

/** @param {string} file @param {'ENOENT' | 'SyntaxError'} [reason] */
function configFailure(file, reason = 'ENOENT') {
  return (/** @type {unknown} */ error) => {
    assert(error instanceof Error)
    assert.equal(error.name, 'ConfigLoadError')
    assert.equal(Reflect.get(error, 'file'), file)
    assert(error.message.startsWith(`${file}:1:1:`))
    assert(error.cause instanceof Error)
    if (reason === 'ENOENT')
      assert.equal(Reflect.get(error.cause, 'code'), reason)
    else assert(error.cause instanceof SyntaxError)
    return true
  }
}

module.exports = { observeRefreshFailure, configFailure }
