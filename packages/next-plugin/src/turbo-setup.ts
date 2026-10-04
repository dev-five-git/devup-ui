import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import {
  collectDevupConfigFiles,
  createCompatTypes,
  createThemeInterfaceArgs,
  loadDevupConfigSync,
  seedFileNumbers,
} from '@devup-ui/plugin-utils'
import type { NextConfig } from 'next'

import { startCoordinator } from './coordinator'
import { stampFile } from './coordinator-ledger'
import { persistInitialState, resumeEngine } from './dev-state'
import { createEngineConfigurer } from './engine-config'
import { installAfterCompileDrain, retainSession } from './lifecycle'
import { planSources, recoverPlanning, type SourcePlan } from './plan'
import { collectPrewarmFiles } from './prewarm'
import { type PrewarmResult, runPrewarm } from './prewarm-run'
import { elapsedMs, profileStart, reportProfile } from './profile'
import {
  type AppContext,
  type AppOptionsInput,
  createAppContext,
  createSession,
  digest,
  pruneDeadSessions,
} from './session'
import {
  consumeSetupHandoff,
  type SetupHandoff,
  storeSetupHandoff,
} from './setup-handoff'
import { createTurboRules } from './turbo-rules'
import { createWasm, type DevupWasm } from './wasm'

type SetupResult = Omit<SetupHandoff, 'key'>

function prepareDirectories(context: AppContext): void {
  mkdirSync(context.distDir, { recursive: true })
  mkdirSync(context.cssDir, { recursive: true })
  const gitignore = join(context.distDir, '.gitignore')
  if (!existsSync(gitignore)) writeFileSync(gitignore, '*')
  writeFileSync(
    join(context.distDir, 'compat.d.ts'),
    createCompatTypes({ ...context.importAliases }),
  )
  // Turbopack resolves every generated stylesheet import to this file, and the
  // CSS loader replaces its content with what the coordinator serves. The file
  // itself never carries live CSS, so its content never changes.
  const placeholder = join(context.cssDir, 'devup-ui.css')
  if (!existsSync(placeholder)) writeFileSync(placeholder, '')
}

function writeThemeTypes(context: AppContext, engine: DevupWasm): void {
  const themeInterface = engine.getThemeInterface(
    ...createThemeInterfaceArgs(context.libPackage),
  )
  if (themeInterface) {
    writeFileSync(join(context.distDir, 'theme.d.ts'), themeInterface)
  }
}

interface PrewarmFields {
  context: AppContext
  engine: DevupWasm
  plan: SourcePlan
}

/** Extract the compiled closure in path order, in development too. */
function prewarm({ context, engine, plan }: PrewarmFields): PrewarmResult {
  const { graph } = plan
  if (!graph) return { files: [], outputs: new Map() }
  const collectStartedAt = performance.now()
  const files = recoverPlanning({
    context,
    file: context.root,
    what: 'devup-ui prewarm',
    code: 'collectPrewarmFiles',
    needs: 'resolvable included packages and readable sources',
    lost: 'extracting the packages the app imports before the bundler asks',
    fallback: plan.expectedBaseFiles,
    work: () =>
      collectPrewarmFiles({
        root: context.root,
        graph,
        expectedBaseFiles: plan.expectedBaseFiles,
        libPackage: context.libPackage,
        include: context.include,
        prewarmAll: context.prewarmAll,
      }),
  })
  return runPrewarm({
    context,
    engine,
    files,
    collectMs: elapsedMs(collectStartedAt),
  })
}

function buildSetup(
  context: AppContext,
  configFiles: readonly string[],
): SetupResult {
  prepareDirectories(context)
  const session = createSession(context)
  mkdirSync(session.sessionDir, { recursive: true })
  pruneDeadSessions(context)

  const theme = loadDevupConfigSync(context.devupFile).theme ?? {}
  const plan = planSources(context)
  const configure = createEngineConfigurer(context, { theme, plan })
  const { engine, revision } = resumeEngine({
    context,
    session,
    createEngine: () => createWasm(context.root),
  })
  // Everything is configured and numbered before the first extraction, so the
  // names depend on the paths and not on the order requests arrive in.
  configure(engine)
  writeThemeTypes(context, engine)
  seedFileNumbers(engine, plan.seedFiles)
  const prewarmed = prewarm({ context, engine, plan })
  if (context.watch) {
    persistInitialState({
      context,
      session,
      engine,
      revision,
      prewarm: prewarmed,
    })
  }

  const coordinator = startCoordinator({
    wasm: engine,
    createEngine: () => createWasm(context.root),
    configureWasm: configure,
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: context.singleCss,
    importAliases: context.importAliases,
    coordinatorPortFile: session.endpointFile,
    projectRoot: context.root,
    identity: session.identity,
    watch: context.watch,
    optionsKey: context.appKey,
    devupFile: context.devupFile,
    sourceRoots: [...context.sourceRoots],
    canonicalMap: plan.canonicalMap,
    expectedBaseFiles: plan.expectedBaseFiles,
    prewarmedFiles: prewarmed.files,
    prewarmedOutputs: prewarmed.outputs,
    sourceMap: context.sourceMap,
    // A production build keeps nothing between runs and must not re-export the
    // whole sheet for every extraction.
    ...(context.watch
      ? { stateFile: session.stateFile, revisionFile: session.revisionFile }
      : {}),
  })
  retainSession({ session, coordinator })

  return {
    defaultTheme: engine.getDefaultTheme(),
    rules: createTurboRules({
      context,
      session,
      themeFiles: configFiles,
      theme,
    }),
    prewarmedFiles: prewarmed.files.length,
    sessionToken: session.token,
  }
}

/** Set up Turbopack for one app: its engine, coordinator and loader rules. */
export function setupTurbopack(
  config: NextConfig,
  options: AppOptionsInput,
): NextConfig {
  const startedAt = profileStart()
  config ??= {}
  config.turbopack ??= {}
  config.turbopack.rules ??= {}
  const context = createAppContext(config, options)
  const configFiles = collectDevupConfigFiles(context.devupFile)
  const key = digest({
    app: context.appKey,
    config: configFiles.map((file) => [file, stampFile(file)]),
  })
  const handed = context.watch ? undefined : consumeSetupHandoff(context, key)
  const setup = handed ?? buildSetup(context, configFiles)
  if (!handed && !context.watch) storeSetupHandoff(context, { key, ...setup })

  config.env ??= {}
  // Per app: an app without a theme must not inherit another app's.
  Object.assign(config.env, {
    DEVUP_UI_DEFAULT_THEME: setup.defaultTheme ?? 'default',
  })
  Object.assign(config.turbopack.rules, setup.rules)
  if (!context.watch) installAfterCompileDrain(config, setup.sessionToken)
  reportProfile('next.setup', {
    cacheHit: handed !== undefined,
    durationMs: elapsedMs(startedAt),
    pid: process.pid,
    prewarmedFiles: setup.prewarmedFiles,
    singleCss: context.singleCss,
    watch: context.watch,
  })
  return config
}
