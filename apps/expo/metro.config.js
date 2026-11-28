const { getDefaultConfig } = require('expo/metro-config')
const { withDevupUI } = require('@devup-ui/metro-plugin')

module.exports = withDevupUI(getDefaultConfig(__dirname))
