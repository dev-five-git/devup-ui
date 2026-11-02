'use strict'
// Private loader: generated pkg/index.cjs is the only CommonJS public entry.
const { platform, arch } = process
let libc
if (platform === 'linux') {
  const report = process.report.getReport()
  libc = report.header.glibcVersionRuntime ? 'gnu' : 'musl'
}
let suffix
if (arch === 'x64' || arch === 'arm64') {
  switch (platform) {
    case 'linux':
      suffix = `${platform}-${arch}-${libc}`
      break
    case 'win32':
      suffix = `${platform}-${arch}-msvc`
      break
    case 'darwin':
      suffix = `${platform}-${arch}`
      break
  }
}
if (!suffix) {
  throw new Error(
    `Unsupported @devup-ui/binding target: ${platform}/${arch}/${libc ?? 'none'}`,
  )
}
const name = `@devup-ui/binding-${suffix}`
try {
  require.resolve(`${name}/package.json`)
} catch (error) {
  if (error.code !== 'MODULE_NOT_FOUND') throw error
  throw new Error(
    `Missing optional dependency ${name}. Reinstall @devup-ui/binding with optional dependencies enabled (do not use --omit=optional).`,
    { cause: error },
  )
}
// Do not catch corrupt images, missing transitive libraries or addon failures.
module.exports = require(name)
