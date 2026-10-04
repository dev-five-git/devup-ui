import { join, relative } from 'node:path'

import { createNodeModulesExcludeRegex } from '@devup-ui/plugin-utils'

import type { AppContext, AppSession } from './session'
import type { TurboRules } from './setup-handoff'

/** Longer than the coordinator's own production wait, which fails first. */
export const REQUEST_TIMEOUT_MS = 120_000

/**
 * Every extension Turbopack compiles as application code. The import graph and
 * the numbering scan list a subset of them until the shared source filter
 * grows the modern ones (fix/plugin-core).
 */
export const SOURCE_RULE = '*.{tsx,ts,jsx,js,mjs,mts,cts,cjs}'

/**
 * Next types loader options as JSON without `null`, but `null` survives the
 * hand-over and is how an alias means "named exports, one to one".
 */
function loaderAliases(context: AppContext): Record<string, string> {
  return { ...context.importAliases } as unknown as Record<string, string>
}

/** The theme as the plain JSON a loader option has to be. */
function structuredJson(value: object): ReturnType<typeof JSON.parse> {
  return JSON.parse(JSON.stringify(value))
}

interface RuleFields {
  context: AppContext
  session: AppSession
  /** The devup config and everything it extends, absolute */
  themeFiles: readonly string[]
  theme: object
}

/**
 * The loader rules of one app. Every path is absolute and every option comes
 * from the captured context, so a rule means the same thing whatever the
 * working directory is when Turbopack runs it.
 */
export function createTurboRules({
  context,
  session,
  themeFiles,
  theme,
}: RuleFields): TurboRules {
  const shared = {
    coordinatorPortFile: session.endpointFile,
    coordinatorIdentity: { ...session.identity },
    projectRoot: context.root,
    themeFiles: [...themeFiles],
    requestTimeoutMs: REQUEST_TIMEOUT_MS,
    watch: context.watch,
    themeFile: context.devupFile,
    theme: structuredJson(theme),
    // The coordinator owns persistence; the loaders only fall back to these
    // without one, and the plugin always starts one.
    sheetFile: join(session.sessionDir, 'sheet.json'),
    classMapFile: join(session.sessionDir, 'classMap.json'),
    fileMapFile: join(session.sessionDir, 'fileMap.json'),
    defaultSheet: {},
    defaultClassMap: {},
    defaultFileMap: {},
    ...(context.watch ? { revisionFile: session.revisionFile } : {}),
  }
  return {
    [`./${relative(context.root, context.cssDir).replaceAll('\\', '/')}/*.css`]:
      [{ loader: '@devup-ui/next-plugin/css-loader', options: shared }],
    [SOURCE_RULE]: {
      loaders: [
        {
          loader: '@devup-ui/next-plugin/loader',
          options: {
            ...shared,
            package: context.libPackage,
            cssDir: context.cssDir,
            singleCss: context.singleCss,
            importAliases: loaderAliases(context),
          },
        },
      ],
      condition: {
        not: {
          path: createNodeModulesExcludeRegex(
            [...context.include],
            '.mdx.[tj]sx?$',
          ),
        },
      },
    },
  }
}
