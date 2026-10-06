module.exports = function downstream(source) {
  return source
}
module.exports.pitch = function pitch() {
  return 'export default "replacement"'
}
