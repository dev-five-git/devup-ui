import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'

export async function observeRawControlBuild(fixture, policy) {
  const { root, page, description, widget, unrelated, landing, mdx } = fixture
  const { kind, request, singleCss } = policy
  const observed = []
  const externalCalls = []
  const parsed = new Set()
  const stages = { description: 0, unrelated: 0 }
  let graph = []
  let obligations = []
  let optionsSeen = false
  let failure
  let output
  const alias = kind === 'raw-alias' || kind === 'raw-alias-operative'
  const externalized = kind.startsWith('external-')
  const external = (id, importer, isResolved) => {
    const selected =
      importer === page &&
      (kind === 'external-unresolved'
        ? id === request && !isResolved
        : id === description && isResolved)
    if (importer === page) externalCalls.push({ id, isResolved, selected })
    return selected
  }
  try {
    // When the native build consumes the real raw policy (no resolver overrides).
    const built = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      resolve: {
        alias: [
          'react/jsx-runtime',
          'react/jsx-dev-runtime',
          'react',
          '@devup-ui/react',
        ].map((name) => ({
          find: name,
          replacement: landing.resolve(name),
        })),
      },
      plugins: [
        DevupUI({ singleCss }),
        mdx(),
        {
          name: 'raw-native-control-observer',
          enforce: 'post',
          options(options) {
            optionsSeen = alias
              ? options.resolve?.alias?.[request] === widget
              : !externalized || options.external === external
          },
          resolveId: {
            order: 'pre',
            handler(id, importer, options) {
              if (id === request && importer === page)
                observed.push({
                  kind: options.kind,
                  isEntry: options.isEntry,
                  custom: Object.keys(options.custom ?? {}),
                })
            },
          },
          transform(_code, id) {
            if (id === description) stages.description++
            if (id === unrelated) stages.unrelated++
          },
          moduleParsed(info) {
            parsed.add(info.id)
          },
          buildEnd: {
            order: 'pre',
            handler() {
              graph = [...this.getModuleIds()]
                .map((id) => {
                  const info = this.getModuleInfo(id)
                  if (
                    id === page &&
                    typeof info?.meta.devupDeferredDefaults === 'string'
                  )
                    obligations = JSON.parse(info.meta.devupDeferredDefaults)
                  return {
                    id,
                    codeNull: info?.code === null,
                    parsed: parsed.has(id),
                    importedIds: [...(info?.importedIds ?? [])].sort(),
                  }
                })
                .sort((a, b) => a.id.localeCompare(b.id))
            },
          },
        },
      ],
      build: {
        write: false,
        minify: false,
        rolldownOptions: {
          input: page,
          ...(alias
            ? { resolve: { alias: { [request]: widget } } }
            : externalized
              ? { external }
              : {}),
        },
      },
    })
    output = (Array.isArray(built) ? built : [built])
      .flatMap((result) => result.output)
      .map((item) =>
        item.type === 'chunk'
          ? {
              type: item.type,
              fileName: item.fileName,
              code: item.code,
              modules: item.modules,
            }
          : {
              type: item.type,
              fileName: item.fileName,
              source: item.source,
            },
      )
  } catch (cause) {
    if (!(cause instanceof Error)) throw cause
    failure = cause
  }
  return {
    observed,
    externalCalls,
    stages,
    graph,
    obligations,
    optionsSeen,
    failure,
    output,
    alias,
  }
}
