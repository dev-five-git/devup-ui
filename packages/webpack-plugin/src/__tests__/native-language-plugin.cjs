module.exports = {
  apply(compiler) {
    compiler.hooks.normalModuleFactory.tap('OpaqueLanguage', (factory) => {
      const parser = {
        hooks: {},
        parse(_source, state) {
          return state
        },
      }
      for (const type of ['javascript/auto', 'opaque-language'])
        factory.hooks.createParser.for(type).tap('OpaqueLanguage', () => parser)
      factory.hooks.createGenerator
        .for('opaque-language')
        .tap('OpaqueLanguage', () =>
          factory.getGenerator('javascript/auto', {}),
        )
    })
  },
}
