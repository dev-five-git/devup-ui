const assert = require('node:assert/strict')
const { basename } = require('node:path')

// Only release genuine loader invocations; never replace their source or output.
const state = {
  order: [],
  waiting: new Map(),
  dispatched: [],
  transformed: {},
}

function gate(source) {
  const name = basename(this.resourcePath)
  if (!state.order.includes(name)) return source
  const callback = this.async()
  assert(!state.waiting.has(name))
  state.waiting.set(name, () => {
    state.dispatched.push(name)
    callback(null, source)
  })
  if (state.waiting.size === 2) state.waiting.get(state.order[0])()
}

function observe(source, map) {
  const name = basename(this.resourcePath)
  state.transformed[name] = source.toString()
  if (name === state.order[0]) state.waiting.get(state.order[1])()
  if (name === state.order[1]) state.waiting.clear()
  this.callback(null, source, map)
}

module.exports = gate
module.exports.observe = observe
module.exports.state = state
