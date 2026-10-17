import { randomUUID } from 'node:crypto'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import type { NextConfig } from 'next'

import type { FinalConfigFinalizer } from '../final-config-adapter'
import { installFinalConfigAdapter } from '../final-config-adapter'
import { createFinalConfigAdapter } from '../final-config-adapter-entry'

export function adapterFixture() {
  const projectDir = mkdtempSync(join(tmpdir(), 'devup-adapter-'))
  const token = randomUUID()
  writeFileSync(join(projectDir, 'shipped-entry.cjs'), 'module.exports = {};\n')
  const session = {
    token,
    projectDir,
    sessionDir: projectDir,
    configFile: join(projectDir, 'next.config.mjs'),
  }
  const context = {
    phase: 'phase-production-build',
    nextVersion: 'fixture',
    projectDir,
  }
  const releases: (() => void)[] = []
  return {
    session,
    context,
    caller: (source: string, extension = 'cjs') => {
      const path = join(projectDir, `caller-${randomUUID()}.${extension}`)
      writeFileSync(path, source)
      return path
    },
    install: (
      config: NextConfig,
      finalize: FinalConfigFinalizer,
      timeoutMs = 45000,
    ) => {
      const installed = installFinalConfigAdapter(config, session, {
        entryPath: join(projectDir, 'shipped-entry.cjs'),
        finalize,
        timeoutMs,
      })
      releases.push(installed.release)
      return { ...installed, adapter: createFinalConfigAdapter(token) }
    },
    close: () => {
      for (const release of releases) release()
      rmSync(projectDir, { recursive: true, force: true })
    },
  }
}
