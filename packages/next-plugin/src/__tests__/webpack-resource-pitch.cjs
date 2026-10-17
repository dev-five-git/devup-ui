module.exports = Object.assign(
  require('./webpack-resource-downstream.cjs').bind(null),
  {
    pitch() {
      return 'export default "replacement"'
    },
  },
)
