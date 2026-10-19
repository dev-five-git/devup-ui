const { addPair, put } = require('./reservation-fixture.cjs')

async function runWatch(compiler, state) {
  let complete
  const nextCompile = () =>
    new Promise((resolve, reject) => {
      complete = { resolve, reject }
    })
  const first = nextCompile()
  const watching = compiler.watch({ poll: 1000 }, (error, stats) => {
    error || stats.hasErrors()
      ? complete.reject(error || new Error(stats.toString()))
      : complete.resolve(stats)
  })
  try {
    await first
    watching.suspend()
    addPair(state.root, state.order)
    state.revise()
    put(state.root, 'src/constants.js', 'export const tone = "crimson"')
    const next = nextCompile()
    watching.resume()
    watching.invalidate()
    await next
    const supplier = nextCompile()
    put(state.root, 'src/constants.js', 'export const tone = "darkmagenta"')
    const stats = await supplier
    state.events.push('supplier-revision-complete')
    return stats
  } finally {
    await new Promise((resolve) => watching.close(resolve))
  }
}

module.exports = { runWatch }
