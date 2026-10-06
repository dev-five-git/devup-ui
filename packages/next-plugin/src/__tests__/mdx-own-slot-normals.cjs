exports.default = function (source) {
  return source
}
exports.downstream = function () {
  throw new Error('downstream normal executed')
}
exports.compiler = function () {
  throw new Error('must not run')
}
