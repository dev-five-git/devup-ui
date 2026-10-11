import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import {
  createCompatTypes,
  createThemeInterfaceArgs,
  loadDevupConfig,
  ProductionNumbering,
  ProductionNumberingError,
  type ResolutionInputObserver,
  runBuildOperation,
} from '@devup-ui/plugin-utils'
import {
  getCss,
  getThemeInterface,
  importCanonicalMap,
  importFileRoutes,
  registerShorthands,
  registerTheme,
  seedFileMap,
  setAtomHoist,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import type { BuildEnvironment, ResolvedConfig } from 'vite'

import { type NativePresence, NonphysicalModules } from './nonphysical-modules'
import {
  productionEngine,
  type ProductionGeneration,
  ProductionLifecycle,
} from './production-engine'
import {
  prepareProductionManifest,
  type ProductionOptions,
} from './production-manifest'

export class ProductionActivation {
  private readonly lifecycle = new ProductionLifecycle()
  private configs = new WeakSet<ResolvedConfig>()
  private config: ResolvedConfig | undefined

  constructor(
    private readonly options: ProductionOptions,
    private readonly endLegacy: () => void,
  ) {}

  private get generation(): ProductionGeneration {
    return this.lifecycle.generation
  }

  get active(): boolean {
    return this.config?.command === 'build' && this.options.extractCss
  }

  get context(): string {
    if (this.generation.current === undefined)
      throw new ProductionNumberingError('unseeded')
    return this.generation.current
  }

  configure(config: ResolvedConfig): boolean {
    if (this.generation.pending === undefined) {
      this.config = config
      if (config?.command === 'build') this.configs.add(config)
      return false
    }
    // Native siblings resolve before preparation; watch closes bundles, not its generation.
    const fresh =
      !this.configs.has(config) &&
      this.generation.participants.size === 0 &&
      (!this.config?.build.watch || this.generation.owner.disposed)
    if (fresh) {
      this.lifecycle.replace()
      this.configs = new WeakSet()
    }
    this.configs.add(config)
    if (fresh) this.config = config
    return fresh
  }

  refine(environments: Readonly<Record<string, BuildEnvironment>>): void {
    if (this.generation.pending !== undefined || this.config === undefined)
      return
    this.config = {
      ...this.config,
      environments: Object.fromEntries(
        Object.entries(environments).map(([key, environment]) => [
          key,
          environment.config,
        ]),
      ),
    }
  }

  async prepare(
    observe: ResolutionInputObserver,
    input?: () => void,
  ): Promise<void> {
    if (input !== undefined && this.lifecycle.input()) input()
    const generation = this.generation
    generation.assertOpen()
    return (generation.pending ??= this.prepareOnce(generation, observe))
  }

  private async prepareOnce(
    generation: ProductionGeneration,
    observe: ResolutionInputObserver,
  ): Promise<void> {
    const config = this.config
    if (config === undefined) throw new ProductionNumberingError('invalid-plan')
    this.endLegacy()
    try {
      const manifest = await prepareProductionManifest(
        config,
        this.options,
        observe,
      )
      generation.assertOpen()
      generation.manifest = manifest
      for (const plan of manifest.contexts) {
        generation.contexts.set(plan.context, {
          plan,
          modules: new NonphysicalModules(plan.files),
          learned: plan.learned,
        })
      }
      const first = manifest.contexts[0]
      if (first === undefined)
        throw new ProductionNumberingError('invalid-plan')
      await this.writeData(first.context, generation)
      generation.assertOpen()
      generation.numbering = new ProductionNumbering(
        generation.owner,
        manifest.contexts,
      )
    } catch (error) {
      generation.fail()
      throw error
    }
  }

  private async writeData(
    key: string,
    generation = this.generation,
  ): Promise<void> {
    const manifest = generation.manifest
    if (manifest === undefined) throw new ProductionNumberingError('unseeded')
    const data = this.run(
      key,
      () => ({
        interfaceCode: getThemeInterface(
          ...createThemeInterfaceArgs(this.options.package),
        ),
        base: getCss(null, false),
      }),
      generation,
    )
    const { paths } = manifest
    await mkdir(paths.distDir, { recursive: true })
    await mkdir(paths.cssDir, { recursive: true })
    await writeFile(join(paths.distDir, '.gitignore'), '*', 'utf8')
    await writeFile(
      join(paths.distDir, 'compat.d.ts'),
      createCompatTypes(this.options.importAliases),
      'utf8',
    )
    await writeFile(
      join(paths.distDir, 'theme.d.ts'),
      data.interfaceCode,
      'utf8',
    )
    if (!this.options.singleCss)
      await writeFile(join(paths.cssDir, 'devup-ui.css'), data.base, 'utf8')
  }

  async updateTheme(): Promise<void> {
    const generation = this.generation
    const manifest = generation.manifest
    if (manifest === undefined) throw new ProductionNumberingError('unseeded')
    const theme = (await loadDevupConfig(manifest.paths.devupFile)).theme ?? {}
    generation.assertOpen()
    generation.manifest = { ...manifest, theme }
    await this.writeData(this.context, generation)
  }

  run<T>(key: string, action: () => T, generation = this.generation): T {
    const manifest = generation.manifest
    if (manifest === undefined)
      throw new ProductionNumberingError('unseeded', { context: key })
    const runtime = generation.runtime(key)
    try {
      return runBuildOperation(
        { integration: 'Vite', root: this.config?.root ?? process.cwd() },
        () =>
          generation.owner.run(productionEngine, () => {
            registerTheme(manifest.theme)
            registerShorthands(this.options.shorthands ?? {})
            setDebug(this.options.debug)
            setPrefix(this.options.prefix ?? null)
            importCanonicalMap(runtime.plan.canonical)
            importFileRoutes(runtime.plan.routes)
            setAtomHoist(runtime.plan.threshold ?? null)
            setModuleResolver(undefined)
            generation.numbering?.seed({ seedFileMap, importCanonicalMap }, key)
            generation.current = key
            return action()
          }),
      )
    } catch (error) {
      generation.fail()
      throw error
    }
  }

  modules(key: string): NonphysicalModules {
    return this.generation.runtime(key).modules
  }

  start(key: string): void {
    const runtime = this.generation.runtime(key)
    this.generation.close(key)
    this.generation.participants.set(key, this.generation.owner.acquire(false))
    runtime.modules.start()
  }

  async finish(
    key: string,
    graph: NativePresence,
    error?: Error,
  ): Promise<void> {
    await this.generation.finish(key, graph, error)
  }

  css(file: number | null | undefined, main: boolean): string {
    return this.run(this.context, () => getCss(file, main))
  }

  invalidate(id: string): void {
    for (const runtime of this.generation.contexts.values())
      runtime.modules.invalidate(id)
  }

  close(key = this.generation.current): void {
    this.generation.close(key)
  }

  dispose(): void {
    this.generation.dispose()
  }
}
