import { join } from 'node:path'

import type { NextInvocationInput } from '../next-invocation-types'

export const installedRoot = join(import.meta.dir, 'fixture-next')
export const projectDir = join(import.meta.dir, 'fixture-project')
export const installedPath = (path: string): string => join(installedRoot, path)

export function invocation(): NextInvocationInput {
  return {
    required: true,
    stage: 'adapter',
    installed: { version: '16.3.6', packageDir: installedRoot },
    evaluation: {
      phase: 'phase-production-build',
      projectDir,
      expectedProjectDir: projectDir,
      configFile: join(projectDir, 'next.config.mjs'),
    },
    runtime: {
      entryPath: installedPath('dist/bin/next'),
      argv: ['node', installedPath('dist/bin/next'), 'build'],
      isMainThread: true,
      hasIpc: false,
      loadedModules: [installedPath('dist/cli/next-build.js')],
      env: { TURBOPACK: 'auto' },
    },
    build: { projectDir, mode: 'default' },
    config: { webpackBuildWorker: undefined, hasWebpack: true },
  }
}
