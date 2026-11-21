import type { DevupEngine } from '@devup-ui/plugin-utils'

/**
 * The environment variable `withDevupUI()` hands the transformer settings
 * through. Metro loads its config in the main process and forks the transform
 * workers with that process's environment, and its transformer API has no
 * other way to pass options to a Babel transformer.
 */
export const SETTINGS_ENV = 'DEVUP_UI_METRO_SETTINGS'

/** What the transformer needs, resolved once in the process that loads the config */
export interface MetroTransformerSettings {
  /** Absolute path of the Babel transformer the compiled code goes to */
  readonly upstreamTransformer: string
  /** The package whose imports are compiled */
  readonly package: string
  /** Packages in `node_modules` that are compiled too */
  readonly include: readonly string[]
  /** Whether the React Native runtime may be imported for what needs it */
  readonly runtime: boolean
  /** Whether the project has react-native-reanimated, which animations run on */
  readonly reanimated: boolean
  /** Absolute path of devup.json */
  readonly devupFile: string
  /** The engine; native by default */
  readonly engine?: DevupEngine
}

export function writeSettings(
  settings: MetroTransformerSettings,
  env: NodeJS.ProcessEnv = process.env,
): void {
  env[SETTINGS_ENV] = JSON.stringify(settings)
}

export class MissingSettingsError extends Error {
  static {
    // Metro's workers send an error by its constructor name, which the
    // minified build would lose
    Object.defineProperty(this, 'name', { value: 'MissingSettingsError' })
  }
  readonly name = 'MissingSettingsError'

  constructor() {
    super(
      '[devup-ui] The Metro transformer of @devup-ui/metro-plugin has no settings. Wrap the Metro config with `withDevupUI(config)` in metro.config.js instead of setting `babelTransformerPath` to it by hand.',
    )
  }
}

export function readSettings(
  env: NodeJS.ProcessEnv = process.env,
): MetroTransformerSettings {
  const raw = env[SETTINGS_ENV]
  if (!raw) throw new MissingSettingsError()
  const settings: MetroTransformerSettings = JSON.parse(raw)
  return settings
}
