import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  existsSync,
  lstatSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'

export const sha = (data) => createHash('sha256').update(data).digest('hex')
export const json = (path, value) =>
  writeFileSync(path, JSON.stringify(value, null, 2))

export function inventory(path, root) {
  if (!existsSync(path)) return []
  const stat = lstatSync(path)
  if (stat.isSymbolicLink())
    throw new Error(`Refusing linked artifact: ${path}`)
  if (stat.isDirectory())
    return readdirSync(path)
      .sort()
      .flatMap((name) => inventory(join(path, name), root))
  if (!stat.isFile()) throw new Error(`Refusing special artifact: ${path}`)
  const bytes = readFileSync(path)
  return [
    {
      filename: relative(root, path).replaceAll('\\', '/'),
      bytes: bytes.length,
      sha256: sha(bytes),
    },
  ]
}

export function captureGenerated(cwd, app, generated) {
  for (const file of app === 'landing'
    ? ['public/search.json', 'src/utils/demos.generated.ts']
    : []) {
    const path = join(cwd, file)
    if (
      realpathSync(dirname(path)).toLowerCase() !== dirname(path).toLowerCase()
    )
      throw new Error(`Generated parent escapes owned app: ${path}`)
    const stat = lstatSync(path, { throwIfNoEntry: false })
    if ((stat && !stat.isFile()) || (!stat && file !== 'public/search.json'))
      throw new Error(`Unexpected generated baseline: ${path}`)
    generated.push(
      stat
        ? { path, exists: true, bytes: readFileSync(path) }
        : { path, exists: false },
    )
  }
}

export function prepareCaches({ root, cwd, env, git }, baseline) {
  // Validate every candidate before deleting any; descendants must be ignored and untracked.
  for (const dir of ['df', 'dist', '.next', 'node_modules/.vite']) {
    const path = join(cwd, dir)
    if (!existsSync(path)) continue
    for (const parent of dir === 'node_modules/.vite'
      ? [join(cwd, 'node_modules'), path]
      : [path]) {
      if (
        lstatSync(parent).isSymbolicLink() ||
        realpathSync(parent).toLowerCase() !== resolve(parent).toLowerCase()
      )
        throw new Error(`Cache escapes owned app: ${parent}`)
    }
    if (git(['ls-files', '--', relative(root, path)]).trim())
      throw new Error(`Tracked cache baseline: ${path}`)
    const files = inventory(path, root)
    if (
      dir === 'df' &&
      (!existsSync(join(path, '.gitignore')) ||
        readFileSync(join(path, '.gitignore'), 'utf8').trim() !== '*')
    )
      throw new Error(`Unrecognized Devup generated cache: ${path}`)
    const markers = {
      dist: ['index.html', 'client', 'server', '.vite'],
      '.next': [
        'BUILD_ID',
        'cache',
        'server',
        'static',
        'types',
        'package.json',
      ],
      'node_modules/.vite': ['deps', 'deps_ssr', '_metadata.json'],
    }
    if (
      dir !== 'df' &&
      files.length &&
      !markers[dir].some((marker) => existsSync(join(path, marker)))
    )
      throw new Error(`Unrecognized generated cache baseline: ${path}`)
    if (dir !== 'df') git(['check-ignore', '--', relative(root, path) + '/'])
    baseline.push({ directory: relative(root, path), files })
  }
  const candidates = new Set(
    baseline.flatMap((cache) => cache.files.map((file) => file.filename)),
  )
  if (candidates.size) {
    const check = spawnSync(
      'git.exe',
      ['-C', root, 'check-ignore', '--stdin', '-z'],
      {
        env,
        input: [...candidates].join('\0') + '\0',
        encoding: 'utf8',
        windowsHide: true,
        maxBuffer: 64 * 1024 ** 2,
      },
    )
    if (check.error) throw check.error
    if (![0, 1].includes(check.status))
      throw new Error(`Batched ignore check failed: ${check.stderr}`)
    const ignored = new Set(check.stdout.split('\0').filter(Boolean))
    if (
      ignored.size !== candidates.size ||
      [...candidates].some((filename) => !ignored.has(filename))
    )
      throw new Error(
        'Cache inventory includes nonignored files; refusing deletion',
      )
  }
}

export function restoreFiles(
  { installed, alias, original, generated },
  result,
) {
  if (installed) writeFileSync(alias, original)
  for (const file of installed ? generated : []) {
    if (
      realpathSync(dirname(file.path)).toLowerCase() !==
      dirname(file.path).toLowerCase()
    )
      throw new Error(
        `Generated restore parent escapes owned app: ${file.path}`,
      )
    const stat = lstatSync(file.path, { throwIfNoEntry: false })
    if (stat && !stat.isFile())
      throw new Error(
        `Refusing to restore over a special generated path: ${file.path}`,
      )
    if (file.exists) writeFileSync(file.path, file.bytes)
    else if (stat) rmSync(file.path)
  }
  result.aliasRestoredByteIdentically = installed
    ? sha(readFileSync(alias)) === sha(original)
    : null
  if (installed && !result.aliasRestoredByteIdentically)
    throw new Error('Alias restoration mismatch')
}
