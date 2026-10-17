function run(compiler) {
  return new Promise((resolve, reject) =>
    compiler.run((error, stats) => {
      if (error) reject(error)
      else if (stats.hasErrors())
        reject(new Error(stats.toString({ all: false, errors: true })))
      else resolve(stats)
    }),
  )
}

function close(compiler) {
  return new Promise((resolve, reject) =>
    compiler.close((error) => (error ? reject(error) : resolve())),
  )
}

module.exports = { run, close }
