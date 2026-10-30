import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  statSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import {
  basename,
  dirname,
  isAbsolute,
  join,
  relative,
  resolve,
  sep,
} from 'node:path'
import { fileURLToPath } from 'node:url'

import { selectHostTarget } from '../binding/package/build-host.mjs'
import { targets } from '../binding/package/matrix.mjs'

export class HostPackError extends Error {
  constructor(category, packageName, cause) {
    super(
      `${packageName}: ${category}${typeof cause === 'string' ? `: ${cause}` : ''}`,
      { cause },
    )
    this.name = 'HostPackError'
    this.category = category
    this.packageName = packageName
  }
}

export function hostPackSelection() {
  const row = selectHostTarget()
  if (!row)
    throw new HostPackError(
      'missing-host',
      '@devup-ui/binding',
      'unsupported host',
    )
  return {
    name: `@devup-ui/binding-${row.suffix}`,
    omitted: targets
      .filter((item) => item !== row)
      .map((item) => `@devup-ui/binding-${item.suffix}`),
    installArgs: [
      `--os=${row.os}`,
      `--cpu=${row.cpu}`,
      ...(row.libc ? [`--libc=${row.libc}`] : []),
    ],
  }
}

export async function packHostEngines(context, run) {
  const host = hostPackSelection()
  const tarballs = {}
  for (const name of ['@devup-ui/binding', host.name]) {
    const short = name.slice('@devup-ui/'.length)
    const source = join(context.repository, 'bindings', `devup-ui-${short}`)
    const manifest = JSON.parse(
      readFileSync(join(source, 'package.json'), 'utf8'),
    )
    const entry = statSync(join(source, manifest.main))
    if (!entry.isFile() || entry.size === 0) {
      throw new HostPackError(
        'byte-mismatch',
        name,
        'missing existing checkout build entry',
      )
    }
    const destination = join(context.root, 'packs', short)
    mkdirSync(destination, { recursive: true })
    await run(
      context.bun,
      [
        'pm',
        'pack',
        '--ignore-scripts',
        '--quiet',
        '--destination',
        destination,
      ],
      {
        cwd: source,
      },
    )
    const files = readdirSync(destination).filter((file) =>
      file.endsWith('.tgz'),
    )
    if (files.length !== 1)
      throw new HostPackError(
        'byte-mismatch',
        name,
        'expected one host tarball',
      )
    tarballs[name] =
      `file:${realpathSync(join(destination, files[0])).split(sep).join('/')}`
  }
  return { host, tarballs }
}

function archivePath(reference, base) {
  if (typeof reference !== 'string' || !reference.startsWith('file:')) {
    throw new TypeError('Expected a local file tarball reference')
  }
  return realpathSync(
    reference.startsWith('file://')
      ? fileURLToPath(reference)
      : resolve(base, reference.slice('file:'.length)),
  )
}

function payload(directory, name, integrity) {
  const files = new Map()
  const pending = [{ directory, prefix: '' }]
  while (pending.length) {
    const current = pending.pop()
    for (const entry of readdirSync(current.directory, {
      withFileTypes: true,
    })) {
      if (entry.name === 'node_modules') continue
      const path = join(current.directory, entry.name)
      const key = current.prefix + entry.name
      if (entry.name === '.bun-tag') {
        if (key !== '.bun-tag') {
          throw new HostPackError('marker-nested', name, key)
        }
        if (!entry.isFile()) {
          throw new HostPackError('marker-kind', name, key)
        }
        if (!readFileSync(path).equals(Buffer.from(integrity))) {
          throw new HostPackError('marker-bytes', name, key)
        }
      }
      if (entry.isDirectory())
        pending.push({ directory: path, prefix: `${key}/` })
      else if (entry.isFile())
        files.set(
          key,
          createHash('sha256').update(readFileSync(path)).digest('hex'),
        )
      else
        throw new HostPackError(
          'byte-mismatch',
          name,
          `nonregular payload ${key}`,
        )
    }
  }
  return files
}

function packedPayload(name, reference) {
  const scratch = mkdtempSync(join(tmpdir(), 'devup-host-pack-'))
  try {
    const file = archivePath(reference, process.cwd())
    const options = { cwd: dirname(file), encoding: 'utf8', timeout: 30000 }
    const members = execFileSync('tar', ['-tf', basename(file)], options)
      .split(/\r?\n/)
      .filter(Boolean)
    const seen = new Set()
    for (const member of members) {
      const parts = member.replace(/\/$/, '').split('/')
      if (parts.includes('.bun-tag')) {
        throw new HostPackError('archive-marker', name, member)
      }
      if (
        parts[0] !== 'package' ||
        member.includes('\\') ||
        parts.some(
          (part) =>
            !part ||
            part === '.' ||
            part === '..' ||
            part.includes(':') ||
            part === 'node_modules',
        ) ||
        seen.has(member)
      ) {
        throw new HostPackError(
          'byte-mismatch',
          name,
          `unsafe archive member ${member}`,
        )
      }
      seen.add(member)
    }
    const listing = execFileSync('tar', ['-tvf', basename(file)], options)
      .split(/\r?\n/)
      .filter(Boolean)
    if (listing.some((line) => line[0] !== '-' && line[0] !== 'd')) {
      throw new HostPackError(
        'byte-mismatch',
        name,
        'archive links or special files',
      )
    }
    execFileSync('tar', ['-xf', basename(file), '-C', scratch], options)
    const directory = join(scratch, 'package')
    if (
      JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8')).name !==
      name
    ) {
      throw new HostPackError('byte-mismatch', name, 'archive package name')
    }
    const compressed = readFileSync(file)
    return {
      files: payload(directory, name),
      sha256: createHash('sha256').update(compressed).digest('hex'),
      integrity: `sha512-${createHash('sha512').update(compressed).digest('base64')}`,
    }
  } catch (cause) {
    if (cause instanceof HostPackError) throw cause
    throw new HostPackError('byte-mismatch', name, cause)
  } finally {
    rmSync(scratch, { recursive: true, force: true })
  }
}

function installedNodes(directory) {
  const pending = [join(directory, 'node_modules')]
  const visited = new Set()
  const packages = new Map()
  while (pending.length) {
    const path = pending.pop()
    const physical = realpathSync(path)
    if (!statSync(physical).isDirectory()) continue
    const parent = dirname(path)
    const slot =
      basename(parent) === 'node_modules'
        ? basename(path)
        : basename(dirname(parent)) === 'node_modules' &&
            basename(parent).startsWith('@')
          ? `${basename(parent)}/${basename(path)}`
          : undefined
    const manifestFile = join(physical, 'package.json')
    if (slot && existsSync(manifestFile)) {
      const manifest = JSON.parse(readFileSync(manifestFile, 'utf8'))
      if (slot.startsWith('@devup-ui/') && manifest.name !== slot) {
        throw new HostPackError(
          'unexpected-package',
          slot,
          'installed manifest name',
        )
      }
      if (
        typeof manifest.name === 'string' &&
        manifest.name.startsWith('@devup-ui/')
      ) {
        packages.set(physical, { name: manifest.name, path: physical })
      }
    } else if (slot?.startsWith('@devup-ui/')) {
      throw new HostPackError(
        'unexpected-package',
        slot,
        'missing installed manifest',
      )
    }
    if (visited.has(physical)) continue
    visited.add(physical)
    for (const entry of readdirSync(physical, { withFileTypes: true })) {
      // Metadata cannot contain packages; payload() still inspects its kind/bytes.
      if (entry.name === '.bun-tag') continue
      if (entry.isDirectory() || entry.isSymbolicLink())
        pending.push(join(physical, entry.name))
    }
  }
  return [...packages.values()]
}

function verifySource(name, reference, context) {
  const records = Object.entries(context.lock.packages).filter(
    ([key, record]) =>
      key === name ||
      key.endsWith(`/${name}`) ||
      (Array.isArray(record) &&
        typeof record[0] === 'string' &&
        record[0].startsWith(`${name}@`)),
  )
  try {
    if (records.length === 0)
      throw new TypeError('missing Bun package source record')
    const expected = archivePath(reference, context.directory)
    for (const [, record] of records) {
      if (
        !Array.isArray(record) ||
        typeof record[0] !== 'string' ||
        !record[0].startsWith(`${name}@`)
      ) {
        throw new TypeError('non-file Bun package source')
      }
      const source = record[0].slice(name.length + 1)
      if (!isAbsolute(source) || realpathSync(source) !== expected) {
        throw new TypeError(
          "Bun package source is not this run's canonical tarball",
        )
      }
    }
  } catch (cause) {
    throw new HostPackError('source-mismatch', name, cause)
  }
  return records.map(([, record]) => record)
}

export function verifyHostInstall(options) {
  const host = hostPackSelection()
  const directory = realpathSync(options.directory)
  const tarballs = new Map(
    Object.entries(options.tarballs).map(([name, reference]) => [
      name.startsWith('@devup-ui/') ? name : `@devup-ui/${name}`,
      reference,
    ]),
  )
  const installed = installedNodes(directory)
  if (!installed.some((item) => item.name === host.name)) {
    throw new HostPackError(
      'missing-host',
      host.name,
      'host sibling not installed',
    )
  }
  if (!installed.some((item) => item.name === '@devup-ui/binding')) {
    throw new HostPackError(
      'source-mismatch',
      '@devup-ui/binding',
      'binding root not installed',
    )
  }
  for (const binding of installed.filter(
    (item) => item.name === '@devup-ui/binding',
  )) {
    try {
      const require = createRequire(join(binding.path, 'package.json'))
      const sibling = realpathSync(
        dirname(require.resolve(`${host.name}/package.json`)),
      )
      if (
        !installed.some(
          (item) => item.name === host.name && item.path === sibling,
        )
      ) {
        throw new TypeError(
          'host sibling resolves outside the installed inventory',
        )
      }
    } catch (cause) {
      throw new HostPackError('missing-host', host.name, cause)
    }
  }
  let lock
  try {
    lock = JSON.parse(
      execFileSync(
        options.bun ?? 'bun',
        [
          '--eval',
          "process.stdout.write(JSON.stringify(Bun.JSONC.parse(require('node:fs').readFileSync(process.env.DEVUP_ENGINE_LOCKFILE, 'utf8'))))",
        ],
        {
          cwd: directory,
          encoding: 'utf8',
          timeout: 30000,
          env: {
            ...process.env,
            DEVUP_ENGINE_LOCKFILE: join(directory, 'bun.lock'),
          },
        },
      ),
    )
    if (
      !lock ||
      typeof lock.packages !== 'object' ||
      lock.packages === null ||
      Array.isArray(lock.packages)
    ) {
      throw new TypeError('missing Bun packages table')
    }
  } catch (cause) {
    throw new HostPackError('source-mismatch', '@devup-ui/binding', cause)
  }
  const cache = new Map()
  for (const item of installed) {
    if (!tarballs.has(item.name) || host.omitted.includes(item.name)) {
      throw new HostPackError(
        'unexpected-package',
        item.name,
        'not in this host pack inventory',
      )
    }
    const location = relative(directory, item.path)
    if (
      location === '..' ||
      location.startsWith(`..${sep}`) ||
      isAbsolute(location)
    ) {
      throw new HostPackError(
        'source-mismatch',
        item.name,
        'installed link escapes consumer',
      )
    }
    const reference = tarballs.get(item.name)
    const records = verifySource(item.name, reference, { directory, lock })
    if (!cache.has(item.name))
      cache.set(item.name, packedPayload(item.name, reference))
    const packed = cache.get(item.name)
    for (const record of records) {
      if (typeof record[2] !== 'string' || record[2].length === 0) {
        throw new HostPackError('lock-integrity-missing', item.name)
      }
      if (record[2] !== packed.integrity) {
        throw new HostPackError('lock-integrity-mismatch', item.name)
      }
    }
    const expected = new Map(packed.files)
    const actual = payload(item.path, item.name, packed.integrity)
    if (actual.has('.bun-tag')) {
      expected.set(
        '.bun-tag',
        createHash('sha256')
          .update(Buffer.from(packed.integrity))
          .digest('hex'),
      )
    }
    if (
      actual.size !== expected.size ||
      [...actual].some(([file, hash]) => expected.get(file) !== hash)
    ) {
      throw new HostPackError(
        'byte-mismatch',
        item.name,
        'installed payload differs from its tarball',
      )
    }
  }
  const receipt = {
    event: 'devup.host-pack-proof',
    mode: 'host-only',
    host: host.name,
    packages: installed,
    omitted: host.omitted,
    tarballs: [...cache].map(([name, packed]) => ({
      name,
      sha256: packed.sha256,
    })),
    reason: `Explicit Bun ${host.installArgs.join(' ')}; non-host Devup siblings verified absent`,
  }
  console.info(JSON.stringify(receipt))
  return receipt
}
