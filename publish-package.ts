import { mkdir, mkdtemp, readdir, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { isAbsolute, join, resolve } from 'node:path'

const DEPENDENCY_FIELDS = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] as const

interface PackageManifest {
  name?: string
  version?: string
  private?: boolean
  dependencies?: Record<string, string>
  devDependencies?: Record<string, string>
  optionalDependencies?: Record<string, string>
  peerDependencies?: Record<string, string>
}

interface CommandResult {
  exitCode: number
  stdout: string
  stderr: string
}

async function runCaptured(
  command: string[],
  cwd: string,
): Promise<CommandResult> {
  const process = Bun.spawn(command, {
    cwd,
    env: globalThis.process.env,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ])
  return { exitCode, stdout, stderr }
}

async function runOrThrow(command: string[], cwd: string): Promise<string> {
  const result = await runCaptured(command, cwd)
  if (result.exitCode !== 0) {
    throw new Error(
      `${command.join(' ')} failed with exit code ${result.exitCode}\n${result.stderr}`,
    )
  }
  return result.stdout.trim()
}

async function readPackedManifest(tarball: string): Promise<PackageManifest> {
  const output = await runOrThrow(
    ['tar', '-xOf', tarball, 'package/package.json'],
    globalThis.process.cwd(),
  )
  return JSON.parse(output) as PackageManifest
}

async function verifyTarball(tarball: string): Promise<void> {
  const manifest = await readPackedManifest(tarball)
  for (const field of DEPENDENCY_FIELDS) {
    for (const [name, range] of Object.entries(manifest[field] ?? {})) {
      if (range.startsWith('workspace:')) {
        throw new Error(`Unresolved workspace dependency: ${field}.${name}`)
      }
    }
  }
}

async function packPackage(
  packageDir: string,
  destination: string,
): Promise<string> {
  await mkdir(destination, { recursive: true })
  const output = await runOrThrow(
    ['bun', 'pm', 'pack', '--quiet', '--destination', destination],
    packageDir,
  )
  const packedPath = output
    .split(/\r?\n/)
    .findLast((line) => line.trim().endsWith('.tgz'))
    ?.trim()
  if (!packedPath) throw new Error('bun pm pack did not return a tarball path')

  const tarball = isAbsolute(packedPath)
    ? packedPath
    : resolve(packageDir, packedPath)
  await verifyTarball(tarball)
  return tarball
}

async function getPublicPackageDirs(rootDir: string): Promise<string[]> {
  const packageDirs: string[] = []
  for (const parent of ['packages', 'bindings']) {
    const parentDir = join(rootDir, parent)
    const entries = await readdir(parentDir, { withFileTypes: true })
    for (const entry of entries) {
      if (!entry.isDirectory()) continue
      const packageDir = join(parentDir, entry.name)
      const manifest = JSON.parse(
        await readFile(join(packageDir, 'package.json'), 'utf8'),
      ) as PackageManifest
      if (manifest.private !== true) packageDirs.push(packageDir)
    }
  }
  return packageDirs.sort()
}

async function verifyWorkspace(rootDir: string): Promise<void> {
  const destination = await mkdtemp(join(tmpdir(), 'devup-ui-verify-'))
  try {
    for (const packageDir of await getPublicPackageDirs(rootDir)) {
      await packPackage(packageDir, destination)
    }
  } finally {
    await rm(destination, { recursive: true, force: true })
  }
}

async function publishTarball(tarball: string, dryRun: boolean): Promise<void> {
  const command = ['npm', 'publish', tarball]
  if (dryRun) command.push('--dry-run')
  const process = Bun.spawn(command, {
    cwd: globalThis.process.cwd(),
    env: globalThis.process.env,
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
  })
  const exitCode = await process.exited
  if (exitCode !== 0) {
    throw new Error(`npm publish failed with exit code ${exitCode}`)
  }
}

async function main(): Promise<void> {
  const [mode, value] = Bun.argv.slice(2)
  if (mode === '--verify-workspace') {
    await verifyWorkspace(import.meta.dir)
    return
  }

  if (mode === '--verify-tarball') {
    if (!value) throw new Error('--verify-tarball requires a tarball path')
    await verifyTarball(resolve(value))
    return
  }

  if (mode === '--pack-only') {
    if (!value) throw new Error('--pack-only requires a destination')
    await packPackage(globalThis.process.cwd(), resolve(value))
    return
  }

  if (mode && mode !== '--dry-run') {
    throw new Error(`Unknown argument: ${mode}`)
  }

  const destination = await mkdtemp(join(tmpdir(), 'devup-ui-publish-'))
  try {
    const tarball = await packPackage(globalThis.process.cwd(), destination)
    await publishTarball(tarball, mode === '--dry-run')
  } finally {
    await rm(destination, { recursive: true, force: true })
  }
}

try {
  await main()
} catch (error) {
  console.error(error instanceof Error ? error.message : error)
  globalThis.process.exitCode = 1
}
