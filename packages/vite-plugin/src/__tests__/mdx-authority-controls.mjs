import assert from 'node:assert/strict'

export function laterAuthorityControls(kind, fixture, decisions) {
  if (!kind.startsWith('later-')) return []
  const predecessor = {
    name: 'actual-options-predecessor',
    resolveId: {
      order: 'pre',
      async handler(request, importer, options) {
        if (request !== './Description.mdx') return
        const inner =
          kind === 'later-nested'
            ? await this.resolve(request, importer, {
                ...options,
                skipSelf: true,
              })
            : undefined
        decisions.push({ inner: inner?.id, outer: fixture.widget })
        return fixture.widget
      },
    },
  }
  return [
    {
      name: 'actual-later-options-insertion',
      enforce: 'post',
      options: {
        order: 'post',
        handler(options) {
          return { ...options, plugins: [predecessor, ...options.plugins] }
        },
      },
      moduleParsed(info) {
        if (info.id !== fixture.page) return
        assert(info.importedIds.includes(fixture.widget))
        assert(info.importedIds.includes(fixture.description))
        decisions.push({ graph: info.importedIds })
      },
    },
  ]
}
